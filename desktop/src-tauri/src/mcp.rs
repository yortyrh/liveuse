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
     A shifted symbol such as ? or + adds Shift by itself. An uppercase letter does not: list shift. ",
    key_names!()
);

pub const MOVE_TO: &str = "Move the cursor to an absolute point on the size grid. The position is \
checked and corrected until the cursor is within about 1 point of it. An omitted axis keeps its current value. \
duration in seconds animates the path; below 0.1 is instant.";

pub const MOVE: &str = "Move the cursor dx, dy points from where it is, corrected like move_to. Positive dy goes down.";

pub const CLICK: &str = "Click. With x and y, the cursor first moves there, corrected like move_to. clicks defaults to 1.";

pub const SCREENSHOT: &str = "Capture the primary display on the size grid, origin at the top left. \
With left, top, width and height, only that region comes back, at the same scale: add left and top \
to a point in the region image to get its screen coordinates.";

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
        "serverInfo": { "name": "liveuse", "version": "0.1.0" }
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
        Ok(value) if name == "screenshot" => Ok(screenshot_content(value)),
        Ok(value) => Ok(text_content(&value.to_string(), false)),
        Err(ActionError::NotFound) => Ok(text_content("ImageNotFoundException", true)),
        Err(err) => Ok(text_content(&err.message(), true)),
    }
}

fn screenshot_content(value: Value) -> Value {
    let width = value.get("width").and_then(Value::as_u64).unwrap_or(0);
    let height = value.get("height").and_then(Value::as_u64).unwrap_or(0);
    let left = value.get("left").and_then(Value::as_u64).unwrap_or(0);
    let top = value.get("top").and_then(Value::as_u64).unwrap_or(0);
    let jpeg = value
        .get("jpeg_base64")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let caption = if left == 0 && top == 0 {
        format!(
            "screenshot {width}x{height}. Click coordinates use this same grid, origin at the top left."
        )
    } else {
        format!(
            "screenshot of the {width}x{height} region at left {left}, top {top}. \
             A point (x, y) in this image is (x + {left}, y + {top}) on screen."
        )
    };
    json!({
        "content": [
            { "type": "text", "text": caption },
            { "type": "image", "data": jpeg, "mimeType": "image/jpeg" }
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
                ("operation", "string", OPERATION),
            ],
            &[],
        ),
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
        "Vertical wheel. Positive scrolls up, negative scrolls down. clicks is the number of notches.",
        object(
            &[
                ("clicks", "integer", "notches"),
                ("x", "integer", "move first"),
                ("y", "integer", "move first"),
                ("operation", "string", OPERATION),
            ],
            &["clicks"],
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
        "locate_on_screen",
        "Find a base64 PNG on the screen. Without confidence, an exact match. With confidence, normalized correlation from 0 to 1. If there is no match, the error is ImageNotFoundException.",
        object(
            &[
                (
                    "image_base64",
                    "string",
                    "PNG at the same resolution as screenshot",
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
