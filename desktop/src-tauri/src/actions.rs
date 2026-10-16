use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, RwLock};
use std::time::Duration;

use base64::Engine;
use image::RgbaImage;
use serde::Serialize;
use serde_json::{Value, json};
use tauri::menu::MenuItem;
use tauri::{AppHandle, Emitter, Manager};

use crate::computer::{self, Call};
use crate::device::Device;
use crate::protocol::{
    Step, button_down, button_up, click_steps, counts_for, measured_gain, plan_hotkey,
    plan_key_down, plan_key_up, plan_press, plan_relative_move, plan_write, scroll_steps,
    signed_scroll,
};
use crate::screen::{self, Geometry};
use crate::vision::{self, Bounds};

#[derive(Debug)]
pub enum ActionError {
    Bad(String),
    NotFound,
    Aborted,
}

impl ActionError {
    pub fn message(&self) -> String {
        match self {
            Self::Bad(message) => message.clone(),
            Self::NotFound => "ImageNotFoundException".into(),
            Self::Aborted => "aborted".into(),
        }
    }
}

#[derive(Clone, Serialize)]
pub struct DeviceView {
    pub connected: bool,
    pub path: Option<String>,
    pub uptime_ms: Option<u64>,
    pub firmware: Option<String>,
    pub error: Option<String>,
    #[serde(default)]
    pub bootloader: bool,
    #[serde(default)]
    pub foreign: bool,
}

impl Default for DeviceView {
    fn default() -> Self {
        Self {
            connected: false,
            path: None,
            uptime_ms: None,
            firmware: None,
            error: None,
            bootloader: false,
            foreign: false,
        }
    }
}

pub struct AppState {
    pub device: Device,
    pub token: RwLock<String>,
    pub port: u16,
    pub abort: AtomicBool,
    /// True for the whole `agent::run`, including remote calls and model waits.
    /// Those do not hold `action_lock`, so Stop must leave `abort` set until the run ends.
    pub agent_running: AtomicBool,
    pub control_depth: AtomicUsize,
    pub operation: Mutex<Option<String>>,
    pub held_keys: Mutex<Vec<String>>,
    /// Screen points moved per HID count on each axis, as last measured.
    pub pointer_gain: Mutex<(f64, f64)>,
    pub action_lock: tokio::sync::Mutex<()>,
    pub app: AppHandle,
    pub server: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    pub listening: AtomicBool,
    pub server_error: Mutex<Option<String>>,
    pub device_info: Mutex<DeviceView>,
    pub flashing: AtomicBool,
    pub firmware_cache: Mutex<Option<crate::firmware::OfferCache>>,
    pub llm_base_url: RwLock<String>,
    pub llm_model: RwLock<String>,
    pub llm_provider: RwLock<String>,
    pub llm_api_key: RwLock<String>,
    pub scroll_inverted: RwLock<Option<bool>>,
    pub config_path: std::path::PathBuf,
    pub status_item: Mutex<Option<MenuItem<tauri::Wry>>>,
    pub mcp_session: String,
}

impl AppState {
    pub fn token(&self) -> String {
        self.token
            .read()
            .unwrap_or_else(|err| err.into_inner())
            .clone()
    }

    pub fn scroll_is_inverted(&self) -> bool {
        self.scroll_inverted
            .read()
            .map(|slot| *slot == Some(true))
            .unwrap_or(false)
    }

    pub fn begin_control(&self, text: &str) {
        self.control_depth.fetch_add(1, Ordering::SeqCst);
        self.set_operation(Some(text.to_string()));
    }

    pub fn end_control(&self) {
        let previous = self.control_depth.fetch_sub(1, Ordering::SeqCst);
        if previous <= 1 {
            self.control_depth.store(0, Ordering::SeqCst);
            self.set_operation(None);
        }
    }

    pub fn set_operation(&self, text: Option<String>) {
        let depth = self.control_depth.load(Ordering::SeqCst);
        if text.is_none() && depth > 0 {
            return;
        }
        if text.is_some() && depth == 0 {
            return;
        }
        if let Ok(mut slot) = self.operation.lock() {
            *slot = text.clone();
        }
        let _ = self.app.emit("operation", json!({ "text": text }));
        if let Some(overlay) = self.app.get_webview_window("overlay") {
            if text.is_some() {
                place_overlay(&overlay);
                let _ = overlay.set_focusable(false);
                let _ = overlay.set_content_protected(true);
                let _ = overlay.show();
            } else {
                let _ = overlay.hide();
            }
        }
        if text.is_some() {
            if let Some(main) = self.app.get_webview_window("main") {
                let _ = main.hide();
            }
        }
    }

    pub fn set_device_view(&self, view: DeviceView) {
        if let Ok(mut slot) = self.device_info.lock() {
            *slot = view.clone();
        }
        if let Ok(item) = self.status_item.lock() {
            if let Some(item) = item.as_ref() {
                let text = if view.connected {
                    "✓ XIAO connected"
                } else if view.bootloader {
                    "● Bootloader"
                } else if view.foreign {
                    "● Other firmware"
                } else {
                    "○ No device"
                };
                let _ = item.set_text(text);
            }
        }
        let _ = self.app.emit("device-status", &view);
    }

