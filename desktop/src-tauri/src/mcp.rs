use serde_json::{Value, json};

use crate::actions::{ActionError, AppState, execute};

macro_rules! key_names {
    () => {
        "Key names. Modifiers: command (cmd, meta, win), ctrl (control), alt (option, opt), shift; \
         commandright, ctrlright, altright and shiftright are the right-hand ones. \
         Keys: enter (return), esc (escape), tab, space, backspace (erases to the left, the Mac delete key), \
         delete (del, erases to the right), insert, home, end, pageup (pgup), pagedown (pgdn), \
         up, down, left, right, f1 to f12, minus, equal, leftbracket, rightbracket, backslash, \
         semicolon, quote, grave, comma, dot, slash. Single characters: a to z, 0 to 9 and printable \
         ASCII punctuation. Not available: capslock, fn, media and volume keys, f13 to f24, numpad keys, printscreen."
    };
}

pub const OPERATION: &str = "short text shown on the on-screen status card while this runs";

pub const WRITE: &str = "Type text as a US-layout keyboard. Only printable ASCII (space through ~); \
a newline presses Enter and a tab presses Tab. Accents, ñ and emoji are rejected: percent-encode \
them in URLs (qu%C3%A9), or on macOS type a dead key with hotkey [\"option\", \"e\"] and then the \
letter. The OS still applies its text settings: on macOS two spaces can become \". \" and words \
can be autocorrected. Returns once the text has been typed. interval is seconds between characters; 0 is fastest.";

pub const PRESS: &str = concat!(
    "Tap one key (presses times), or tap each key in keys one after another. keys is a sequence, \
     not a combination: use hotkey for shortcuts. ",
    key_names!()
);

pub const HOTKEY: &str = concat!(
    "Shortcut: hold the keys in order, then release them in reverse. macOS examples: \
     [\"command\", \"space\"] Spotlight, [\"command\", \"t\"] new tab, [\"command\", \"l\"] address bar, \
     [\"command\", \",\"] settings, [\"command\", \"shift\", \"/\"] Help menu, [\"command\", \"=\"] zoom in. \
     A shifted symbol such as ? or + adds Shift by itself. An uppercase letter does not: list shift. \
     A shortcut that opens something (Spotlight, a window, a menu, a tab) takes a moment to appear: \
     call wait with about 0.5 before typing into it. ",
    key_names!()
);

pub const WAIT: &str = "Pause for duration seconds, at most 300; fractions such as 0.3 work. \
The OS and apps react with a delay: after a shortcut or click that opens Spotlight, an app, a window, \
a menu or a page, wait about 0.3 to 1 second before typing or clicking into it, longer for an app \
launch or a page load. Keys sent too early are lost.";

pub const MOVE_TO: &str = "Move the cursor to an absolute point on the size grid. The position is \
checked and corrected until the cursor is within about 1 point of it. An omitted axis keeps its current value. \
duration in seconds animates the path; below 0.1 is instant.";

pub const MOVE: &str = "Move the cursor dx, dy points from where it is, corrected like move_to. Positive dy goes down.";

pub const CLICK: &str = "Click. With x and y, the cursor first moves there, corrected like move_to. clicks defaults to 1.";

pub const SPACE: &str = "coordinate system of every coordinate in this call. pixels (default): \
absolute pixels of the full screenshot. normalized: 0 to 1000 on both axes whatever the screen size, \
[0, 0] is the top left and [1000, 1000] the bottom right. Use the system you natively point in; many \
vision models point on the 0-1000 grid. Never rescale a point from one system to the other: give it \
as you located it and label it. Results such as position stay in pixels.";

const SPACE_KEYS: &[&str] = &[
    "x",
    "y",
    "dx",
    "dy",
    "coordinate",
    "start_coordinate",
    "region",
];

/// Caption for a full screenshot of `width` x `height`, shared by MCP and the built-in agent.
pub fn grid_note(width: u64, height: u64) -> String {
    format!(
        "Coordinates are pixels of this image, x 0 to {} and y 0 to {}, or 0 to 1000 on both axes \
         when the call sets space to normalized. Use the system you natively point in; many vision \
         models point on the 0-1000 grid. Never rescale a point yourself: label it with the system \
         it came from.",
        width.saturating_sub(1),
        height.saturating_sub(1)
    )
}

