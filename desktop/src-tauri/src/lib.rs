mod actions;
mod agent;
mod device;
#[cfg(debug_assertions)]
mod dev_api;
mod http_api;
mod llm;
mod mcp;
mod protocol;
mod screen;
mod vision;

#[cfg(test)]
mod icon_art;

use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use actions::{AppState, DeviceView, finish_abort_if_idle, show_main};
use base64::Engine;
use device::{Device, find_xiao};
use protocol::parse_status;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const DEFAULT_PORT: u16 = 8765;
pub const STOP_SHORTCUT: &str = "Control+Option+Command+Escape";

#[derive(Serialize, Deserialize)]
struct FileConfig {
    token: String,
    port: u16,
    llm_base_url: String,
    llm_model: String,
    #[serde(default = "default_provider")]
    llm_provider: String,
    #[serde(default)]
    llm_api_key: String,
    #[serde(default)]
    scroll_inverted: Option<bool>,
}

fn default_provider() -> String {
    "openai".into()
}

#[derive(Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PerformTarget {
    Local,
    Remote { url: String, token: String },
}

#[tauri::command]
fn share_info(state: tauri::State<Arc<AppState>>) -> Value {
    let error = state.server_error.lock().ok().and_then(|slot| slot.clone());
    json!({
        "token": state.token(),
        "port": state.port,
        "urls": lan_urls(state.port),
        "listening": state.listening.load(Ordering::SeqCst),
        "serverError": error,
    })
}

#[tauri::command]
fn regenerate_token(state: tauri::State<Arc<AppState>>) -> Result<Value, String> {
    let token = new_token();
    *state.token.write().map_err(|err| err.to_string())? = token;
    save_config(&state)?;
    Ok(share_info(state))
}

#[tauri::command]
fn device_status(state: tauri::State<Arc<AppState>>) -> DeviceView {
    state
        .device_info
        .lock()
        .map(|slot| slot.clone())
        .unwrap_or_default()
}

#[tauri::command]
fn llm_settings(state: tauri::State<Arc<AppState>>) -> Value {
    settings_json(&state)
}

pub(crate) fn settings_json(state: &AppState) -> Value {
    let provider = state
        .llm_provider
        .read()
        .map(|value| llm::canonical_provider(&value).unwrap_or_else(|_| value.clone()))
        .unwrap_or_else(|_| "openai/openai".into());
    let key = state
        .llm_api_key
        .read()
        .map(|value| value.clone())
        .unwrap_or_default();
    json!({
        "provider": provider,
        "baseUrl": state.llm_base_url.read().map(|v| v.clone()).unwrap_or_default(),
        "model": state.llm_model.read().map(|v| v.clone()).unwrap_or_default(),
        "hasApiKey": !key.trim().is_empty(),
        "needsKey": llm::needs_key(&provider),
    })
}

#[tauri::command]
fn llm_providers() -> Vec<llm::ProviderChoice> {
    llm::providers()
}

#[tauri::command]
fn llm_catalog(provider: String) -> Result<Vec<llm::ModelChoice>, String> {
    llm::catalog_models(&provider)
}

#[tauri::command]
async fn list_llm_models(
    state: tauri::State<'_, Arc<AppState>>,
    provider: String,
    base_url: String,
    api_key: Option<String>,
) -> Result<Vec<llm::ModelChoice>, String> {
    let saved = state
        .llm_api_key
        .read()
        .map_err(|err| err.to_string())?
        .clone();
    let key = api_key
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(saved);
    llm::account_models(&provider, &base_url, &key).await
}

#[tauri::command]
fn scroll_direction(state: tauri::State<Arc<AppState>>) -> Value {
    let inverted = state.scroll_inverted.read().ok().and_then(|slot| *slot);
    json!({ "inverted": inverted })
}

#[tauri::command]
fn save_scroll_direction(state: tauri::State<Arc<AppState>>, inverted: bool) -> Result<(), String> {
    *state
        .scroll_inverted
        .write()
        .map_err(|err| err.to_string())? = Some(inverted);
    save_config(&state)
}

#[tauri::command]
async fn scroll_probe(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    actions::probe_scroll(&state)
        .await
        .map_err(|err| err.message())
}

#[tauri::command]
async fn scroll_probe_nudge(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    actions::probe_nudge(&state)
        .await
        .map_err(|err| err.message())
}