    pub async fn ensure_server(self: &std::sync::Arc<Self>) {
        if self.listening.load(Ordering::SeqCst) {
            return;
        }
        let listener = match tokio::net::TcpListener::bind(("0.0.0.0", self.port)).await {
            Ok(listener) => listener,
            Err(err) => {
                if let Ok(mut slot) = self.server_error.lock() {
                    *slot = Some(err.to_string());
                }
                return;
            }
        };
        let (tx, rx) = tokio::sync::oneshot::channel();
        if let Ok(mut slot) = self.server.lock() {
            *slot = Some(tx);
        }
        self.listening.store(true, Ordering::SeqCst);
        if let Ok(mut slot) = self.server_error.lock() {
            *slot = None;
        }
        let router = crate::http_api::router(std::sync::Arc::clone(self));
        tokio::spawn(async move {
            let _ = axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = rx.await;
                })
                .await;
        });
    }

    pub fn stop_server(&self) {
        if let Ok(mut slot) = self.server.lock() {
            slot.take();
        }
        self.listening.store(false, Ordering::SeqCst);
    }
}

pub fn place_overlay(window: &tauri::WebviewWindow) {
    let Ok(Some(monitor)) = window.current_monitor() else {
        return;
    };
    let area = monitor.work_area();
    let scale = monitor.scale_factor();
    let Ok(size) = window.outer_size() else {
        return;
    };
    let x = area.position.x as f64 / scale + 16.0;
    let y = area.position.y as f64 / scale + area.size.height as f64 / scale
        - size.height as f64 / scale
        - 16.0;
    let _ = window.set_position(tauri::LogicalPosition::new(x, y));
}

pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub async fn execute(state: &AppState, name: &str, args: &Value) -> Result<Value, ActionError> {
    let _guard = state.action_lock.lock().await;
    if state.abort.load(Ordering::SeqCst) {
        if !state.agent_running.load(Ordering::SeqCst) {
            state.abort.store(false, Ordering::SeqCst);
        }
        release_held(state).await;
        return Err(ActionError::Aborted);
    }
    let quiet = matches!(
        name,
        "status"
            | "size"
            | "position"
            | "cursor_position"
            | "on_screen"
            | "pixel"
            | "pixel_matches_color"
    );
    if !quiet {
        state.set_operation(Some(operation_label(name, args)));
    }
    let result = dispatch(state, name, args).await;
    let aborted = if state.agent_running.load(Ordering::SeqCst) {
        state.abort.load(Ordering::SeqCst)
    } else {
        state.abort.swap(false, Ordering::SeqCst)
    };
    if aborted {
        release_held(state).await;
    }
    if !quiet {
        state.set_operation(None);
    }
    if aborted {
        Err(ActionError::Aborted)
    } else {
        result
    }
}

async fn dispatch(state: &AppState, name: &str, args: &Value) -> Result<Value, ActionError> {
    match computer::prepare(name, args)? {
        Call::Forward {
            name,
            args,
            modifiers,
        } => with_modifiers(state, &modifiers, || dispatch_named(state, &name, &args)).await,
        Call::Key { keys, repeat } => {
            if keys.len() == 1 {
                run_steps(state, plan_press(&keys[0], repeat, 0.0)?).await?;
            } else {
                for _ in 0..repeat {
                    run_steps(state, plan_hotkey(&keys, 0.0)?).await?;
                }
            }
            Ok(json!({ "ok": true }))
        }
        Call::Hold { keys, seconds } => {
            with_modifiers(state, &keys, || async {
                sleep_abortable(state, Duration::from_secs_f64(seconds)).await?;
                Ok(json!({ "ok": true }))
            })
            .await
        }
        Call::Wait { seconds } => {
            sleep_abortable(state, Duration::from_secs_f64(seconds)).await?;
            Ok(json!({ "ok": true }))
        }
        Call::Zoom { x0, y0, x1, y1 } => zoom_action(state, x0, y0, x1, y1),
        Call::Drag {
            start,
            end,
            modifiers,
        } => {
            with_modifiers(state, &modifiers, || async {
                let grid = shot_grid()?;
                let start = grid.point_to_logical(start.0, start.1)?;
                let end = grid.point_to_logical(end.0, end.1)?;
                move_to(state, Some(start.0), Some(start.1), 0.0, "linear".into()).await?;
                run_steps(state, vec![button_down("left")?]).await?;
                let moved = move_to(state, Some(end.0), Some(end.1), 0.0, "linear".into()).await;
                let released = run_steps(state, vec![button_up("left")?]).await;
                moved?;
                released?;
                Ok(json!({ "ok": true }))
            })
            .await
        }
    }
}