pub const SCREENSHOT: &str = "Capture the primary display. Coordinates in every tool use this image, \
origin at the top left, Y down: either its pixels, or 0 to 1000 on both axes when the call sets space \
to normalized. Use the system you natively point in; many vision models point on the 0-1000 grid. \
The image stays at logical resolution unless the display exceeds \
2576 px on the long edge or 4784 visual tokens, in which case it is scaled down and clicks are \
scaled back up. With left, top, width and height, those edges are on this grid and the image is \
the 1:1 logical crop, so another tool can inspect it.";

pub const PIXEL: &str = "RGB color of one screen pixel. Each call captures the whole screen; to \
inspect an area, take a screenshot of a region instead.";

pub fn input_schema(properties: &[(&str, &str, &str)], required: &[&str]) -> Value {
    let mut props = serde_json::Map::new();
    for (name, kind, description) in properties {
        let mut prop = json!({ "type": kind, "description": description });
        match (*name, *kind) {
            ("rgb", "array") => {
                prop["items"] = json!({ "type": "integer", "minimum": 0, "maximum": 255 });
            }
            ("coordinate" | "start_coordinate", "array") => {
                prop["items"] = json!({ "type": "integer" });
                prop["minItems"] = json!(2);
                prop["maxItems"] = json!(2);
            }
            ("region", "array") => {
                prop["items"] = json!({ "type": "integer" });
                prop["minItems"] = json!(4);
                prop["maxItems"] = json!(4);
            }
            (_, "array") => prop["items"] = json!({ "type": "string" }),
            ("region", "object") => {
                prop["properties"] = json!({
                    "left": { "type": "integer" },
                    "top": { "type": "integer" },
                    "width": { "type": "integer" },
                    "height": { "type": "integer" }
                });
                prop["required"] = json!(["left", "top", "width", "height"]);
            }
            _ => {}
        }
        props.insert((*name).to_string(), prop);
    }
    let mut required: Vec<&str> = required.to_vec();
    if properties
        .iter()
        .any(|(name, _, _)| SPACE_KEYS.contains(name))
    {
        props.insert(
            "space".into(),
            json!({ "type": "string", "enum": ["pixels", "normalized"], "description": SPACE }),
        );
        required.push("space");
    }
    json!({
        "type": "object",
        "properties": props,
        "required": required
    })
}

pub async fn handle(state: &AppState, body: Value) -> Option<Value> {
    if body.get("id").is_none() {
        return None;
    }
    let id = body.get("id").cloned().unwrap_or(Value::Null);
    let method = body.get("method").and_then(Value::as_str).unwrap_or("");
    let result = match method {
        "initialize" => Ok(initialize(&body)),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tool_defs() })),
        "tools/call" => tools_call(state, &body).await,
        _ => Err((-32601, "method not found".to_string())),
    };
    Some(match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err((code, message)) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": code, "message": message }
        }),
    })
}

fn initialize(body: &Value) -> Value {
    let requested = body
        .pointer("/params/protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or("2025-03-26");
    let version = match requested {
        "2024-11-05" | "2025-03-26" | "2025-06-18" => requested,
        _ => "2025-03-26",
    };
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": "liveuse", "version": "0.2.0" }
    })
}

async fn tools_call(state: &AppState, body: &Value) -> Result<Value, (i32, String)> {
    let name = body
        .pointer("/params/name")
        .and_then(Value::as_str)
        .ok_or((-32602, "missing tool name".to_string()))?;
    let args = body
        .pointer("/params/arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    match execute(state, name, &args).await {
        Ok(value) if name == "screenshot" || name == "zoom" => Ok(image_content(name, value)),
        Ok(value) if name == "cursor_position" => Ok(text_content(&cursor_text(&value), false)),
        Ok(value) => Ok(text_content(
            &phrase(name, &args).unwrap_or_else(|| value.to_string()),
            false,
        )),
        Err(ActionError::NotFound) => Ok(text_content("ImageNotFoundException", true)),
        Err(err) => Ok(text_content(&err.message(), true)),
    }
}

fn cursor_text(value: &Value) -> String {
    let x = value.get("x").and_then(Value::as_i64).unwrap_or(0);
    let y = value.get("y").and_then(Value::as_i64).unwrap_or(0);
    format!("X={x},Y={y}")
}