#[tauri::command]
fn save_llm(
    state: tauri::State<Arc<AppState>>,
    provider: String,
    base_url: String,
    model: String,
    api_key: Option<String>,
) -> Result<(), String> {
    write_llm(&state, provider, base_url, model, api_key)
}

pub(crate) fn write_llm(
    state: &AppState,
    provider: String,
    base_url: String,
    model: String,
    api_key: Option<String>,
) -> Result<(), String> {
    let provider = llm::canonical_provider(&provider)?;
    let model = model.trim();
    if model.is_empty() {
        return Err("Choose a model.".into());
    }
    let base = base_url.trim().trim_end_matches('/').to_string();
    *state.llm_provider.write().map_err(|err| err.to_string())? = provider;
    *state.llm_base_url.write().map_err(|err| err.to_string())? = base;
    *state.llm_model.write().map_err(|err| err.to_string())? = model.to_string();
    if let Some(key) = api_key {
        if !key.trim().is_empty() {
            *state.llm_api_key.write().map_err(|err| err.to_string())? = key.trim().to_string();
        }
    }
    save_config(&state)
}

#[tauri::command]
async fn perform(
    state: tauri::State<'_, Arc<AppState>>,
    target: PerformTarget,
    action: String,
    args: Option<Value>,
) -> Result<Value, String> {
    let args = args.unwrap_or_else(|| json!({}));
    match target {
        PerformTarget::Local => actions::execute(&state, &action, &args)
            .await
            .map_err(|err| err.message()),
        PerformTarget::Remote { url, token } => {
            remote_perform(&url, &token, &action, &args, &state.abort).await
        }
    }
}

#[tauri::command]
async fn preview_frame(
    state: tauri::State<'_, Arc<AppState>>,
    target: PerformTarget,
) -> Result<String, String> {
    let value = match target {
        PerformTarget::Local => actions::execute(&state, "screenshot", &json!({}))
            .await
            .map_err(|err| err.message())?,
        PerformTarget::Remote { url, token } => {
            remote_perform(&url, &token, "screenshot", &json!({}), &state.abort).await?
        }
    };
    value
        .get("jpeg_base64")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "no image".into())
}

#[tauri::command]
fn request_abort(app: tauri::AppHandle, state: tauri::State<Arc<AppState>>) {
    state.abort.store(true, Ordering::SeqCst);
    let _ = app.emit("agent-abort", json!({}));
    show_main(&app);
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn(async move {
        finish_abort_if_idle(&state).await;
    });
}

#[tauri::command]
fn clear_abort(state: tauri::State<Arc<AppState>>) {
    state.abort.store(false, Ordering::SeqCst);
}

#[tauri::command]
fn reveal(app: tauri::AppHandle) {
    show_main(&app);
}