async fn dispatch_named(state: &AppState, name: &str, args: &Value) -> Result<Value, ActionError> {
    let mapped;
    let args = if maps_motion(name) {
        mapped = map_motion(args, &shot_grid()?)?;
        &mapped
    } else {
        args
    };
    match name {
        "status" => Ok(json!(device_view(state))),
        "size" => {
            let grid = shot_grid()?;
            Ok(json!({ "width": grid.shot_width, "height": grid.shot_height }))
        }
        "position" => {
            let grid = shot_grid()?;
            let (x, y) = screen::cursor_position().map_err(ActionError::Bad)?;
            let (x, y) = grid.point_from_logical(x, y);
            Ok(json!({ "x": x, "y": y }))
        }
        "on_screen" => {
            let grid = shot_grid()?;
            let x = required_i32(args, "x")?;
            let y = required_i32(args, "y")?;
            Ok(json!({
                "on_screen": crate::protocol::on_screen(x, y, grid.shot_width, grid.shot_height)
            }))
        }
        "move_to" => {
            move_to(
                state,
                opt_i32(args, "x"),
                opt_i32(args, "y"),
                duration(args)?,
                tween(args)?,
            )
            .await?;
            Ok(json!({ "ok": true }))
        }
        "move" => {
            move_by(
                state,
                opt_i32(args, "dx").unwrap_or(0),
                opt_i32(args, "dy").unwrap_or(0),
                duration(args)?,
                tween(args)?,
            )
            .await?;
            Ok(json!({ "ok": true }))
        }
        "drag_to" | "drag" => {
            drag(state, name, args).await?;
            Ok(json!({ "ok": true }))
        }
        "click" | "double_click" | "triple_click" | "right_click" => {
            click(state, name, args).await?;
            Ok(json!({ "ok": true }))
        }
        "mouse_down" => {
            maybe_move(state, args).await?;
            run_steps(state, vec![button_down(&button_name(args))?]).await?;
            Ok(json!({ "ok": true }))
        }
        "mouse_up" => {
            maybe_move(state, args).await?;
            run_steps(state, vec![button_up(&button_name(args))?]).await?;
            Ok(json!({ "ok": true }))
        }
        "scroll" | "hscroll" => {
            maybe_move(state, args).await?;
            let steps = required_i32(args, "clicks").or_else(|_| required_i32(args, "steps"))?;
            let steps = signed_scroll(steps, state.scroll_is_inverted());
            run_steps(state, scroll_steps(name == "hscroll", steps)).await?;
            Ok(json!({ "ok": true }))
        }
        "write" => {
            let text = required_str(args, "text")?;
            run_steps(state, plan_write(&text, number(args, "interval", 0.0)?)?).await?;
            Ok(json!({ "ok": true }))
        }
        "press" => {
            run_steps(state, press_steps(args)?).await?;
            Ok(json!({ "ok": true }))
        }
        "key_down" => {
            run_steps(state, vec![plan_key_down(&required_str(args, "key")?)?]).await?;
            Ok(json!({ "ok": true }))
        }
        "key_up" => {
            run_steps(state, vec![plan_key_up(&required_str(args, "key")?)?]).await?;
            Ok(json!({ "ok": true }))
        }
        "hotkey" => {
            let keys = string_list(args, "keys")?;
            run_steps(state, plan_hotkey(&keys, number(args, "interval", 0.0)?)?).await?;
            Ok(json!({ "ok": true }))
        }
        "screenshot" => screenshot_action(state, args),
        "locate_on_screen" => {
            let grid = shot_grid()?;
            let args = map_locate_region(args, &grid)?;
            let found = find_boxes(state, &args).await?;
            let Some(bounds) = found.into_iter().next() else {
                return Err(ActionError::NotFound);
            };
            Ok(json_bounds(bounds_to_shot(&grid, bounds)))
        }
        "locate_center_on_screen" => {
            let grid = shot_grid()?;
            let args = map_locate_region(args, &grid)?;
            let found = find_boxes(state, &args).await?;
            let Some(bounds) = found.into_iter().next() else {
                return Err(ActionError::NotFound);
            };
            let bounds = bounds_to_shot(&grid, bounds);
            Ok(json!({
                "x": bounds.left as i32 + bounds.width as i32 / 2,
                "y": bounds.top as i32 + bounds.height as i32 / 2
            }))
        }
        "locate_all_on_screen" => {
            let grid = shot_grid()?;
            let args = map_locate_region(args, &grid)?;
            let found = find_boxes(state, &args).await?;
            Ok(json!({
                "boxes": found
                    .into_iter()
                    .map(|bounds| json_bounds(bounds_to_shot(&grid, bounds)))
                    .collect::<Vec<_>>()
            }))
        }
        "pixel" => {
            let image = grab(state).await?;
            let x = required_i32(args, "x")?;
            let y = required_i32(args, "y")?;
            let pixel = rgba_at(&image, x, y)?;
            Ok(json!({ "r": pixel[0], "g": pixel[1], "b": pixel[2] }))
        }
        "pixel_matches_color" => {
            let image = grab(state).await?;
            let x = required_i32(args, "x")?;
            let y = required_i32(args, "y")?;
            let pixel = rgba_at(&image, x, y)?;
            let expected = rgb_arg(args)?;
            let tolerance = number(args, "tolerance", 0.0)?;
            if !(0.0..=255.0).contains(&tolerance) {
                return Err(ActionError::Bad("tolerance must be 0 to 255".into()));
            }
            let tolerance = tolerance as u8;
            Ok(json!({
                "matches": vision::pixel_matches(
                    [pixel[0], pixel[1], pixel[2]],
                    expected,
                    tolerance
                )
            }))
        }
        other => Err(ActionError::Bad(format!("accion desconocida: {other}"))),
    }
}