fn phrase(name: &str, args: &Value) -> Option<String> {
    match name {
        "left_click" | "right_click" | "middle_click" | "double_click" | "triple_click" => {
            Some("Clicked.".into())
        }
        "left_click_drag" => Some("Dragged.".into()),
        "mouse_move" => Some("Moved.".into()),
        "left_mouse_down" => Some("Pressed.".into()),
        "left_mouse_up" => Some("Released.".into()),
        "type" => Some("Typed.".into()),
        "key" | "hold_key" => Some("Pressed.".into()),
        "wait" => Some("Waited.".into()),
        "scroll" if args.get("scroll_direction").is_some() => Some("Scrolled.".into()),
        _ => None,
    }
}

fn image_content(name: &str, value: Value) -> Value {
    let width = value.get("width").and_then(Value::as_u64).unwrap_or(0);
    let height = value.get("height").and_then(Value::as_u64).unwrap_or(0);
    let png = value
        .get("png_base64")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let caption = if name == "zoom" {
        let x0 = value.get("x0").and_then(Value::as_i64).unwrap_or(0);
        let y0 = value.get("y0").and_then(Value::as_i64).unwrap_or(0);
        let x1 = value.get("x1").and_then(Value::as_i64).unwrap_or(0);
        let y1 = value.get("y1").and_then(Value::as_i64).unwrap_or(0);
        let shot_w = value.get("shot_width").and_then(Value::as_u64).unwrap_or(0);
        let shot_h = value
            .get("shot_height")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        format!(
            "zoom {width}x{height} of the region ({x0}, {y0})-({x1}, {y1}) in pixels. \
             Click coordinates stay on the full {shot_w}x{shot_h} screenshot, origin at the top left, \
             or 0 to 1000 of the full screen with space normalized."
        )
    } else if value.get("cropped").and_then(Value::as_bool) == Some(true) {
        let left = value.get("left").and_then(Value::as_u64).unwrap_or(0);
        let top = value.get("top").and_then(Value::as_u64).unwrap_or(0);
        let grid_w = value
            .get("grid_width")
            .and_then(Value::as_u64)
            .unwrap_or(width);
        let grid_h = value
            .get("grid_height")
            .and_then(Value::as_u64)
            .unwrap_or(height);
        if grid_w == width && grid_h == height {
            format!(
                "screenshot of the {width}x{height} region at left {left}, top {top}. \
                 A point (x, y) in this image is (x + {left}, y + {top}) on the screenshot grid."
            )
        } else {
            format!(
                "logical {width}x{height} crop of the {grid_w}x{grid_h} region at left {left}, top {top}. \
                 A point (x, y) in this image is ({left} + x * {grid_w} / {width}, {top} + y * {grid_h} / {height}) \
                 on the screenshot grid."
            )
        }
    } else {
        format!(
            "screenshot {width}x{height}, origin at the top left, Y down. {}",
            grid_note(width, height)
        )
    };
    json!({
        "content": [
            { "type": "text", "text": caption },
            { "type": "image", "data": png, "mimeType": "image/png" }
        ],
        "isError": false
    })
}

fn text_content(text: &str, is_error: bool) -> Value {
    json!({
        "content": [{ "type": "text", "text": text }],
        "isError": is_error
    })
}