#[tauri::command]
async fn run_agent(
    state: tauri::State<'_, Arc<AppState>>,
    target: PerformTarget,
    history: Vec<agent::Turn>,
    prompt: String,
) -> Result<(), String> {
    agent::run(state.inner(), target, history, prompt).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            share_info,
            regenerate_token,
            device_status,
            llm_settings,
            llm_providers,
            llm_catalog,
            list_llm_models,
            save_llm,
            scroll_direction,
            save_scroll_direction,
            scroll_probe,
            scroll_probe_nudge,
            perform,
            preview_frame,
            request_abort,
            clear_abort,
            reveal,
            run_agent
        ])
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            let state = Arc::new(load_state(app.handle())?);
            app.manage(Arc::clone(&state));
            install_tray(app.handle(), &state)?;
            app.global_shortcut()
                .on_shortcut(STOP_SHORTCUT, |app, _shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    let Some(state) = app.try_state::<Arc<AppState>>() else {
                        return;
                    };
                    let state = Arc::clone(&state);
                    state.abort.store(true, Ordering::SeqCst);
                    let _ = app.emit("agent-abort", json!({}));
                    show_main(app);
                    tauri::async_runtime::spawn(async move {
                        finish_abort_if_idle(&state).await;
                    });
                })
                .map_err(|err| -> Box<dyn std::error::Error> { Box::new(err) })?;
            if let Some(overlay) = app.get_webview_window("overlay") {
                let _ = overlay.set_focusable(false);
                let _ = overlay.set_content_protected(true);
                let _ = overlay.hide();
            }
            #[cfg(debug_assertions)]
            dev_api::spawn(Arc::clone(&state));
            let poll_state = Arc::clone(&state);
            tauri::async_runtime::spawn(async move {
                loop {
                    poll_device(&poll_state).await;
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to start LiveUse")
        .run(|app, event| {
            if let tauri::RunEvent::Reopen { .. } = event {
                show_main(app);
            }
        });
}

fn load_state(app: &tauri::AppHandle) -> Result<AppState, Box<dyn std::error::Error>> {
    let dir = app.path().app_config_dir()?;
    std::fs::create_dir_all(&dir)?;
    let config_path = dir.join("config.json");
    let config = if config_path.exists() {
        let text = std::fs::read_to_string(&config_path)?;
        serde_json::from_str(&text).unwrap_or_else(|_| default_config())
    } else {
        let config = default_config();
        std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
        config
    };
    Ok(AppState {
        device: Device::new(),
        token: RwLock::new(config.token),
        port: config.port,
        abort: std::sync::atomic::AtomicBool::new(false),
        agent_running: std::sync::atomic::AtomicBool::new(false),
        control_depth: std::sync::atomic::AtomicUsize::new(0),
        operation: std::sync::Mutex::new(None),
        held_keys: std::sync::Mutex::new(Vec::new()),
        pointer_gain: std::sync::Mutex::new((1.0, 1.0)),
        action_lock: tokio::sync::Mutex::new(()),
        app: app.clone(),
        server: std::sync::Mutex::new(None),
        listening: std::sync::atomic::AtomicBool::new(false),
        server_error: std::sync::Mutex::new(None),
        device_info: std::sync::Mutex::new(DeviceView::default()),
        llm_base_url: RwLock::new(config.llm_base_url),
        llm_model: RwLock::new(config.llm_model),
        llm_provider: RwLock::new(
            llm::canonical_provider(&config.llm_provider).unwrap_or(config.llm_provider),
        ),
        llm_api_key: RwLock::new(config.llm_api_key),
        scroll_inverted: RwLock::new(config.scroll_inverted),
        config_path,
        status_item: std::sync::Mutex::new(None),
        mcp_session: new_token(),
    })
}

fn default_config() -> FileConfig {
    FileConfig {
        token: new_token(),
        port: DEFAULT_PORT,
        llm_base_url: "https://api.openai.com/v1".into(),
        llm_model: "gpt-4o-mini".into(),
        llm_provider: default_provider(),
        llm_api_key: String::new(),
        scroll_inverted: None,
    }
}

fn save_config(state: &AppState) -> Result<(), String> {
    let config = FileConfig {
        token: state.token(),
        port: state.port,
        llm_base_url: state
            .llm_base_url
            .read()
            .map_err(|err| err.to_string())?
            .clone(),
        llm_model: state
            .llm_model
            .read()
            .map_err(|err| err.to_string())?
            .clone(),
        llm_provider: state
            .llm_provider
            .read()
            .map_err(|err| err.to_string())?
            .clone(),
        llm_api_key: state
            .llm_api_key
            .read()
            .map_err(|err| err.to_string())?
            .clone(),
        scroll_inverted: state
            .scroll_inverted
            .read()
            .map_err(|err| err.to_string())?
            .clone(),
    };
    std::fs::write(
        &state.config_path,
        serde_json::to_string_pretty(&config).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())
}

fn new_token() -> String {
    let bytes: [u8; 32] = rand::random();
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn lan_urls(port: u16) -> Vec<String> {
    let mut urls = Vec::new();
    if let Ok(ifaces) = if_addrs::get_if_addrs() {
        for iface in ifaces {
            if iface.is_loopback() {
                continue;
            }
            if let std::net::IpAddr::V4(ip) = iface.ip() {
                urls.push(format!("http://{ip}:{port}"));
            }
        }
    }
    urls.sort();
    urls.dedup();
    urls
}

fn install_tray(
    app: &tauri::AppHandle,
    state: &AppState,
) -> Result<(), Box<dyn std::error::Error>> {
    let status = MenuItem::with_id(app, "status", "○ No device", false, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&status, &separator, &open, &quit])?;
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))?;
    let _tray = TrayIconBuilder::with_id("liveuse")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    if let Ok(mut slot) = state.status_item.lock() {
        *slot = Some(status);
    }
    Ok(())
}

