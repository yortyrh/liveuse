use serde_json::{Value, json};

pub const MAX_HOLD_SECONDS: f64 = 300.0;
pub const MAX_KEY_REPEAT: i64 = 100;
pub const MAX_SCROLL_AMOUNT: i64 = 100;

pub enum Call {
    Forward {
        name: String,
        args: Value,
        modifiers: Vec<String>,
    },
    Key {
        keys: Vec<String>,
        repeat: u32,
    },
    Hold {
        keys: Vec<String>,
        seconds: f64,
    },
    Wait {
        seconds: f64,
    },
    Zoom {
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
    },
    Drag {
        start: (i32, i32),
        end: (i32, i32),
        modifiers: Vec<String>,
    },
}

pub fn prepare(name: &str, args: &Value) -> Result<Call, String> {
    match name {
        "left_click" => click_call("click", "left", args, true),
        "middle_click" => click_call("click", "middle", args, true),
        "click" | "double_click" | "triple_click" | "right_click" => {
            let anthropic = args.get("coordinate").is_some() || args.get("text").is_some();
            if !anthropic {
                return Ok(pass(name, args));
            }
            let (tool, button) = match name {
                "right_click" => ("right_click", "right"),
                _ => (name, "left"),
            };
            click_call(tool, button, args, false)
        }
        "mouse_move" => {
            let (x, y) =
                coordinate(args, "coordinate")?.ok_or_else(|| "missing coordinate".to_string())?;
            Ok(Call::Forward {
                name: "move_to".into(),
                args: json!({ "x": x, "y": y, "duration": 0 }),
                modifiers: Vec::new(),
            })
        }
        "left_mouse_down" => Ok(Call::Forward {
            name: "mouse_down".into(),
            args: json!({ "button": "left" }),
            modifiers: Vec::new(),
        }),
        "left_mouse_up" => Ok(Call::Forward {
            name: "mouse_up".into(),
            args: json!({ "button": "left" }),
            modifiers: Vec::new(),
        }),
        "left_click_drag" => {
            let start = coordinate(args, "start_coordinate")?
                .ok_or_else(|| "missing start_coordinate".to_string())?;
            let end =
                coordinate(args, "coordinate")?.ok_or_else(|| "missing coordinate".to_string())?;
            Ok(Call::Drag {
                start,
                end,
                modifiers: modifiers_of(args)?,
            })
        }
        "cursor_position" => Ok(Call::Forward {
            name: "position".into(),
            args: json!({}),
            modifiers: Vec::new(),
        }),
        "type" => Ok(Call::Forward {
            name: "write".into(),
            args: args.clone(),
            modifiers: Vec::new(),
        }),
        "key" => {
            let keys = chord(&required_str(args, "text")?)?;
            Ok(Call::Key {
                keys,
                repeat: repeat_of(args)?,
            })
        }
        "hold_key" => Ok(Call::Hold {
            keys: chord(&required_str(args, "text")?)?,
            seconds: seconds_of(args, "duration", MAX_HOLD_SECONDS)?,
        }),
        "wait" => Ok(Call::Wait {
            seconds: seconds_of(args, "duration", MAX_HOLD_SECONDS)?,
        }),
        "zoom" => {
            let region = args
                .get("region")
                .and_then(Value::as_array)
                .ok_or_else(|| "missing region".to_string())?;
            if region.len() != 4 {
                return Err("region must be [x0, y0, x1, y1]".into());
            }
            let corner = |index: usize| as_i32(&region[index]);
            Ok(Call::Zoom {
                x0: corner(0)?,
                y0: corner(1)?,
                x1: corner(2)?,
                y1: corner(3)?,
            })
        }
        "scroll" | "hscroll" => prepare_scroll(name, args),
        other => Ok(pass(other, args)),
    }
}

fn pass(name: &str, args: &Value) -> Call {
    Call::Forward {
        name: name.into(),
        args: args.clone(),
        modifiers: Vec::new(),
    }
}

fn click_call(name: &str, button: &str, args: &Value, force_button: bool) -> Result<Call, String> {
    let mut out = json!({ "button": button, "clicks": 1 });
    if !force_button {
        if let Some(button) = args.get("button").and_then(Value::as_str) {
            out["button"] = json!(button);
        }
        if let Some(clicks) = args.get("clicks") {
            out["clicks"] = clicks.clone();
        }
        if let Some(interval) = args.get("interval") {
            out["interval"] = interval.clone();
        }
    }
    if let Some((x, y)) = point_of(args)? {
        out["x"] = json!(x);
        out["y"] = json!(y);
    }
    if name == "right_click" {
        out["button"] = json!("right");
    }
    Ok(Call::Forward {
        name: name.into(),
        args: out,
        modifiers: modifiers_of(args)?,
    })
}