fn tool_defs() -> Vec<Value> {
    let object = input_schema;
    let mut tools = Vec::new();
    let push = |tools: &mut Vec<Value>, name: &str, description: &str, schema: Value| {
        tools.push(json!({
            "name": name,
            "description": description,
            "inputSchema": schema
        }));
    };
    push(
        &mut tools,
        "status",
        "Status of the connected XIAO.",
        object(&[], &[]),
    );
    push(
        &mut tools,
        "size",
        "Width and height of the primary display, in points. The origin is the top-left corner and Y grows downward.",
        object(&[], &[]),
    );
    push(
        &mut tools,
        "position",
        "Current cursor position on the same grid as size.",
        object(&[], &[]),
    );
    push(
        &mut tools,
        "on_screen",
        "True if the point is inside the screen. The pixel (width, height) is outside.",
        object(&[("x", "integer", "x"), ("y", "integer", "y")], &["x", "y"]),
    );
    push(
        &mut tools,
        "move_to",
        MOVE_TO,
        object(
            &[
                ("x", "integer", "target x; omit to keep the current x"),
                ("y", "integer", "target y; omit to keep the current y"),
                ("duration", "number", "seconds"),
                (
                    "tween",
                    "string",
                    "linear, ease_in_quad, ease_out_quad, ease_in_out_quad, ease_in_bounce, ease_in_elastic",
                ),
                ("operation", "string", OPERATION),
            ],
            &[],
        ),
    );
    push(
        &mut tools,
        "move",
        MOVE,
        object(
            &[
                ("dx", "integer", "pixels, positive to the right"),
                ("dy", "integer", "pixels, positive downward"),
                ("duration", "number", "seconds"),
                ("tween", "string", "tween"),
                ("operation", "string", OPERATION),
            ],
            &[],
        ),
    );
    push(
        &mut tools,
        "drag_to",
        "Drag to a coordinate while holding the button.",
        object(
            &[
                ("x", "integer", "x"),
                ("y", "integer", "y"),
                ("duration", "number", "seconds"),
                ("tween", "string", "tween"),
                ("button", "string", "left, right, or middle"),
                ("operation", "string", OPERATION),
            ],
            &[],
        ),
    );
    push(
        &mut tools,
        "drag",
        "Drag a relative distance.",
        object(
            &[
                ("dx", "integer", "dx"),
                ("dy", "integer", "dy"),
                ("duration", "number", "seconds"),
                ("tween", "string", "tween"),
                ("button", "string", "left, right, or middle"),
                ("operation", "string", OPERATION),
            ],
            &[],
        ),
    );
    push(
        &mut tools,
        "click",
        CLICK,
        object(
            &[
                ("x", "integer", "x"),
                ("y", "integer", "y"),
                ("clicks", "integer", "repeats"),
                ("interval", "number", "seconds between clicks"),
                ("button", "string", "left, right, or middle"),
                ("operation", "string", OPERATION),
            ],
            &[],
        ),
    );
    push(
        &mut tools,
        "double_click",
        "Left double-click, unless button says otherwise.",
        object(
            &[
                ("x", "integer", "x"),
                ("y", "integer", "y"),
                (
                    "coordinate",
                    "array",
                    "[x, y] on the screenshot; used instead of x and y",
                ),
                (
                    "text",
                    "string",
                    "modifiers to hold, such as shift or ctrl+shift",
                ),
                ("interval", "number", "seconds"),
                ("button", "string", "button"),
                ("operation", "string", OPERATION),
            ],
            &[],
        ),
    );
    push(
        &mut tools,
        "triple_click",
        "Triple-click.",
        object(
            &[
                ("x", "integer", "x"),
                ("y", "integer", "y"),
                (
                    "coordinate",
                    "array",
                    "[x, y] on the screenshot; used instead of x and y",
                ),
                (
                    "text",
                    "string",
                    "modifiers to hold, such as shift or ctrl+shift",
                ),
                ("interval", "number", "seconds"),
                ("button", "string", "button"),
                ("operation", "string", OPERATION),
            ],
            &[],
        ),
    );
    push(
        &mut tools,
        "right_click",
        "Right-click.",
        object(
            &[
                ("x", "integer", "x"),
                ("y", "integer", "y"),
                (
                    "coordinate",
                    "array",
                    "[x, y] on the screenshot; used instead of x and y",
                ),
                (
                    "text",
                    "string",
                    "modifiers to hold, such as shift or ctrl+shift",
                ),
                ("operation", "string", OPERATION),
            ],
            &[],
        ),
    );
    push(
        &mut tools,
        "left_click",
        "Left-click. coordinate is optional; without it the click lands at the cursor. text holds modifiers such as shift, ctrl, alt, super, or ctrl+shift.",
        object(
            &[
                ("coordinate", "array", "[x, y] on the screenshot"),
                ("text", "string", "modifiers to hold during the click"),
                ("operation", "string", OPERATION),
            ],
            &[],
        ),
    );
    push(
        &mut tools,
        "middle_click",
        "Middle-click. Same coordinate and text as left_click.",
        object(
            &[
                ("coordinate", "array", "[x, y] on the screenshot"),
                ("text", "string", "modifiers to hold during the click"),
                ("operation", "string", OPERATION),
            ],
            &[],
        ),
    );
    push(
        &mut tools,
        "left_click_drag",
        "Press at start_coordinate, drag to coordinate, and release. text holds modifiers for the drag.",
        object(
            &[
                ("start_coordinate", "array", "[x, y] where the drag starts"),
                ("coordinate", "array", "[x, y] where the drag ends"),
                ("text", "string", "modifiers to hold"),
                ("operation", "string", OPERATION),
            ],
            &["start_coordinate", "coordinate"],
        ),
    );
    push(
        &mut tools,
        "mouse_move",
        "Move the cursor to coordinate without clicking.",
        object(
            &[
                ("coordinate", "array", "[x, y] on the screenshot"),
                ("operation", "string", OPERATION),
            ],
            &["coordinate"],
        ),
    );
    push(
        &mut tools,
        "left_mouse_down",
        "Press the left mouse button at the current cursor.",
        object(&[("operation", "string", OPERATION)], &[]),
    );
    push(
        &mut tools,
        "left_mouse_up",
        "Release the left mouse button at the current cursor.",
        object(&[("operation", "string", OPERATION)], &[]),
    );
    push(
        &mut tools,
        "mouse_down",
        "Hold a mouse button down.",
        object(
            &[
                ("button", "string", "left, right, or middle"),
                ("x", "integer", "x"),
                ("y", "integer", "y"),
                ("operation", "string", OPERATION),
            ],
            &[],
        ),
    );
    push(
        &mut tools,
        "mouse_up",
        "Release a mouse button.",
        object(
            &[
                ("button", "string", "left, right, or middle"),
                ("x", "integer", "x"),
                ("y", "integer", "y"),
                ("operation", "string", OPERATION),
            ],
            &[],
        ),
    );
    push(
        &mut tools,
        "scroll",
        "Scroll the wheel. Either clicks (positive up, negative down) or scroll_direction up, down, left, or right with scroll_amount. coordinate or x, y move there first. text holds modifiers. left and right scroll horizontally.",
        object(
            &[
                ("clicks", "integer", "notches; positive up"),
                ("scroll_direction", "string", "up, down, left, or right"),
                ("scroll_amount", "integer", "notches, 0 to 100"),
                ("coordinate", "array", "[x, y] on the screenshot"),
                ("x", "integer", "move first"),
                ("y", "integer", "move first"),
                ("text", "string", "modifiers to hold"),
                ("operation", "string", OPERATION),
            ],
            &[],
        ),
    );
    push(
        &mut tools,
        "hscroll",
        "Horizontal wheel. Positive scrolls right.",
        object(
            &[
                ("clicks", "integer", "notches"),
                ("x", "integer", "x"),
                ("y", "integer", "y"),
                ("operation", "string", OPERATION),
            ],
            &["clicks"],
        ),
    );
    push(
        &mut tools,
        "write",
        WRITE,
        object(
            &[
                (
                    "text",
                    "string",
                    "printable ASCII; \\n is Enter, \\t is Tab",
                ),
                ("interval", "number", "seconds between characters"),
                ("operation", "string", OPERATION),
            ],
            &["text"],
        ),
    );
    push(
        &mut tools,
        "press",
        PRESS,
        object(
            &[
                ("key", "string", "one key name or character"),
                ("keys", "array", "keys tapped one after another"),
                ("presses", "integer", "repeats of key"),
                ("interval", "number", "seconds"),
                ("operation", "string", OPERATION),
            ],
            &[],
        ),
    );
    push(
        &mut tools,
        "key_down",
        "Hold a key down until key_up. Same key names as press.",
        object(
            &[("key", "string", "key"), ("operation", "string", OPERATION)],
            &["key"],
        ),
    );
    push(
        &mut tools,
        "key_up",
        "Release a key.",
        object(
            &[("key", "string", "key"), ("operation", "string", OPERATION)],
            &["key"],
        ),
    );
    push(
        &mut tools,
        "type",
        "Type literal text at the current keyboard focus. Same limits as write.",
        object(
            &[
                (
                    "text",
                    "string",
                    "printable ASCII; \\n is Enter, \\t is Tab",
                ),
                ("interval", "number", "seconds between characters"),
                ("operation", "string", OPERATION),
            ],
            &["text"],
        ),
    );
    push(
        &mut tools,
        "key",
        "Press a key or a plus-joined combination such as Return, ctrl+s, or alt+Tab. repeat is 1 to 100. super is Command.",
        object(
            &[
                ("text", "string", "key or combination"),
                ("repeat", "integer", "1 to 100, default 1"),
                ("operation", "string", OPERATION),
            ],
            &["text"],
        ),
    );
    push(
        &mut tools,
        "hold_key",
        "Hold a key or combination for duration seconds, at most 300.",
        object(
            &[
                ("text", "string", "key or combination"),
                ("duration", "number", "seconds, 0 to 300"),
                ("operation", "string", OPERATION),
            ],
            &["text", "duration"],
        ),
    );
    push(
        &mut tools,
        "wait",
        WAIT,
        object(
            &[
                ("duration", "number", "seconds, 0 to 300"),
                ("operation", "string", OPERATION),
            ],
            &["duration"],
        ),
    );
    push(
        &mut tools,
        "hotkey",
        HOTKEY,
        object(
            &[
                ("keys", "array", "keys in order"),
                ("interval", "number", "seconds between steps"),
                ("operation", "string", OPERATION),
            ],
            &["keys"],
        ),
    );
    push(
        &mut tools,
        "screenshot",
        SCREENSHOT,
        object(
            &[
                ("left", "integer", "region left edge on screen"),
                ("top", "integer", "region top edge on screen"),
                ("width", "integer", "region width"),
                ("height", "integer", "region height"),
                ("operation", "string", OPERATION),
            ],
            &[],
        ),
    );
    push(
        &mut tools,
        "zoom",
        "Magnify a region of the full screenshot. region is [x0, y0, x1, y1] on that screenshot. The image is the physical pixels of that area, scaled to fit the screenshot size. Later clicks stay on the full screenshot grid. Zoom in before clicking a small target whose position is uncertain.",
        object(
            &[
                ("region", "array", "[x0, y0, x1, y1] on the full screenshot"),
                ("operation", "string", OPERATION),
            ],
            &["region"],
        ),
    );
    push(
        &mut tools,
        "cursor_position",
        "Cursor position on the screenshot grid, as X and Y.",
        object(&[], &[]),
    );
    push(
        &mut tools,
        "locate_on_screen",
        "Find a base64 PNG on the screen. Without confidence, an exact match. With confidence, normalized correlation from 0 to 1. If there is no match, the error is ImageNotFoundException.",
        object(
            &[
                (
                    "image_base64",
                    "string",
                    "PNG at logical resolution, such as a screenshot region crop",
                ),
                ("grayscale", "boolean", "compare luma"),
                ("confidence", "number", "0 to 1"),
                ("region", "object", "{left, top, width, height}"),
                ("operation", "string", OPERATION),
            ],
            &["image_base64"],
        ),
    );
    push(
        &mut tools,
        "locate_center_on_screen",
        "Center of the first locate_on_screen match.",
        object(
            &[
                ("image_base64", "string", "PNG"),
                ("grayscale", "boolean", "luma"),
                ("confidence", "number", "0 to 1"),
                ("region", "object", "region"),
                ("operation", "string", OPERATION),
            ],
            &["image_base64"],
        ),
    );
    push(
        &mut tools,
        "locate_all_on_screen",
        "Up to 50 boxes, from the top left downward. Empty list if there is no match.",
        object(
            &[
                ("image_base64", "string", "PNG"),
                ("grayscale", "boolean", "luma"),
                ("confidence", "number", "0 to 1"),
                ("region", "object", "region"),
                ("operation", "string", OPERATION),
            ],
            &["image_base64"],
        ),
    );
    push(
        &mut tools,
        "pixel",
        PIXEL,
        object(&[("x", "integer", "x"), ("y", "integer", "y")], &["x", "y"]),
    );
    push(
        &mut tools,
        "pixel_matches_color",
        "True if the pixel is within tolerance units of each channel. Captures the whole screen.",
        object(
            &[
                ("x", "integer", "x"),
                ("y", "integer", "y"),
                ("rgb", "array", "[r, g, b], each 0 to 255"),
                ("tolerance", "integer", "slack per channel, 0 to 255"),
            ],
            &["x", "y", "rgb"],
        ),
    );
    tools
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(name: &str) -> Value {
        tool_defs()
            .into_iter()
            .find(|tool| tool["name"] == name)
            .expect("tool exists")
    }

    #[test]
    fn space_is_offered_only_where_coordinates_are() {
        for name in [
            "left_click",
            "click",
            "move",
            "zoom",
            "scroll",
            "left_click_drag",
        ] {
            let schema = &tool(name)["inputSchema"];
            assert_eq!(
                schema["properties"]["space"]["enum"],
                json!(["pixels", "normalized"]),
                "{name}"
            );
            assert!(
                schema["required"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("space")),
                "{name}"
            );
        }
        for name in ["write", "hotkey", "wait", "size", "screenshot"] {
            assert!(
                tool(name)["inputSchema"]["properties"]
                    .get("space")
                    .is_none(),
                "{name}"
            );
        }
    }
}