fn device_view(state: &AppState) -> DeviceView {
    state
        .device_info
        .lock()
        .map(|slot| slot.clone())
        .unwrap_or_default()
}

fn geometry() -> Result<Geometry, ActionError> {
    screen::primary_geometry().map_err(ActionError::Bad)
}

fn shot_grid() -> Result<screen::Grid, ActionError> {
    let geometry = geometry()?;
    Ok(screen::Grid::from_logical(geometry.width, geometry.height))
}

fn maps_motion(name: &str) -> bool {
    matches!(
        name,
        "move_to"
            | "move"
            | "drag_to"
            | "drag"
            | "click"
            | "double_click"
            | "triple_click"
            | "right_click"
            | "mouse_down"
            | "mouse_up"
            | "scroll"
            | "hscroll"
            | "pixel"
            | "pixel_matches_color"
    )
}

fn map_motion(args: &Value, grid: &screen::Grid) -> Result<Value, ActionError> {
    let mut args = args.clone();
    match (opt_i32(&args, "x"), opt_i32(&args, "y")) {
        (Some(x), Some(y)) => {
            let (x, y) = grid.point_to_logical(x, y)?;
            args["x"] = json!(x);
            args["y"] = json!(y);
        }
        (Some(x), None) => {
            if x < 0 || x as u32 >= grid.shot_width {
                return Err(ActionError::Bad(format!(
                    "[{x}, …] is outside the {}x{} screenshot",
                    grid.shot_width, grid.shot_height
                )));
            }
            args["x"] = json!(screen::map_axis(x, grid.shot_width, grid.logical_width));
        }
        (None, Some(y)) => {
            if y < 0 || y as u32 >= grid.shot_height {
                return Err(ActionError::Bad(format!(
                    "[…, {y}] is outside the {}x{} screenshot",
                    grid.shot_width, grid.shot_height
                )));
            }
            args["y"] = json!(screen::map_axis(y, grid.shot_height, grid.logical_height));
        }
        (None, None) => {}
    }
    if opt_i32(&args, "dx").is_some() || opt_i32(&args, "dy").is_some() {
        let (dx, dy) = grid.delta_to_logical(
            opt_i32(&args, "dx").unwrap_or(0),
            opt_i32(&args, "dy").unwrap_or(0),
        );
        if args.get("dx").is_some() {
            args["dx"] = json!(dx);
        }
        if args.get("dy").is_some() {
            args["dy"] = json!(dy);
        }
    }
    Ok(args)
}

fn map_locate_region(args: &Value, grid: &screen::Grid) -> Result<Value, ActionError> {
    let mut args = args.clone();
    let Some(bounds) = region(&args)? else {
        return Ok(args);
    };
    if bounds.width == 0
        || bounds.height == 0
        || bounds.left.saturating_add(bounds.width) > grid.shot_width
        || bounds.top.saturating_add(bounds.height) > grid.shot_height
    {
        return Err(ActionError::Bad("region is outside the screenshot".into()));
    }
    let (left, width) = screen::map_span(
        bounds.left,
        bounds.width,
        grid.shot_width,
        grid.logical_width,
    );
    let (top, height) = screen::map_span(
        bounds.top,
        bounds.height,
        grid.shot_height,
        grid.logical_height,
    );
    if let Some(object) = args.as_object_mut() {
        object.remove("left");
        object.remove("top");
        object.remove("width");
        object.remove("height");
    }
    args["region"] = json!({ "left": left, "top": top, "width": width, "height": height });
    Ok(args)
}

fn bounds_to_shot(grid: &screen::Grid, bounds: Bounds) -> Bounds {
    let (left, width) = screen::map_span(
        bounds.left,
        bounds.width,
        grid.logical_width,
        grid.shot_width,
    );
    let (top, height) = screen::map_span(
        bounds.top,
        bounds.height,
        grid.logical_height,
        grid.shot_height,
    );
    Bounds {
        left,
        top,
        width,
        height,
    }
}

async fn with_modifiers<F, Fut>(
    state: &AppState,
    modifiers: &[String],
    action: F,
) -> Result<Value, ActionError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<Value, ActionError>>,
{
    let mut held = Vec::new();
    for key in modifiers {
        if let Err(err) = run_steps(state, vec![plan_key_down(key)?]).await {
            release_named(state, &held).await;
            return Err(err);
        }
        held.push(key.clone());
    }
    let result = action().await;
    release_named(state, &held).await;
    result
}