async fn poll_device(state: &Arc<AppState>) {
    match find_xiao() {
        Some(path)
            if state.device.path().as_deref() == Some(path.as_str())
                && !state.device.is_broken() =>
        {
            if let Ok(_guard) = state.action_lock.try_lock() {
                if let Ok(line) = state.device.transact("STATUS".into(), None).await {
                    if let Some(parsed) = parse_status(&line) {
                        let mut view = state
                            .device_info
                            .lock()
                            .map(|slot| slot.clone())
                            .unwrap_or_default();
                        view.connected = true;
                        view.path = Some(path);
                        view.uptime_ms = Some(parsed.uptime_ms);
                        view.firmware = Some(parsed.firmware);
                        view.error = None;
                        state.set_device_view(view);
                    }
                }
            }
        }
        Some(path) => match state.device.open(&path).await {
            Ok(parsed) => {
                state.set_device_view(DeviceView {
                    connected: true,
                    path: Some(path),
                    uptime_ms: Some(parsed.uptime_ms),
                    firmware: Some(parsed.firmware),
                    error: None,
                });
                state.ensure_server().await;
            }
            Err(err) => {
                state.device.close();
                state.stop_server();
                state.set_device_view(DeviceView {
                    connected: false,
                    error: Some(err),
                    ..DeviceView::default()
                });
            }
        },
        None => {
            if state.device.path().is_some() || state.listening.load(Ordering::SeqCst) {
                state.device.close();
                state.stop_server();
                state.set_device_view(DeviceView::default());
            }
        }
    }
}

pub(crate) async fn remote_perform(
    url: &str,
    token: &str,
    action: &str,
    args: &Value,
    abort: &AtomicBool,
) -> Result<Value, String> {
    let root = api_root(url);
    let endpoint = format!("{root}/v1/{action}");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|err| err.to_string())?;
    let is_get = matches!(
        action,
        "status" | "size" | "position" | "on_screen" | "pixel" | "screenshot"
    );
    let request = if is_get {
        client.get(with_query(&endpoint, args)).bearer_auth(token)
    } else {
        client.post(endpoint).bearer_auth(token).json(args)
    };
    let response = until_abort(abort, request.send())
        .await?
        .map_err(|err| err.to_string())?;
    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let bytes = until_abort(abort, response.bytes())
        .await?
        .map_err(|err| err.to_string())?;
    if content_type.contains("image/png") {
        if !status.is_success() {
            return Err(String::from_utf8_lossy(&bytes).into());
        }
        let image = image::load_from_memory(&bytes)
            .map_err(|err| err.to_string())?
            .to_rgba8();
        let png = actions::encode_png(&image).map_err(|err| err.message())?;
        let jpeg = actions::encode_jpeg(&image).map_err(|err| err.message())?;
        return Ok(json!({
            "width": image.width(),
            "height": image.height(),
            "png_base64": base64::engine::general_purpose::STANDARD.encode(png),
            "jpeg_base64": base64::engine::general_purpose::STANDARD.encode(jpeg),
        }));
    }
    let value: Value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({ "error": String::from_utf8_lossy(&bytes).to_string() }));
    if !status.is_success() {
        let message = value
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("remote error");
        return Err(message.to_string());
    }
    Ok(value)
}

async fn until_abort<T>(abort: &AtomicBool, fut: impl Future<Output = T>) -> Result<T, String> {
    tokio::pin!(fut);
    tokio::select! {
        biased;
        value = &mut fut => Ok(value),
        () = wait_abort(abort) => Err("aborted".into()),
    }
}

pub(crate) async fn wait_abort(abort: &AtomicBool) {
    while !abort.load(Ordering::SeqCst) {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

fn api_root(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    trimmed.strip_suffix("/v1").unwrap_or(trimmed).to_string()
}

fn with_query(endpoint: &str, args: &Value) -> String {
    let pairs = query_pairs(args);
    if pairs.is_empty() {
        return endpoint.to_string();
    }
    let query = pairs
        .into_iter()
        .map(|(key, value)| format!("{}={}", encode_query(&key), encode_query(&value)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{endpoint}?{query}")
}

fn encode_query(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn query_pairs(args: &Value) -> Vec<(String, String)> {
    let Some(object) = args.as_object() else {
        return Vec::new();
    };
    object
        .iter()
        .filter_map(|(key, value)| {
            let text = match value {
                Value::Null => return None,
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            Some((key.clone(), text))
        })
        .collect()
}
