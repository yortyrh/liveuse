use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use futures::StreamExt;
use rig::agent::{
    AgentHook, DispatchAction, DispatchEvent, HookContext, OutcomeAction, OutcomeEvent,
};
use rig::message::{AssistantContent, ImageMediaType, ToolName, ToolResultContent};
use rig::prelude::*;
use rig::streaming::{Item, PartKind, StreamEvent};
use rig::tool::{DynamicTool, ToolExecutionError, ToolOutput};
use serde::Deserialize;
use serde_json::{Value, json};
use tauri::Emitter;

use crate::PerformTarget;
use crate::actions::{AppState, execute, show_main};
use crate::mcp;
use crate::remote_perform;

const PREAMBLE: &str = "\
You are the LiveUse agent. You control the computer with the tools, like a human operator.
The screen origin is the top left and Y grows downward. Screenshot, position, and click coordinates share the same grid.
Look at the screen before acting. After each important action, look again. When you are done, reply in the user's language and do not call more tools.
If a tool returns aborted, stop immediately.";

#[derive(Deserialize)]
pub struct Turn {
    pub role: String,
    pub text: String,
}

pub async fn run(
    state: &Arc<AppState>,
    target: PerformTarget,
    history: Vec<Turn>,
    prompt: String,
) -> Result<(), String> {
    let _guard = RunningGuard::enter(state);
    let key = state
        .llm_api_key
        .read()
        .map_err(|err| err.to_string())?
        .clone();
    let local = matches!(target, PerformTarget::Local);
    if local {
        let summary = prompt.trim().chars().take(80).collect::<String>();
        let label = if summary.is_empty() {
            "Working".to_string()
        } else {
            summary
        };
        state.begin_control(&label);
    }
    let result = run_agent(state, target, history, prompt, key).await;
    if local {
        state.end_control();
        show_main(&state.app);
    }
    result
}

async fn run_agent(
    state: &Arc<AppState>,
    target: PerformTarget,
    history: Vec<Turn>,
    prompt: String,
    key: String,
) -> Result<(), String> {
    let base = state
        .llm_base_url
        .read()
        .map_err(|err| err.to_string())?
        .clone();
    let model_name = state
        .llm_model
        .read()
        .map_err(|err| err.to_string())?
        .clone();
    let provider = state
        .llm_provider
        .read()
        .map_err(|err| err.to_string())?
        .clone();
    let model = crate::llm::completion_model(&provider, &model_name, &base, &key)?;
    let tools = computer_tools(Arc::clone(state), target);
    let agent = AgentBuilder::new(model)
        .preamble(PREAMBLE)
        .dynamic_tools(tools)
        .default_max_turns(24)
        .build();
    let history = history_messages(&history);
    let mut stream = agent
        .prompt(prompt)
        .history(history)
        .max_turns(24)
        .add_hook(AbortHook {
            state: Arc::clone(state),
        })
        .stream();
    let mut segment: Option<TextSegment> = None;
    loop {
        let item = tokio::select! {
            biased;
            item = stream.next() => item,
            () = crate::wait_abort(&state.abort) => {
                if let Some(current) = segment.take() {
                    current.finish(state);
                }
                return finish_error(state, "aborted");
            }
        };
        let Some(item) = item else {
            break;
        };
        if state.abort.load(Ordering::SeqCst) {
            if let Some(current) = segment.take() {
                current.finish(state);
            }
            return finish_error(state, "aborted");
        }
        match item {
            Ok(MultiTurnStreamItem::StreamAssistantItem(Item::Event(event))) => match event {
                StreamEvent::Start {
                    kind: PartKind::Text, ..
                } => {
                    if let Some(current) = segment.as_mut() {
                        current.saw_delta = false;
                    }
                }
                StreamEvent::Text { text, .. } => {
                    open_segment(&mut segment).push_delta(state, &text);
                }
                StreamEvent::End {
                    content: AssistantContent::Text(text),
                    ..
                } => {
                    open_segment(&mut segment).push_settled(state, &text.text);
                }
                _ => {}
            },
            Ok(MultiTurnStreamItem::ToolCall { .. }) => {
                if let Some(current) = segment.take() {
                    current.finish(state);
                }
            }
            Ok(MultiTurnStreamItem::ModelTurnRetried { .. }) => {
                if let Some(current) = segment.take() {
                    current.reset(state);
                }
            }
            Ok(MultiTurnStreamItem::FinalResponse(response)) => {
                settle_final(state, segment.take(), &response.output());
                return Ok(());
            }
            Ok(_) => {}
            Err(err) => {
                if let Some(current) = segment.take() {
                    current.finish(state);
                }
                return finish_error(state, &err.to_string());
            }
        }
    }
    if let Some(current) = segment.take() {
        current.finish(state);
    }
    Ok(())
}