async fn release_named(state: &AppState, keys: &[String]) {
    for key in keys.iter().rev() {
        if let Ok(step) = plan_key_up(key) {
            let _ = run_steps(state, vec![step]).await;
        }
    }
}

fn protect_overlay(state: &AppState) {
    if let Some(overlay) = state.app.get_webview_window("overlay") {
        let _ = overlay.set_content_protected(true);
    }
}

fn capture(state: &AppState) -> Result<screen::Frame, ActionError> {
    protect_overlay(state);
    screen::capture_frame().map_err(ActionError::Bad)
}

fn capture_logical(state: &AppState) -> Result<RgbaImage, ActionError> {
    protect_overlay(state);
    screen::capture_logical().map_err(ActionError::Bad)
}

fn screenshot_action(state: &AppState, args: &Value) -> Result<Value, ActionError> {
    let with_png = args.get("format").and_then(Value::as_str) != Some("jpeg");
    let logical = capture_logical(state)?;
    let grid = screen::Grid::from_logical(logical.width(), logical.height());
    let Some(bounds) = shot_region(args, &grid)? else {
        let image = if (grid.shot_width, grid.shot_height) == logical.dimensions() {
            logical
        } else {
            screen::resize_image(&logical, grid.shot_width, grid.shot_height)
        };
        let mut value = encoded_json(&image, with_png)?;
        value["left"] = json!(0);
        value["top"] = json!(0);
        value["cropped"] = json!(false);
        return Ok(value);
    };
    let (left, width) =
        screen::map_span(bounds.left, bounds.width, grid.shot_width, logical.width());
    let (top, height) = screen::map_span(
        bounds.top,
        bounds.height,
        grid.shot_height,
        logical.height(),
    );
    let image = crop_region(
        &logical,
        Some(Bounds {
            left,
            top,
            width,
            height,
        }),
    )?;
    let mut value = encoded_json(&image, with_png)?;
    value["left"] = json!(bounds.left);
    value["top"] = json!(bounds.top);
    value["grid_width"] = json!(bounds.width);
    value["grid_height"] = json!(bounds.height);
    value["cropped"] = json!(true);
    Ok(value)
}

fn shot_region(args: &Value, grid: &screen::Grid) -> Result<Option<Bounds>, ActionError> {
    let Some(bounds) = region(args)? else {
        return Ok(None);
    };
    if bounds.width == 0
        || bounds.height == 0
        || bounds.left.saturating_add(bounds.width) > grid.shot_width
        || bounds.top.saturating_add(bounds.height) > grid.shot_height
    {
        return Err(ActionError::Bad("region is outside the screenshot".into()));
    }
    Ok(Some(bounds))
}

fn zoom_action(state: &AppState, x0: i32, y0: i32, x1: i32, y1: i32) -> Result<Value, ActionError> {
    let frame = capture(state)?;
    let grid = frame.grid();
    let image = screen::zoom_image(&frame.device, grid, x0, y0, x1, y1)?;
    let mut value = encoded_json(&image, true)?;
    value["x0"] = json!(x0);
    value["y0"] = json!(y0);
    value["x1"] = json!(x1);
    value["y1"] = json!(y1);
    value["shot_width"] = json!(grid.shot_width);
    value["shot_height"] = json!(grid.shot_height);
    Ok(value)
}

/// The OS scales HID counts (about 1.45x on a default macOS setup), so a move is
/// followed by up to MOVE_CORRECTIONS instant moves toward the target.
const MOVE_CORRECTIONS: usize = 5;
const MOVE_SETTLE: Duration = Duration::from_millis(25);

async fn move_to(
    state: &AppState,
    x: Option<i32>,
    y: Option<i32>,
    duration: f64,
    tween: String,
) -> Result<(), ActionError> {
    if x.is_none() && y.is_none() {
        return Ok(());
    }
    let geometry = geometry()?;
    let mut current = screen::cursor_position().map_err(ActionError::Bad)?;
    let target = (x.unwrap_or(current.0), y.unwrap_or(current.1));
    let mut duration = duration;
    for _ in 0..=MOVE_CORRECTIONS {
        let dx = target.0.saturating_sub(current.0);
        let dy = target.1.saturating_sub(current.1);
        if dx == 0 && dy == 0 {
            break;
        }
        let gain = pointer_gain(state);
        let sent = (counts_for(dx, gain.0), counts_for(dy, gain.1));
        run_move(state, sent.0, sent.1, duration, tween.clone()).await?;
        duration = 0.0;
        sleep_abortable(state, MOVE_SETTLE).await?;
        let next = screen::cursor_position().map_err(ActionError::Bad)?;
        let pinned_x = next.0 <= 0 || next.0 >= geometry.width as i32 - 1;
        let pinned_y = next.1 <= 0 || next.1 >= geometry.height as i32 - 1;
        let learned = (
            if pinned_x {
                gain.0
            } else {
                measured_gain(sent.0, next.0 - current.0, gain.0)
            },
            if pinned_y {
                gain.1
            } else {
                measured_gain(sent.1, next.1 - current.1, gain.1)
            },
        );
        if let Ok(mut slot) = state.pointer_gain.lock() {
            *slot = learned;
        }
        current = next;
    }
    Ok(())
}