fn prepare_scroll(name: &str, args: &Value) -> Result<Call, String> {
    let Some(direction) = args.get("scroll_direction").and_then(Value::as_str) else {
        return Ok(Call::Forward {
            name: name.into(),
            args: args.clone(),
            modifiers: modifiers_of(args)?,
        });
    };
    let amount = as_i64(args.get("scroll_amount").ok_or("missing scroll_amount")?)?;
    if !(0..=MAX_SCROLL_AMOUNT).contains(&amount) {
        return Err(format!("scroll_amount must be 0 to {MAX_SCROLL_AMOUNT}"));
    }
    let (name, clicks) = match direction {
        "up" => ("scroll", amount),
        "down" => ("scroll", -amount),
        "right" => ("hscroll", amount),
        "left" => ("hscroll", -amount),
        _ => return Err("scroll_direction must be up, down, left, or right".into()),
    };
    let mut out = json!({ "clicks": clicks });
    if let Some((x, y)) = point_of(args)? {
        out["x"] = json!(x);
        out["y"] = json!(y);
    }
    Ok(Call::Forward {
        name: name.into(),
        args: out,
        modifiers: modifiers_of(args)?,
    })
}

fn point_of(args: &Value) -> Result<Option<(i32, i32)>, String> {
    if args.get("coordinate").is_some() {
        return coordinate(args, "coordinate");
    }
    match (opt_i32(args, "x"), opt_i32(args, "y")) {
        (Some(x), Some(y)) => Ok(Some((x, y))),
        (None, None) => Ok(None),
        _ => Err("x and y are both required".into()),
    }
}

pub fn coordinate(args: &Value, key: &str) -> Result<Option<(i32, i32)>, String> {
    let Some(value) = args.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let list = value
        .as_array()
        .ok_or_else(|| format!("{key} must be [x, y]"))?;
    if list.len() != 2 {
        return Err(format!("{key} must be [x, y]"));
    }
    Ok(Some((as_i32(&list[0])?, as_i32(&list[1])?)))
}

fn modifiers_of(args: &Value) -> Result<Vec<String>, String> {
    let Some(text) = args.get("text").and_then(Value::as_str) else {
        return Ok(Vec::new());
    };
    if text.trim().is_empty() {
        return Err("missing modifier".into());
    }
    chord(text)?
        .into_iter()
        .map(|key| canonical_modifier(&key))
        .collect()
}

pub fn chord(text: &str) -> Result<Vec<String>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("missing text".into());
    }
    let parts = if text == "+" {
        vec!["+".to_string()]
    } else if text.contains('+') {
        text.split('+')
            .map(|part| {
                if part.is_empty() {
                    "+".to_string()
                } else {
                    alias_key(part)
                }
            })
            .collect()
    } else {
        vec![alias_key(text)]
    };
    if parts.is_empty() {
        return Err("missing text".into());
    }
    Ok(parts)
}

fn alias_key(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "super" | "superleft" => "command".into(),
        "page_up" => "pageup".into(),
        "page_down" => "pagedown".into(),
        other => other.to_string(),
    }
}

fn canonical_modifier(name: &str) -> Result<String, String> {
    match name {
        "shift" | "shiftleft" => Ok("shift".into()),
        "shiftright" => Ok("shiftright".into()),
        "ctrl" | "control" | "ctrlleft" => Ok("ctrl".into()),
        "ctrlright" => Ok("ctrlright".into()),
        "alt" | "option" | "opt" | "altleft" | "optionleft" => Ok("alt".into()),
        "altright" | "optionright" => Ok("altright".into()),
        "command" | "cmd" | "meta" | "win" | "winleft" | "commandleft" => Ok("command".into()),
        "commandright" | "winright" | "cmdright" => Ok("commandright".into()),
        other => Err(format!("unsupported modifier: {other}")),
    }
}

fn repeat_of(args: &Value) -> Result<u32, String> {
    let repeat = match args.get("repeat") {
        None | Some(Value::Null) => 1,
        Some(value) => as_i64(value)?,
    };
    if !(1..=MAX_KEY_REPEAT).contains(&repeat) {
        return Err(format!("repeat must be 1 to {MAX_KEY_REPEAT}"));
    }
    Ok(repeat as u32)
}

fn seconds_of(args: &Value, key: &str, max: f64) -> Result<f64, String> {
    let Some(value) = args.get(key) else {
        return Err(format!("missing {key}"));
    };
    let seconds = value
        .as_f64()
        .or_else(|| value.as_i64().map(|n| n as f64))
        .ok_or_else(|| format!("{key} must be a number"))?;
    if !(0.0..=max).contains(&seconds) {
        return Err(format!("{key} must be 0 to {max} seconds"));
    }
    Ok(seconds)
}

fn required_str(args: &Value, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("missing {key}"))
}

fn opt_i32(args: &Value, key: &str) -> Option<i32> {
    args.get(key).and_then(|value| as_i32(value).ok())
}