struct TextSegment {
    id: String,
    text: String,
    saw_delta: bool,
}

fn open_segment(segment: &mut Option<TextSegment>) -> &mut TextSegment {
    segment.get_or_insert_with(|| TextSegment {
        id: new_id(),
        text: String::new(),
        saw_delta: false,
    })
}

impl TextSegment {
    fn push_delta(&mut self, state: &AppState, fragment: &str) {
        self.saw_delta = true;
        self.text.push_str(fragment);
        emit_delta(state, &self.id, &self.text, false, false);
    }

    fn push_settled(&mut self, state: &AppState, text: &str) {
        if self.saw_delta || text.is_empty() {
            return;
        }
        self.text.push_str(text);
        emit_delta(state, &self.id, &self.text, false, false);
    }

    fn finish(mut self, state: &AppState) {
        let text = self.text.trim();
        if text.is_empty() {
            self.reset(state);
            return;
        }
        self.text = text.to_string();
        emit_delta(state, &self.id, &self.text, true, false);
    }

    fn reset(self, state: &AppState) {
        emit_delta(state, &self.id, "", true, true);
    }
}

fn settle_final(state: &AppState, segment: Option<TextSegment>, final_text: &str) {
    let final_text = final_text.trim();
    let Some(mut segment) = segment else {
        emit_line(
            state,
            "assistant",
            if final_text.is_empty() {
                "Done."
            } else {
                final_text
            },
            None,
        );
        return;
    };
    if segment.text.trim().is_empty() {
        if final_text.is_empty() {
            segment.reset(state);
            emit_line(state, "assistant", "Done.", None);
            return;
        }
        segment.text = final_text.to_string();
        segment.saw_delta = true;
        segment.finish(state);
        return;
    }
    if final_text.len() > segment.text.trim().len() && final_text.starts_with(segment.text.trim()) {
        segment.text = final_text.to_string();
        segment.saw_delta = true;
    }
    segment.finish(state);
}

fn finish_error(state: &AppState, err: &str) -> Result<(), String> {
    let stopped = state.abort.load(Ordering::SeqCst)
        || err.to_lowercase().contains("abort")
        || err.to_lowercase().contains("cancel");
    if stopped {
        emit_line(state, "assistant", "Operation stopped.", None);
        Ok(())
    } else {
        Err(err.to_string())
    }
}

struct RunningGuard<'a> {
    state: &'a AppState,
}

impl<'a> RunningGuard<'a> {
    fn enter(state: &'a AppState) -> Self {
        state.abort.store(false, Ordering::SeqCst);
        state.agent_running.store(true, Ordering::SeqCst);
        Self { state }
    }
}

impl Drop for RunningGuard<'_> {
    fn drop(&mut self) {
        self.state.agent_running.store(false, Ordering::SeqCst);
        self.state.abort.store(false, Ordering::SeqCst);
    }
}

struct AbortHook {
    state: Arc<AppState>,
}

impl AgentHook for AbortHook {
    async fn on_dispatch(&self, _ctx: &HookContext, _event: DispatchEvent<'_>) -> DispatchAction {
        if self.state.abort.load(Ordering::SeqCst) {
            DispatchAction::stop("aborted")
        } else {
            DispatchAction::proceed()
        }
    }

    async fn on_outcome(&self, _ctx: &HookContext, _event: OutcomeEvent<'_>) -> OutcomeAction {
        if self.state.abort.load(Ordering::SeqCst) {
            OutcomeAction::stop("aborted")
        } else {
            OutcomeAction::proceed()
        }
    }
}

fn history_messages(turns: &[Turn]) -> Vec<Message> {
    turns
        .iter()
        .filter_map(|turn| match turn.role.as_str() {
            "user" => Some(Message::user(turn.text.clone())),
            "assistant" => Some(Message::assistant(turn.text.clone())),
            _ => None,
        })
        .collect()
}