fn pointer_gain(state: &AppState) -> (f64, f64) {
    state
        .pointer_gain
        .lock()
        .map(|slot| *slot)
        .unwrap_or((1.0, 1.0))
}

async fn move_by(
    state: &AppState,
    dx: i32,
    dy: i32,
    duration: f64,
    tween: String,
) -> Result<(), ActionError> {
    if dx == 0 && dy == 0 {
        return Ok(());
    }
    let (cx, cy) = screen::cursor_position().map_err(ActionError::Bad)?;
    move_to(
        state,
        Some(cx.saturating_add(dx)),
        Some(cy.saturating_add(dy)),
        duration,
        tween,
    )
    .await
}

async fn run_move(
    state: &AppState,
    dx: i32,
    dy: i32,
    duration: f64,
    tween: String,
) -> Result<(), ActionError> {
    let chunks = plan_relative_move(dx, dy, duration, &tween)?;
    for chunk in chunks {
        check_abort(state)?;
        state
            .device
            .transact(format!("MOVE {} {}", chunk.dx, chunk.dy), None)
            .await
            .map_err(ActionError::Bad)?;
        sleep_abortable(state, chunk.sleep_after).await?;
    }
    Ok(())
}

/// Raw positive wheel under the cursor. The check button is already under it,
/// so the probe does not move the pointer. Compensation stays off, and the window stays up.
pub async fn probe_scroll(state: &AppState) -> Result<(), ActionError> {
    if state.device.path().is_none() {
        return Err(ActionError::Bad("no device".into()));
    }
    let _guard = state.action_lock.lock().await;
    run_steps(state, scroll_steps(false, 4)).await
}

/// A few pixels out and back, without hiding the window, to see if the cursor is on the button.
pub async fn probe_nudge(state: &AppState) -> Result<(), ActionError> {
    if state.device.path().is_none() {
        return Err(ActionError::Bad("no device".into()));
    }
    let _guard = state.action_lock.lock().await;
    run_move(state, 4, 0, 0.0, "linear".into()).await?;
    run_move(state, -4, 0, 0.0, "linear".into()).await
}

async fn maybe_move(state: &AppState, args: &Value) -> Result<(), ActionError> {
    move_to(
        state,
        opt_i32(args, "x"),
        opt_i32(args, "y"),
        0.0,
        "linear".into(),
    )
    .await
}

async fn drag(state: &AppState, name: &str, args: &Value) -> Result<(), ActionError> {
    let button = button_name(args);
    run_steps(state, vec![button_down(&button)?]).await?;
    let moved = if name == "drag_to" {
        move_to(
            state,
            opt_i32(args, "x"),
            opt_i32(args, "y"),
            duration(args)?,
            tween(args)?,
        )
        .await
    } else {
        move_by(
            state,
            opt_i32(args, "dx").unwrap_or(0),
            opt_i32(args, "dy").unwrap_or(0),
            duration(args)?,
            tween(args)?,
        )
        .await
    };
    let released = run_steps(state, vec![button_up(&button)?]).await;
    moved?;
    released
}

async fn click(state: &AppState, name: &str, args: &Value) -> Result<(), ActionError> {
    maybe_move(state, args).await?;
    let (button, clicks) = match name {
        "double_click" => (button_name(args), 2),
        "triple_click" => (button_name(args), 3),
        "right_click" => ("right".to_string(), 1),
        _ => (button_name(args), number(args, "clicks", 1.0)? as u32),
    };
    run_steps(
        state,
        click_steps(&button, clicks.max(1), number(args, "interval", 0.0)?)?,
    )
    .await
}

fn press_steps(args: &Value) -> Result<Vec<Step>, ActionError> {
    let interval = number(args, "interval", 0.0)?;
    let presses = number(args, "presses", 1.0)? as u32;
    if let Some(list) = args.get("keys").and_then(Value::as_array) {
        let mut steps = Vec::new();
        for key in list {
            let key = key
                .as_str()
                .ok_or_else(|| ActionError::Bad("keys must be a list of strings".into()))?;
            steps.extend(plan_press(key, 1, interval)?);
        }
        return Ok(steps);
    }
    Ok(plan_press(
        &required_str(args, "key")?,
        presses.max(1),
        interval,
    )?)
}

async fn run_steps(state: &AppState, steps: Vec<Step>) -> Result<(), ActionError> {
    for step in steps {
        check_abort(state)?;
        match step {
            Step::Sleep(duration) => sleep_abortable(state, duration).await?,
            Step::Cdc { line, silent } => {
                let timeout = if silent {
                    Some(Duration::from_millis(
                        40 + 30 * line.len().saturating_sub(5) as u64,
                    ))
                } else {
                    None
                };
                state
                    .device
                    .transact(line.clone(), timeout)
                    .await
                    .map_err(ActionError::Bad)?;
                note_held(state, &line);
            }
        }
    }
    Ok(())
}