fn as_i32(value: &Value) -> Result<i32, String> {
    let n = as_i64(value)?;
    i32::try_from(n).map_err(|_| "coordinate is out of range".into())
}

fn as_i64(value: &Value) -> Result<i64, String> {
    value
        .as_i64()
        .or_else(|| {
            value
                .as_f64()
                .and_then(|n| (n == n.trunc()).then_some(n as i64))
        })
        .ok_or_else(|| "expected an integer".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn keys_of(call: Call) -> Vec<String> {
        match call {
            Call::Key { keys, .. } | Call::Hold { keys, .. } => keys,
            _ => panic!("expected keys"),
        }
    }

    fn forward<'a>(call: &'a Call) -> (&'a str, &'a Value, &'a [String]) {
        match call {
            Call::Forward {
                name,
                args,
                modifiers,
            } => (name, args, modifiers),
            _ => panic!("expected forward"),
        }
    }

    #[test]
    fn key_chord_maps_super_and_pages() {
        let call = prepare("key", &json!({ "text": "ctrl+s", "repeat": 2 })).unwrap();
        match call {
            Call::Key { keys, repeat } => {
                assert_eq!(keys, vec!["ctrl", "s"]);
                assert_eq!(repeat, 2);
            }
            _ => panic!("expected key"),
        }
        assert_eq!(
            keys_of(prepare("key", &json!({ "text": "super+a" })).unwrap()),
            vec!["command", "a"]
        );
        assert_eq!(
            keys_of(prepare("key", &json!({ "text": "Return" })).unwrap()),
            vec!["return"]
        );
        assert_eq!(
            keys_of(prepare("key", &json!({ "text": "page_up" })).unwrap()),
            vec!["pageup"]
        );
        assert_eq!(
            keys_of(prepare("key", &json!({ "text": "page_down" })).unwrap()),
            vec!["pagedown"]
        );
    }

    #[test]
    fn repeat_and_duration_are_refused_outside_range() {
        assert!(prepare("key", &json!({ "text": "a", "repeat": 0 })).is_err());
        assert!(prepare("key", &json!({ "text": "a", "repeat": 101 })).is_err());
        assert!(prepare("wait", &json!({ "duration": 301 })).is_err());
        assert!(prepare("hold_key", &json!({ "text": "shift", "duration": 301 })).is_err());
        assert!(prepare("hold_key", &json!({ "text": "shift", "duration": -1 })).is_err());
    }

    #[test]
    fn click_takes_coordinate_and_modifiers() {
        let call = prepare(
            "left_click",
            &json!({ "coordinate": [10, 20], "text": "ctrl+shift" }),
        )
        .unwrap();
        let (name, args, modifiers) = forward(&call);
        assert_eq!(name, "click");
        assert_eq!(args["x"], 10);
        assert_eq!(args["y"], 20);
        assert_eq!(modifiers, ["ctrl", "shift"]);
        assert!(prepare("left_click", &json!({ "text": "a" })).is_err());
    }

    #[test]
    fn coordinate_must_be_a_pair() {
        assert!(prepare("mouse_move", &json!({ "coordinate": [1] })).is_err());
        assert!(prepare("mouse_move", &json!({})).is_err());
        let call = prepare("mouse_move", &json!({ "coordinate": [4, 5] })).unwrap();
        let (name, args, _) = forward(&call);
        assert_eq!(name, "move_to");
        assert_eq!(args["x"], 4);
        assert_eq!(args["y"], 5);
    }

    #[test]
    fn scroll_direction_becomes_signed_clicks() {
        let down = prepare(
            "scroll",
            &json!({
                "scroll_direction": "down",
                "scroll_amount": 3,
                "coordinate": [5, 6]
            }),
        )
        .unwrap();
        let (name, args, _) = forward(&down);
        assert_eq!(name, "scroll");
        assert_eq!(args["clicks"], -3);
        assert_eq!(args["x"], 5);
        assert_eq!(args["y"], 6);

        let up = prepare(
            "scroll",
            &json!({ "scroll_direction": "up", "scroll_amount": 3 }),
        )
        .unwrap();
        let (name, args, _) = forward(&up);
        assert_eq!(name, "scroll");
        assert_eq!(args["clicks"], 3);

        let left = prepare(
            "scroll",
            &json!({ "scroll_direction": "left", "scroll_amount": 2 }),
        )
        .unwrap();
        let (name, args, _) = forward(&left);
        assert_eq!(name, "hscroll");
        assert_eq!(args["clicks"], -2);

        let legacy = prepare("scroll", &json!({ "clicks": 4 })).unwrap();
        let (name, args, _) = forward(&legacy);
        assert_eq!(name, "scroll");
        assert_eq!(args["clicks"], 4);

        assert!(
            prepare(
                "scroll",
                &json!({ "scroll_direction": "down", "scroll_amount": 101 })
            )
            .is_err()
        );
    }
}