fn computer_tools(state: Arc<AppState>, target: PerformTarget) -> Vec<DynamicTool> {
    let specs = [
        (
            "screenshot",
            "Capture the screen. width and height are the click grid, origin at the top left.",
            schema(&[("operation", "string", "status text")], &[]),
        ),
        ("position", "Current cursor position.", schema(&[], &[])),
        ("size", "Screen size.", schema(&[], &[])),
        (
            "move_to",
            mcp::MOVE_TO,
            schema(
                &[
                    ("x", "integer", "x"),
                    ("y", "integer", "y"),
                    ("duration", "number", "seconds"),
                    (
                        "tween",
                        "string",
                        "linear, ease_in_quad, ease_out_quad, ease_in_out_quad, ease_in_bounce, ease_in_elastic",
                    ),
                    ("operation", "string", "status"),
                ],
                &[],
            ),
        ),
        (
            "move",
            mcp::MOVE,
            schema(
                &[
                    ("dx", "integer", "pixels"),
                    ("dy", "integer", "pixels"),
                    ("duration", "number", "seconds"),
                    ("tween", "string", "tween"),
                    ("operation", "string", "status"),
                ],
                &[],
            ),
        ),
        (
            "click",
            mcp::CLICK,
            schema(
                &[
                    ("x", "integer", "x"),
                    ("y", "integer", "y"),
                    ("clicks", "integer", "repeats"),
                    ("interval", "number", "seconds"),
                    ("button", "string", "left, right, or middle"),
                    ("operation", "string", "status"),
                ],
                &[],
            ),
        ),
        (
            "double_click",
            "Double-click.",
            schema(
                &[
                    ("x", "integer", "x"),
                    ("y", "integer", "y"),
                    ("button", "string", "button"),
                    ("operation", "string", "status"),
                ],
                &[],
            ),
        ),
        (
            "right_click",
            "Right-click.",
            schema(
                &[
                    ("x", "integer", "x"),
                    ("y", "integer", "y"),
                    ("operation", "string", "status"),
                ],
                &[],
            ),
        ),
        (
            "drag_to",
            "Drag to a coordinate.",
            schema(
                &[
                    ("x", "integer", "x"),
                    ("y", "integer", "y"),
                    ("button", "string", "left, right, or middle"),
                    ("duration", "number", "seconds"),
                    ("operation", "string", "status"),
                ],
                &[],
            ),
        ),
        (
            "scroll",
            "Vertical wheel. Positive scrolls up.",
            schema(
                &[
                    ("clicks", "integer", "notches"),
                    ("x", "integer", "x"),
                    ("y", "integer", "y"),
                    ("operation", "string", "status"),
                ],
                &["clicks"],
            ),
        ),
        (
            "hscroll",
            "Horizontal wheel. Positive scrolls right.",
            schema(
                &[
                    ("clicks", "integer", "notches"),
                    ("operation", "string", "status"),
                ],
                &["clicks"],
            ),
        ),
        (
            "write",
            mcp::WRITE,
            schema(
                &[
                    ("text", "string", "text"),
                    ("interval", "number", "seconds between keys"),
                    ("operation", "string", "status"),
                ],
                &["text"],
            ),
        ),
        (
            "press",
            mcp::PRESS,
            schema(
                &[
                    ("key", "string", "one key name or character"),
                    ("keys", "array", "keys tapped one after another"),
                    ("presses", "integer", "repeats"),
                    ("operation", "string", "status"),
                ],
                &[],
            ),
        ),
        (
            "hotkey",
            mcp::HOTKEY,
            schema(
                &[("keys", "array", "keys"), ("operation", "string", "status")],
                &["keys"],
            ),
        ),
        (
            "key_down",
            "Hold a key.",
            schema(
                &[("key", "string", "key"), ("operation", "string", "status")],
                &["key"],
            ),
        ),
        (
            "key_up",
            "Release a key.",
            schema(
                &[("key", "string", "key"), ("operation", "string", "status")],
                &["key"],
            ),
        ),
    ];
    specs
        .into_iter()
        .map(|(name, description, parameters)| {
            tool(
                name,
                description,
                parameters,
                Arc::clone(&state),
                target.clone(),
            )
        })
        .collect()
}