fn note_held(state: &AppState, line: &str) {
    let Ok(mut keys) = state.held_keys.lock() else {
        return;
    };
    if let Some(name) = line.strip_prefix("KDOWN ") {
        keys.push(name.to_string());
    } else if let Some(name) = line.strip_prefix("KUP ") {
        keys.retain(|key| key != name);
    }
}

pub async fn finish_abort_if_idle(state: &AppState) {
    if let Ok(_guard) = state.action_lock.try_lock() {
        release_held(state).await;
        if !state.agent_running.load(Ordering::SeqCst) {
            state.abort.store(false, Ordering::SeqCst);
        }
        state.set_operation(None);
    }
}

async fn release_held(state: &AppState) {
    let keys = state
        .held_keys
        .lock()
        .map(|mut slot| std::mem::take(&mut *slot))
        .unwrap_or_default();
    for button in ["left", "right", "middle"] {
        let _ = state.device.transact(format!("MUP {button}"), None).await;
    }
    for key in keys.into_iter().rev() {
        let _ = state.device.transact(format!("KUP {key}"), None).await;
    }
}

fn check_abort(state: &AppState) -> Result<(), ActionError> {
    if state.abort.load(Ordering::SeqCst) {
        Err(ActionError::Aborted)
    } else {
        Ok(())
    }
}

async fn sleep_abortable(state: &AppState, total: Duration) -> Result<(), ActionError> {
    let mut left = total;
    while left > Duration::ZERO {
        check_abort(state)?;
        let slice = left.min(Duration::from_millis(50));
        tokio::time::sleep(slice).await;
        left = left.saturating_sub(slice);
    }
    check_abort(state)
}

async fn grab(state: &AppState) -> Result<RgbaImage, ActionError> {
    capture_logical(state)
}

fn crop_region(image: &RgbaImage, bounds: Option<Bounds>) -> Result<RgbaImage, ActionError> {
    let Some(bounds) = bounds else {
        return Ok(image.clone());
    };
    if bounds.left.saturating_add(bounds.width) > image.width()
        || bounds.top.saturating_add(bounds.height) > image.height()
    {
        return Err(ActionError::Bad("region is outside the screen".into()));
    }
    Ok(
        image::imageops::crop_imm(image, bounds.left, bounds.top, bounds.width, bounds.height)
            .to_image(),
    )
}

async fn find_boxes(state: &AppState, args: &Value) -> Result<Vec<Bounds>, ActionError> {
    let image = grab(state).await?;
    let needle = vision::decode_png_base64(&required_str(args, "image_base64")?)?;
    let grayscale = args
        .get("grayscale")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let confidence = args.get("confidence").and_then(Value::as_f64);
    vision::locate(&image, &needle, region(args)?, grayscale, confidence).map_err(ActionError::Bad)
}

fn encoded_json(image: &RgbaImage, with_png: bool) -> Result<Value, ActionError> {
    let jpeg = encode_jpeg(image)?;
    let mut value = json!({
        "width": image.width(),
        "height": image.height(),
        "jpeg_base64": base64::engine::general_purpose::STANDARD.encode(jpeg),
    });
    if with_png {
        value["png_base64"] =
            json!(base64::engine::general_purpose::STANDARD.encode(encode_png(image)?));
    }
    Ok(value)
}

pub fn encode_png(image: &RgbaImage) -> Result<Vec<u8>, ActionError> {
    let mut bytes = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut bytes);
    image::ImageEncoder::write_image(
        encoder,
        image.as_raw(),
        image.width(),
        image.height(),
        image::ExtendedColorType::Rgba8,
    )
    .map_err(|err| ActionError::Bad(err.to_string()))?;
    Ok(bytes)
}

pub fn encode_jpeg(image: &RgbaImage) -> Result<Vec<u8>, ActionError> {
    let rgb = image::DynamicImage::ImageRgba8(image.clone()).to_rgb8();
    let mut bytes = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 70);
    encoder
        .encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|err| ActionError::Bad(err.to_string()))?;
    Ok(bytes)
}

fn rgba_at(image: &RgbaImage, x: i32, y: i32) -> Result<[u8; 4], ActionError> {
    if x < 0 || y < 0 || x as u32 >= image.width() || y as u32 >= image.height() {
        return Err(ActionError::Bad("pixel is outside the screen".into()));
    }
    Ok(image.get_pixel(x as u32, y as u32).0)
}

fn json_bounds(bounds: Bounds) -> Value {
    json!({
        "left": bounds.left,
        "top": bounds.top,
        "width": bounds.width,
        "height": bounds.height
    })
}