fn tool(
    name: &'static str,
    description: &'static str,
    parameters: Value,
    state: Arc<AppState>,
    target: PerformTarget,
) -> DynamicTool {
    let tool_name = ToolName::new(name).expect("static tool name");
    DynamicTool::new(tool_name, description, parameters, move |args| {
        let state = Arc::clone(&state);
        let target = target.clone();
        Box::pin(async move { call_tool(&state, &target, name, args).await })
            as Pin<Box<dyn Future<Output = Result<ToolOutput, ToolExecutionError>> + Send>>
    })
}

async fn call_tool(
    state: &AppState,
    target: &PerformTarget,
    name: &str,
    args: Value,
) -> Result<ToolOutput, ToolExecutionError> {
    if state.abort.load(Ordering::SeqCst) {
        return Err(ToolExecutionError::cancelled("aborted"));
    }
    let value = match target {
        PerformTarget::Local => execute(state, name, &args)
            .await
            .map_err(|err| tool_error(&err.message()))?,
        PerformTarget::Remote { url, token } => {
            remote_perform(url, token, name, &args, &state.abort)
        }
            .await
            .map_err(|err| tool_error(&err))?,
    };
    if state.abort.load(Ordering::SeqCst) {
        return Err(ToolExecutionError::cancelled("aborted"));
    }
    if name == "screenshot" {
        return screenshot_output(state, value);
    }
    let text = describe(name, &args);
    let shown = compact_result(&value);
    emit_line(state, "tool", &format!("{text} → {shown}"), None);
    Ok(ToolOutput::json(strip_images(value)))
}

fn screenshot_output(state: &AppState, value: Value) -> Result<ToolOutput, ToolExecutionError> {
    let width = value.get("width").and_then(Value::as_u64).unwrap_or(0);
    let height = value.get("height").and_then(Value::as_u64).unwrap_or(0);
    let jpeg = value
        .get("jpeg_base64")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if jpeg.is_empty() {
        return Err(ToolExecutionError::other("screenshot had no image"));
    }
    let caption =
        format!("Screen {width}×{height}. Coordinates use this grid, origin at the top left.");
    emit_line(
        state,
        "tool",
        &caption,
        Some(format!("data:image/jpeg;base64,{jpeg}")),
    );
    ToolOutput::content(vec![
        ToolResultContent::text(caption),
        ToolResultContent::image_base64(jpeg, Some(ImageMediaType::JPEG), None),
    ])
}

fn tool_error(message: &str) -> ToolExecutionError {
    if message == "aborted" {
        ToolExecutionError::cancelled("aborted")
    } else {
        ToolExecutionError::other(message.to_string())
    }
}

fn describe(name: &str, args: &Value) -> String {
    if let Some(text) = args.get("operation").and_then(Value::as_str) {
        let text = text.trim();
        if !text.is_empty() {
            return text.to_string();
        }
    }
    let brief = args
        .as_object()
        .map(|object| {
            object
                .iter()
                .filter(|(key, _)| *key != "operation" && *key != "image_base64")
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    if brief.is_empty() {
        name.to_string()
    } else {
        format!("{name} {brief}")
    }
}

fn compact_result(value: &Value) -> String {
    let shown = strip_images(value.clone());
    let text = shown.to_string();
    if text.len() <= 500 {
        text
    } else {
        format!("{}…", &text[..500])
    }
}

fn strip_images(mut value: Value) -> Value {
    if let Some(object) = value.as_object_mut() {
        object.remove("jpeg_base64");
        object.remove("png_base64");
        object.remove("image_base64");
    }
    value
}

fn schema(properties: &[(&str, &str, &str)], required: &[&str]) -> Value {
    mcp::input_schema(properties, required)
}

fn new_id() -> String {
    format!("{:016x}", rand::random::<u64>())
}

fn emit_line(state: &AppState, role: &str, text: &str, image: Option<String>) {
    let _ = state.app.emit(
        "agent-line",
        json!({
            "id": new_id(),
            "role": role,
            "text": text,
            "image": image,
        }),
    );
}

fn emit_delta(state: &AppState, id: &str, text: &str, done: bool, reset: bool) {
    let _ = state.app.emit(
        "agent-delta",
        json!({
            "id": id,
            "text": text,
            "done": done,
            "reset": reset,
        }),
    );
}