fn region(args: &Value) -> Result<Option<Bounds>, ActionError> {
    let Some(value) = args.get("region") else {
        let left = opt_i32(args, "left");
        let top = opt_i32(args, "top");
        let width = opt_i32(args, "width");
        let height = opt_i32(args, "height");
        if left.is_none() && top.is_none() && width.is_none() && height.is_none() {
            return Ok(None);
        }
        return Ok(Some(Bounds {
            left: left.ok_or_else(|| ActionError::Bad("region incompleta".into()))? as u32,
            top: top.ok_or_else(|| ActionError::Bad("region incompleta".into()))? as u32,
            width: width.ok_or_else(|| ActionError::Bad("region incompleta".into()))? as u32,
            height: height.ok_or_else(|| ActionError::Bad("region incompleta".into()))? as u32,
        }));
    };
    if value.is_null() {
        return Ok(None);
    }
    let left = value.get("left").and_then(Value::as_i64).unwrap_or(-1);
    let top = value.get("top").and_then(Value::as_i64).unwrap_or(-1);
    let width = value.get("width").and_then(Value::as_i64).unwrap_or(-1);
    let height = value.get("height").and_then(Value::as_i64).unwrap_or(-1);
    if left < 0 || top < 0 || width <= 0 || height <= 0 {
        return Err(ActionError::Bad("region invalida".into()));
    }
    Ok(Some(Bounds {
        left: left as u32,
        top: top as u32,
        width: width as u32,
        height: height as u32,
    }))
}

fn rgb_arg(args: &Value) -> Result<[u8; 3], ActionError> {
    let channel = |value: Option<i64>| {
        value
            .and_then(|n| u8::try_from(n).ok())
            .ok_or_else(|| ActionError::Bad("rgb channels must be integers 0 to 255".into()))
    };
    if let Some(list) = args.get("rgb").and_then(Value::as_array) {
        if list.len() != 3 {
            return Err(ActionError::Bad("rgb must be [r, g, b]".into()));
        }
        return Ok([
            channel(list[0].as_i64())?,
            channel(list[1].as_i64())?,
            channel(list[2].as_i64())?,
        ]);
    }
    Ok([
        channel(Some(required_i32(args, "r")?.into()))?,
        channel(Some(required_i32(args, "g")?.into()))?,
        channel(Some(required_i32(args, "b")?.into()))?,
    ])
}

fn operation_label(name: &str, args: &Value) -> String {
    if let Some(text) = args.get("operation").and_then(Value::as_str) {
        if !text.trim().is_empty() {
            return text.trim().to_string();
        }
    }
    match name {
        "write" => format!("Typing {}", preview(args.get("text"))),
        "press" => format!(
            "Pressing {}",
            preview(args.get("key").or_else(|| args.get("keys")))
        ),
        "hotkey" => format!("Shortcut {}", preview(args.get("keys"))),
        "move_to" => format!(
            "Moving to {}, {}",
            preview(args.get("x")),
            preview(args.get("y"))
        ),
        "move" | "drag" => format!(
            "Moving {}, {}",
            preview(args.get("dx")),
            preview(args.get("dy"))
        ),
        "drag_to" => format!(
            "Dragging to {}, {}",
            preview(args.get("x")),
            preview(args.get("y"))
        ),
        "click" | "double_click" | "triple_click" | "right_click" => format!("Click {name}"),
        "screenshot" => "Capturing the screen".into(),
        "locate_on_screen" | "locate_center_on_screen" | "locate_all_on_screen" => {
            "Searching the screen".into()
        }
        other => other.replace('_', " "),
    }
}

fn preview(value: Option<&Value>) -> String {
    let Some(value) = value else {
        return "…".into();
    };
    let text = value.to_string();
    if text.len() > 42 {
        format!("{}…", &text[..42])
    } else {
        text
    }
}

fn button_name(args: &Value) -> String {
    args.get("button")
        .and_then(Value::as_str)
        .unwrap_or("left")
        .to_string()
}

fn duration(args: &Value) -> Result<f64, ActionError> {
    number(args, "duration", 0.0)
}

fn tween(args: &Value) -> Result<String, ActionError> {
    Ok(args
        .get("tween")
        .and_then(Value::as_str)
        .unwrap_or("linear")
        .to_string())
}

fn number(args: &Value, key: &str, default: f64) -> Result<f64, ActionError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(value) => value
            .as_f64()
            .or_else(|| value.as_i64().map(|n| n as f64))
            .ok_or_else(|| ActionError::Bad(format!("{key} must be a number"))),
    }
}

fn opt_i32(args: &Value, key: &str) -> Option<i32> {
    args.get(key).and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_f64().map(|n| n as i64))
            .and_then(|n| i32::try_from(n).ok())
    })
}

fn required_i32(args: &Value, key: &str) -> Result<i32, ActionError> {
    opt_i32(args, key).ok_or_else(|| ActionError::Bad(format!("missing {key}")))
}

fn required_str(args: &Value, key: &str) -> Result<String, ActionError> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| ActionError::Bad(format!("missing {key}")))
}

fn string_list(args: &Value, key: &str) -> Result<Vec<String>, ActionError> {
    args.get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| ActionError::Bad(format!("missing {key}")))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| ActionError::Bad(format!("{key} must be a list of strings")))
        })
        .collect()
}

impl From<String> for ActionError {
    fn from(message: String) -> Self {
        Self::Bad(message)
    }
}
