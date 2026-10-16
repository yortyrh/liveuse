use std::time::Duration;

pub const MAX_AXIS: i32 = 2000;
const TYPE_CHUNK: usize = 80;
const SCROLL_CHUNK: i32 = 40;
/// TYPE prints nothing on success. The firmware runs lines in order, so this echo
/// ends with " done" only after the typing before it has finished.
const TYPE_SYNC: &str = "ECHO type done";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    Cdc {
        line: String,
        silent: bool,
    },
    Sleep(Duration),
    /// Characters the HID keyboard cannot type. The host pastes this text.
    Paste(String),
}

pub fn tokens_equal(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in left.iter().zip(right.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceStatus {
    pub uptime_ms: u64,
    pub firmware: String,
}

pub fn parse_status(line: &str) -> Option<DeviceStatus> {
    let line = line.trim();
    let rest = line.strip_prefix("OK ")?;
    let mut uptime_ms = None;
    let mut firmware = None;
    for part in rest.split_whitespace() {
        if let Some(value) = part.strip_prefix("uptime_ms=") {
            uptime_ms = value.parse().ok();
        } else if let Some(value) = part.strip_prefix("fw=") {
            firmware = Some(value.to_string());
        }
    }
    Some(DeviceStatus {
        uptime_ms: uptime_ms?,
        firmware: firmware?,
    })
}

pub fn on_screen(x: i32, y: i32, width: u32, height: u32) -> bool {
    x >= 0 && y >= 0 && (x as u32) < width && (y as u32) < height
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyKind {
    Modifier,
    Named,
}

fn unsupported(name: &str) -> bool {
    const NONE: &[&str] = &[
        "accept",
        "add",
        "apps",
        "browserback",
        "browserfavorites",
        "browserforward",
        "browserhome",
        "browserrefresh",
        "browsersearch",
        "browserstop",
        "capslock",
        "clear",
        "convert",
        "decimal",
        "divide",
        "execute",
        "final",
        "fn",
        "hanguel",
        "hangul",
        "hanja",
        "help",
        "junja",
        "kana",
        "kanji",
        "launchapp1",
        "launchapp2",
        "launchmail",
        "launchmediaselect",
        "modechange",
        "multiply",
        "nexttrack",
        "nonconvert",
        "num0",
        "num1",
        "num2",
        "num3",
        "num4",
        "num5",
        "num6",
        "num7",
        "num8",
        "num9",
        "numlock",
        "pause",
        "playpause",
        "prevtrack",
        "print",
        "printscreen",
        "prntscrn",
        "prtsc",
        "prtscr",
        "scrolllock",
        "select",
        "separator",
        "sleep",
        "stop",
        "subtract",
        "volumedown",
        "volumemute",
        "volumeup",
        "yen",
    ];
    if NONE.contains(&name) {
        return true;
    }
    let Some(rest) = name.strip_prefix('f') else {
        return false;
    };
    let Ok(n) = rest.parse::<u8>() else {
        return false;
    };
    (13..=24).contains(&n)
}

fn named_key(name: &str) -> Result<(String, KeyKind), String> {
    let name = name.trim().to_ascii_lowercase();
    if name.is_empty() {
        return Err("empty key".into());
    }
    if unsupported(&name) {
        return Err(format!("unsupported key: {name}"));
    }
    let (canon, kind) = match name.as_str() {
        "cmd" | "command" | "meta" | "win" | "winleft" | "commandleft" => {
            ("cmd", KeyKind::Modifier)
        }
        "winright" | "commandright" | "cmdright" => ("winright", KeyKind::Modifier),
        "ctrl" | "control" | "ctrlleft" => ("ctrl", KeyKind::Modifier),
        "ctrlright" => ("ctrlright", KeyKind::Modifier),
        "alt" | "opt" | "option" | "altleft" | "optionleft" => ("alt", KeyKind::Modifier),
        "altright" | "optionright" => ("altright", KeyKind::Modifier),
        "shift" | "shiftleft" => ("shift", KeyKind::Modifier),
        "shiftright" => ("shiftright", KeyKind::Modifier),
        "return" | "enter" | "\n" | "\r" => ("enter", KeyKind::Named),
        "escape" | "esc" => ("esc", KeyKind::Named),
        "del" | "delete" => ("delete", KeyKind::Named),
        "pgup" | "pageup" => ("pageup", KeyKind::Named),
        "pgdn" | "pagedown" => ("pagedown", KeyKind::Named),
        "\t" | "tab" => ("tab", KeyKind::Named),
        "space" => ("space", KeyKind::Named),
        "backspace" | "minus" | "equal" | "leftbracket" | "rightbracket" | "backslash"
        | "semicolon" | "quote" | "grave" | "comma" | "dot" | "slash" | "right" | "left"
        | "down" | "up" | "insert" | "home" | "end" => (name.as_str(), KeyKind::Named),
        other => {
            if let Some(rest) = other.strip_prefix('f')
                && let Ok(n) = rest.parse::<u8>()
                && (1..=12).contains(&n)
            {
                return Ok((other.to_string(), KeyKind::Named));
            }
            return Err(format!("unsupported key: {other}"));
        }
    };
    Ok((canon.to_string(), kind))
}

enum Token {
    Modifier(String),
    Named(String),
    Char(char),
}

fn token_of(raw: &str) -> Result<Token, String> {
    if raw == " " {
        return Ok(Token::Named("space".into()));
    }
    let chars: Vec<char> = raw.chars().collect();
    if chars.len() == 1 {
        let ch = chars[0];
        if ch == '\n' || ch == '\r' {
            return Ok(Token::Named("enter".into()));
        }
        if ch == '\t' {
            return Ok(Token::Named("tab".into()));
        }
        if (0x20..=0x7E).contains(&(ch as u32)) {
            return Ok(Token::Char(ch));
        }
        return Err(format!("unsupported key: {raw}"));
    }
    match named_key(raw)? {
        (name, KeyKind::Modifier) => Ok(Token::Modifier(name)),
        (name, KeyKind::Named) => Ok(Token::Named(name)),
    }
}

/// The firmware chord parser takes letters, digits and key names, not punctuation.
/// Returns the US key that carries `ch`, and whether it needs Shift.
fn punctuation_key(ch: char) -> Option<(bool, Token)> {
    let (shift, base) = match ch {
        '-' => (false, "minus"),
        '=' => (false, "equal"),
        '[' => (false, "leftbracket"),
        ']' => (false, "rightbracket"),
        '\\' => (false, "backslash"),
        ';' => (false, "semicolon"),
        '\'' => (false, "quote"),
        '`' => (false, "grave"),
        ',' => (false, "comma"),
        '.' => (false, "dot"),
        '/' => (false, "slash"),
        '_' => (true, "minus"),
        '+' => (true, "equal"),
        '{' => (true, "leftbracket"),
        '}' => (true, "rightbracket"),
        '|' => (true, "backslash"),
        ':' => (true, "semicolon"),
        '"' => (true, "quote"),
        '~' => (true, "grave"),
        '<' => (true, "comma"),
        '>' => (true, "dot"),
        '?' => (true, "slash"),
        '!' => (true, "1"),
        '@' => (true, "2"),
        '#' => (true, "3"),
        '$' => (true, "4"),
        '%' => (true, "5"),
        '^' => (true, "6"),
        '&' => (true, "7"),
        '*' => (true, "8"),
        '(' => (true, "9"),
        ')' => (true, "0"),
        _ => return None,
    };
    let token = match base.chars().next() {
        Some(digit) if base.len() == 1 => Token::Char(digit),
        _ => Token::Named(base.to_string()),
    };
    Some((shift, token))
}

fn expand_punctuation(parsed: Vec<Token>) -> Vec<Token> {
    let mut has_shift = parsed
        .iter()
        .any(|token| matches!(token, Token::Modifier(name) if name == "shift"));
    let mut out = Vec::with_capacity(parsed.len());
    for token in parsed {
        let Token::Char(ch) = token else {
            out.push(token);
            continue;
        };
        match punctuation_key(ch) {
            Some((shift, key)) => {
                if shift && !has_shift {
                    out.push(Token::Modifier("shift".into()));
                    has_shift = true;
                }
                out.push(key);
            }
            None => out.push(Token::Char(ch)),
        }
    }
    out
}

/// KDOWN and KUP split their payload on '+', so a literal plus is spelled out.
fn held_key_name(token: Token) -> String {
    match token {
        Token::Char('+') => "shift+equal".into(),
        Token::Char(ch) => ch.to_string(),
        Token::Named(name) | Token::Modifier(name) => name,
    }
}

fn cdc(line: impl Into<String>, silent: bool) -> Step {
    Step::Cdc {
        line: line.into(),
        silent,
    }
}

fn sync_after_type(out: &mut Vec<Step>) {
    let ends_silent = out
        .iter()
        .rev()
        .find_map(|step| match step {
            Step::Cdc { silent, .. } => Some(*silent),
            Step::Sleep(_) | Step::Paste(_) => None,
        })
        .unwrap_or(false);
    if ends_silent {
        out.push(cdc(TYPE_SYNC, false));
    }
}

fn sleep_secs(seconds: f64) -> Result<Option<Step>, String> {
    if seconds < 0.0 {
        return Err("negative interval".into());
    }
    if seconds == 0.0 {
        return Ok(None);
    }
    Ok(Some(Step::Sleep(Duration::from_secs_f64(seconds))))
}

pub fn plan_key_down(key: &str) -> Result<Step, String> {
    Ok(cdc(
        format!("KDOWN {}", held_key_name(token_of(key)?)),
        false,
    ))
}

pub fn plan_key_up(key: &str) -> Result<Step, String> {
    Ok(cdc(format!("KUP {}", held_key_name(token_of(key)?)), false))
}

pub fn plan_press(key: &str, presses: u32, interval: f64) -> Result<Vec<Step>, String> {
    if presses == 0 {
        return Err("presses must be at least 1".into());
    }
    let mut out = Vec::new();
    for i in 0..presses {
        if i > 0
            && let Some(pause) = sleep_secs(interval)?
        {
            out.push(pause);
        }
        match token_of(key)? {
            Token::Char(ch) => out.push(cdc(format!("TYPE {ch}"), true)),
            Token::Named(name) => out.push(cdc(format!("KEY {name}"), false)),
            Token::Modifier(name) => {
                out.push(cdc(format!("KDOWN {name}"), false));
                out.push(cdc(format!("KUP {name}"), false));
            }
        }
    }
    sync_after_type(&mut out);
    Ok(out)
}

pub fn plan_hotkey(keys: &[String], interval: f64) -> Result<Vec<Step>, String> {
    if keys.is_empty() {
        return Err("empty hotkey".into());
    }
    let parsed: Vec<Token> = keys
        .iter()
        .map(|key| token_of(key))
        .collect::<Result<_, _>>()?;
    let parsed = expand_punctuation(parsed);
    if interval <= 0.0 {
        let non_mod = parsed
            .iter()
            .filter(|token| !matches!(token, Token::Modifier(_)))
            .count();
        if non_mod <= 1 {
            let mut mods = Vec::new();
            let mut key = None;
            for token in &parsed {
                match token {
                    Token::Modifier(name) => mods.push(name.clone()),
                    Token::Named(name) => key = Some(name.clone()),
                    Token::Char(ch) => key = Some(ch.to_string()),
                }
            }
            mods.sort_by_key(|name| match name.as_str() {
                "cmd" => 0,
                "winright" => 1,
                "ctrl" => 2,
                "ctrlright" => 3,
                "alt" => 4,
                "altright" => 5,
                "shift" => 6,
                "shiftright" => 7,
                _ => 8,
            });
            if let Some(key) = key {
                mods.push(key);
                return Ok(vec![cdc(format!("KEY {}", mods.join("+")), false)]);
            }
        }
        let parts: Vec<String> = parsed
            .iter()
            .map(|token| match token {
                Token::Modifier(name) | Token::Named(name) => name.clone(),
                Token::Char(ch) => ch.to_string(),
            })
            .collect();
        return Ok(vec![cdc(format!("HOTKEY {}", parts.join("+")), false)]);
    }
    let mut out = Vec::new();
    for (i, token) in parsed.iter().enumerate() {
        if i > 0
            && let Some(pause) = sleep_secs(interval)?
        {
            out.push(pause);
        }
        let name = match token {
            Token::Modifier(name) | Token::Named(name) => name.clone(),
            Token::Char(ch) => ch.to_string(),
        };
        out.push(cdc(format!("KDOWN {name}"), false));
    }
    for token in parsed.iter().rev() {
        if let Some(pause) = sleep_secs(interval)? {
            out.push(pause);
        }
        let name = match token {
            Token::Modifier(name) | Token::Named(name) => name.clone(),
            Token::Char(ch) => ch.to_string(),
        };
        out.push(cdc(format!("KUP {name}"), false));
    }
    Ok(out)
}

pub fn plan_write(text: &str, interval: f64) -> Result<Vec<Step>, String> {
    if text.is_empty() {
        return Err("empty text".into());
    }
    if interval < 0.0 {
        return Err("negative interval".into());
    }
    let mut out = Vec::new();
    let mut printable = String::new();
    let mut foreign = String::new();
    let flush_printable = |printable: &mut String, out: &mut Vec<Step>| {
        while !printable.is_empty() {
            // Firmware 0.4.0 drops every space between TYPE and the text.
            let spaces = printable.len() - printable.trim_start_matches(' ').len();
            if spaces > 0 {
                out.extend((0..spaces).map(|_| cdc("KEY space", false)));
                printable.drain(..spaces);
                continue;
            }
            let take = printable.len().min(TYPE_CHUNK);
            let chunk: String = printable.chars().take(take).collect();
            let byte_len = chunk.len();
            out.push(cdc(format!("TYPE {chunk}"), true));
            printable.drain(..byte_len);
        }
    };
    let flush_foreign = |foreign: &mut String, out: &mut Vec<Step>| {
        if foreign.is_empty() {
            return;
        }
        // The device must finish the ASCII typed before this paste.
        sync_after_type(out);
        out.push(Step::Paste(std::mem::take(foreign)));
    };
    let mut first = true;
    for ch in text.chars() {
        // A pasted run is one paste, so the interval does not split it.
        if paste_char(ch) && !foreign.is_empty() {
            foreign.push(ch);
            continue;
        }
        if !first && interval > 0.0 {
            flush_printable(&mut printable, &mut out);
            flush_foreign(&mut foreign, &mut out);
            if let Some(pause) = sleep_secs(interval)? {
                out.push(pause);
            }
        }
        first = false;
        match ch {
            '\n' | '\r' => {
                flush_printable(&mut printable, &mut out);
                flush_foreign(&mut foreign, &mut out);
                out.push(cdc("KEY enter", false));
            }
            '\t' => {
                flush_printable(&mut printable, &mut out);
                flush_foreign(&mut foreign, &mut out);
                out.push(cdc("KEY tab", false));
            }
            ' '..='~' => {
                flush_foreign(&mut foreign, &mut out);
                printable.push(ch);
            }
            ch if (ch as u32) < 0x20 || ch == '\u{7F}' => {
                return Err(format!("unsupported control character: {ch:?}"));
            }
            ch => {
                flush_printable(&mut printable, &mut out);
                foreign.push(ch);
            }
        }
    }
    flush_printable(&mut printable, &mut out);
    flush_foreign(&mut foreign, &mut out);
    sync_after_type(&mut out);
    Ok(out)
}

/// A character the US HID keyboard cannot send, and that is safe to paste.
fn paste_char(ch: char) -> bool {
    !matches!(ch, '\n' | '\r' | '\t' | ' '..='~') && !((ch as u32) < 0x20 || ch == '\u{7F}')
}

pub fn chunk_delta(dx: i32, dy: i32) -> Vec<(i32, i32)> {
    let xs = chunk_axis(dx);
    let ys = chunk_axis(dy);
    let n = xs.len().max(ys.len());
    (0..n)
        .map(|i| {
            (
                xs.get(i).copied().unwrap_or(0),
                ys.get(i).copied().unwrap_or(0),
            )
        })
        .filter(|(x, y)| *x != 0 || *y != 0)
        .collect()
}

fn chunk_axis(total: i32) -> Vec<i32> {
    if total == 0 {
        return Vec::new();
    }
    let sign = total.signum();
    let mut left = total.abs();
    let mut out = Vec::new();
    while left > 0 {
        let step = left.min(MAX_AXIS);
        out.push(step * sign);
        left -= step;
    }
    out
}

pub fn ease(name: &str, t: f64) -> Result<f64, String> {
    let t = t.clamp(0.0, 1.0);
    match name {
        "" | "linear" => Ok(t),
        "ease_in_quad" => Ok(t * t),
        "ease_out_quad" => Ok(t * (2.0 - t)),
        "ease_in_out_quad" => {
            if t < 0.5 {
                Ok(2.0 * t * t)
            } else {
                Ok(-1.0 + (4.0 - 2.0 * t) * t)
            }
        }
        "ease_in_bounce" => Ok(ease_in_bounce(t)),
        "ease_in_elastic" => Ok(ease_in_elastic(t)),
        other => Err(format!("unknown tween: {other}")),
    }
}

fn ease_out_bounce(t: f64) -> f64 {
    if t < 1.0 / 2.75 {
        7.5625 * t * t
    } else if t < 2.0 / 2.75 {
        let t = t - 1.5 / 2.75;
        7.5625 * t * t + 0.75
    } else if t < 2.5 / 2.75 {
        let t = t - 2.25 / 2.75;
        7.5625 * t * t + 0.9375
    } else {
        let t = t - 2.625 / 2.75;
        7.5625 * t * t + 0.984375
    }
}

fn ease_in_bounce(t: f64) -> f64 {
    if t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else {
        1.0 - ease_out_bounce(1.0 - t)
    }
}

fn ease_in_elastic(t: f64) -> f64 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let p = 0.3;
    let s = p / 4.0;
    let shifted = t - 1.0;
    -(2.0_f64.powf(10.0 * shifted)) * ((shifted - s) * (2.0 * std::f64::consts::PI) / p).sin()
}

/// HID counts to send so the cursor travels `delta` points at `gain` points per count.
pub fn counts_for(delta: i32, gain: f64) -> i32 {
    if delta == 0 {
        return 0;
    }
    let counts = (delta as f64 / gain).round() as i32;
    if counts == 0 { delta.signum() } else { counts }
}

/// Points per count seen on one axis. Short or blocked moves keep the previous value.
pub fn measured_gain(sent: i32, moved: i32, previous: f64) -> f64 {
    if sent.abs() < 4 || moved == 0 || sent.signum() != moved.signum() {
        return previous;
    }
    (moved as f64 / sent as f64).clamp(0.2, 5.0)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveChunk {
    pub dx: i32,
    pub dy: i32,
    pub sleep_after: Duration,
}

pub fn plan_relative_move(
    dx: i32,
    dy: i32,
    duration: f64,
    tween: &str,
) -> Result<Vec<MoveChunk>, String> {
    if duration < 0.0 {
        return Err("negative duration".into());
    }
    let _ = ease(tween, 0.0)?;
    if dx == 0 && dy == 0 {
        return Ok(Vec::new());
    }
    if duration < 0.1 {
        return Ok(chunk_delta(dx, dy)
            .into_iter()
            .map(|(x, y)| MoveChunk {
                dx: x,
                dy: y,
                sleep_after: Duration::ZERO,
            })
            .collect());
    }
    let samples = ((duration * 60.0).round() as i32).clamp(2, 240);
    let slice = Duration::from_secs_f64(duration / samples as f64);
    let mut prev_x = 0i32;
    let mut prev_y = 0i32;
    let mut out: Vec<MoveChunk> = Vec::new();
    for i in 1..=samples {
        let eased = ease(tween, i as f64 / samples as f64)?;
        let x = if i == samples {
            dx
        } else {
            (dx as f64 * eased).round() as i32
        };
        let y = if i == samples {
            dy
        } else {
            (dy as f64 * eased).round() as i32
        };
        let pieces = chunk_delta(x - prev_x, y - prev_y);
        prev_x = x;
        prev_y = y;
        let n = pieces.len();
        if n == 0 {
            if let Some(last) = out.last_mut() {
                last.sleep_after += slice;
            }
            continue;
        }
        for (j, (cx, cy)) in pieces.into_iter().enumerate() {
            out.push(MoveChunk {
                dx: cx,
                dy: cy,
                sleep_after: if j + 1 == n { slice } else { Duration::ZERO },
            });
        }
    }
    Ok(out)
}

pub fn click_steps(button: &str, clicks: u32, interval: f64) -> Result<Vec<Step>, String> {
    if clicks == 0 {
        return Err("clicks must be at least 1".into());
    }
    let button = button.trim().to_ascii_lowercase();
    if !matches!(button.as_str(), "left" | "right" | "middle") {
        return Err("button must be left, right, or middle".into());
    }
    if interval < 0.0 {
        return Err("negative interval".into());
    }
    let mut out = Vec::new();
    if button == "left" && clicks == 2 && interval == 0.0 {
        out.push(cdc("CLICK double", false));
        return Ok(out);
    }
    for i in 0..clicks {
        if i > 0
            && let Some(pause) = sleep_secs(interval)?
        {
            out.push(pause);
        }
        if button == "middle" {
            out.push(cdc("MDOWN middle", false));
            out.push(cdc("MUP middle", false));
        } else {
            out.push(cdc(format!("CLICK {button}"), false));
        }
    }
    Ok(out)
}

pub fn button_down(button: &str) -> Result<Step, String> {
    let button = normalize_button(button)?;
    Ok(cdc(format!("MDOWN {button}"), false))
}

pub fn button_up(button: &str) -> Result<Step, String> {
    let button = normalize_button(button)?;
    Ok(cdc(format!("MUP {button}"), false))
}

fn normalize_button(button: &str) -> Result<String, String> {
    let button = button.trim().to_ascii_lowercase();
    if matches!(button.as_str(), "left" | "right" | "middle") {
        Ok(button)
    } else {
        Err("button must be left, right, or middle".into())
    }
}

/// Flip HID notches when the OS turns a positive wheel into a downward scroll.
pub fn signed_scroll(steps: i32, inverted: bool) -> i32 {
    if inverted {
        steps.saturating_neg()
    } else {
        steps
    }
}

pub fn scroll_steps(horizontal: bool, steps: i32) -> Vec<Step> {
    if steps == 0 {
        return Vec::new();
    }
    let verb = if horizontal { "HSCROLL" } else { "SCROLL" };
    let sign = steps.signum();
    let mut left = steps.abs();
    let mut out = Vec::new();
    while left > 0 {
        let chunk = left.min(SCROLL_CHUNK);
        out.push(cdc(format!("{verb} {}", chunk * sign), false));
        left -= chunk;
    }
    out
}

#[cfg(test)]
pub fn lines_of(steps: &[Step]) -> Vec<String> {
    steps
        .iter()
        .filter_map(|step| match step {
            Step::Cdc { line, .. } => Some(line.clone()),
            Step::Sleep(_) | Step::Paste(_) => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_compare_rejects_a_different_secret() {
        assert!(tokens_equal("abc", "abc"));
        assert!(!tokens_equal("abc", "abd"));
        assert!(!tokens_equal("abc", "abcd"));
    }

    #[test]
    fn status_line_carries_uptime_and_firmware() {
        let parsed = parse_status("OK uptime_ms=1500 fw=0.4.0").unwrap();
        assert_eq!(parsed.uptime_ms, 1500);
        assert_eq!(parsed.firmware, "0.4.0");
        assert!(parse_status("MOVE done").is_none());
    }

    #[test]
    fn on_screen_matches_pyautogui_bounds() {
        assert!(on_screen(0, 0, 1920, 1080));
        assert!(on_screen(1919, 1079, 1920, 1080));
        assert!(!on_screen(1920, 1080, 1920, 1080));
        assert!(!on_screen(-1, 0, 1920, 1080));
    }

    #[test]
    fn aliases_become_firmware_lines() {
        assert_eq!(
            lines_of(&plan_press("escape", 1, 0.0).unwrap()),
            vec!["KEY esc".to_string()]
        );
        assert_eq!(
            lines_of(&plan_press("enter", 2, 0.0).unwrap()),
            vec!["KEY enter".to_string(), "KEY enter".to_string()]
        );
        assert_eq!(
            lines_of(&plan_hotkey(&["ctrl".into(), "c".into()], 0.0).unwrap()),
            vec!["KEY ctrl+c".to_string()]
        );
        assert_eq!(
            lines_of(&plan_hotkey(&["command".into(), "shift".into(), "a".into()], 0.0).unwrap()),
            vec!["KEY cmd+shift+a".to_string()]
        );
        assert!(plan_press("capslock", 1, 0.0).is_err());
        assert!(plan_press("f13", 1, 0.0).is_err());
    }

    #[test]
    fn hotkey_with_interval_presses_then_releases_in_reverse() {
        assert_eq!(
            lines_of(&plan_hotkey(&["ctrl".into(), "c".into()], 0.05).unwrap()),
            vec![
                "KDOWN ctrl".to_string(),
                "KDOWN c".to_string(),
                "KUP c".to_string(),
                "KUP ctrl".to_string(),
            ]
        );
    }

    #[test]
    fn write_chunks_ascii_and_turns_newlines_into_enter() {
        assert_eq!(
            lines_of(&plan_write("Hi", 0.0).unwrap()),
            vec!["TYPE Hi".to_string(), TYPE_SYNC.to_string()]
        );
        assert_eq!(
            lines_of(&plan_write("a\nb", 0.0).unwrap()),
            vec![
                "TYPE a".to_string(),
                "KEY enter".to_string(),
                "TYPE b".to_string(),
                TYPE_SYNC.to_string()
            ]
        );
        assert_eq!(
            lines_of(&plan_write("a\n", 0.0).unwrap()),
            vec!["TYPE a".to_string(), "KEY enter".to_string()]
        );
        let long = "x".repeat(90);
        let lines = lines_of(&plan_write(&long, 0.0).unwrap());
        assert_eq!(lines.len(), 3);
        assert!(lines[0].len() <= 5 + TYPE_CHUNK);
    }

    #[test]
    fn write_never_starts_a_type_payload_with_a_space() {
        assert_eq!(
            lines_of(&plan_write(" ", 0.0).unwrap()),
            vec!["KEY space".to_string()]
        );
        assert_eq!(
            lines_of(&plan_write("a\n  b", 0.0).unwrap()),
            vec![
                "TYPE a".to_string(),
                "KEY enter".to_string(),
                "KEY space".to_string(),
                "KEY space".to_string(),
                "TYPE b".to_string(),
                TYPE_SYNC.to_string()
            ]
        );
        assert_eq!(
            lines_of(&plan_write("a b", 0.05).unwrap()),
            vec![
                "TYPE a".to_string(),
                "KEY space".to_string(),
                "TYPE b".to_string(),
                TYPE_SYNC.to_string()
            ]
        );
        let text = format!("{} tail", "x".repeat(TYPE_CHUNK));
        let lines = lines_of(&plan_write(&text, 0.0).unwrap());
        assert_eq!(lines[1], "KEY space");
        assert_eq!(lines[2], "TYPE tail");
        let typed: String = lines
            .iter()
            .filter_map(|line| line.strip_prefix("TYPE "))
            .collect::<String>();
        assert_eq!(typed.len() + 1, text.len());
        assert!(plan_write("\u{1}", 0.0).is_err());
        assert!(plan_write("\u{7F}", 0.0).is_err());
    }

    #[test]
    fn write_pastes_non_ascii_as_one_run() {
        let steps = plan_write("hi 👋", 0.0).unwrap();
        assert!(
            steps
                .iter()
                .any(|step| matches!(step, Step::Paste(text) if text == "👋"))
        );
        assert!(
            plan_write("hi", 0.0)
                .unwrap()
                .iter()
                .all(|step| !matches!(step, Step::Paste(_)))
        );
        let family = "👨‍👩‍👧‍👦";
        let pasted: Vec<_> = plan_write(family, 0.2)
            .unwrap()
            .into_iter()
            .filter_map(|step| match step {
                Step::Paste(text) => Some(text),
                _ => None,
            })
            .collect();
        assert_eq!(pasted, vec![family.to_string()]);
        assert!(matches!(
            plan_write("é", 0.0).unwrap().first(),
            Some(Step::Paste(text)) if text == "é"
        ));
    }

    #[test]
    fn hotkey_spells_punctuation_with_firmware_key_names() {
        let chord = |keys: &[&str]| {
            lines_of(
                &plan_hotkey(
                    &keys
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect::<Vec<_>>(),
                    0.0,
                )
                .unwrap(),
            )
        };
        assert_eq!(chord(&["command", ","]), vec!["KEY cmd+comma".to_string()]);
        assert_eq!(
            chord(&["command", "+"]),
            vec!["KEY cmd+shift+equal".to_string()]
        );
        assert_eq!(
            chord(&["command", "?"]),
            vec!["KEY cmd+shift+slash".to_string()]
        );
        assert_eq!(
            chord(&["command", "shift", "/"]),
            vec!["KEY cmd+shift+slash".to_string()]
        );
        assert_eq!(
            chord(&["command", "shift", "?"]),
            vec!["KEY cmd+shift+slash".to_string()]
        );
        assert_eq!(chord(&["ctrl", "!"]), vec!["KEY ctrl+shift+1".to_string()]);
        assert_eq!(chord(&["command", "t"]), vec!["KEY cmd+t".to_string()]);
        assert_eq!(
            lines_of(&plan_hotkey(&["ctrl".into(), "=".into()], 0.05).unwrap()),
            vec![
                "KDOWN ctrl".to_string(),
                "KDOWN equal".to_string(),
                "KUP equal".to_string(),
                "KUP ctrl".to_string(),
            ]
        );
    }

    #[test]
    fn held_plus_is_spelled_out_and_press_waits_for_typing() {
        assert_eq!(
            lines_of(&[plan_key_down("+").unwrap(), plan_key_up("+").unwrap()]),
            vec![
                "KDOWN shift+equal".to_string(),
                "KUP shift+equal".to_string()
            ]
        );
        assert_eq!(
            lines_of(&plan_press("a", 1, 0.0).unwrap()),
            vec!["TYPE a".to_string(), TYPE_SYNC.to_string()]
        );
        assert_eq!(
            lines_of(&plan_press("space", 1, 0.0).unwrap()),
            vec!["KEY space".to_string()]
        );
    }

    #[test]
    fn long_moves_split_at_2000_and_tweens_land_on_the_target() {
        let instant = plan_relative_move(2500, -100, 0.0, "linear").unwrap();
        assert_eq!(instant[0].dx, 2000);
        assert_eq!(instant[1].dx, 500);
        assert!(instant.iter().all(|chunk| chunk.dx.abs() <= MAX_AXIS));
        assert!(instant.iter().all(|chunk| chunk.sleep_after.is_zero()));

        let eased = plan_relative_move(1000, 0, 1.0, "ease_in_quad").unwrap();
        let sum: i32 = eased.iter().map(|chunk| chunk.dx).sum();
        assert_eq!(sum, 1000);
        assert!(
            eased
                .iter()
                .all(|chunk| chunk.dx.abs() <= MAX_AXIS && chunk.dx >= 0)
        );
        assert!(eased.first().unwrap().dx < eased.last().unwrap().dx);
        assert!(plan_relative_move(10, 0, 1.0, "ease_in_elastic").is_ok());
        assert!(plan_relative_move(10, 0, 1.0, "nope").is_err());
        assert!((ease("ease_in_bounce", 1.0).unwrap() - 1.0).abs() < 1e-9);
        assert_eq!(ease("ease_in_elastic", 0.0).unwrap(), 0.0);
        assert_eq!(ease("ease_in_elastic", 1.0).unwrap(), 1.0);
    }

    #[test]
    fn pointer_gain_converts_points_to_counts() {
        assert_eq!(counts_for(145, 1.45), 100);
        assert_eq!(counts_for(-145, 1.45), -100);
        assert_eq!(counts_for(1, 1.45), 1);
        assert_eq!(counts_for(0, 1.45), 0);
        assert!((measured_gain(100, 145, 1.0) - 1.45).abs() < 1e-9);
        assert_eq!(measured_gain(2, 3, 1.0), 1.0);
        assert_eq!(measured_gain(100, 0, 1.3), 1.3);
        assert_eq!(measured_gain(100, -20, 1.3), 1.3);
        assert_eq!(measured_gain(10, 900, 1.0), 5.0);
    }

    #[test]
    fn clicks_and_scroll_map_onto_cdc() {
        assert_eq!(
            lines_of(&click_steps("left", 2, 0.0).unwrap()),
            vec!["CLICK double".to_string()]
        );
        assert_eq!(
            lines_of(&click_steps("middle", 1, 0.0).unwrap()),
            vec!["MDOWN middle".to_string(), "MUP middle".to_string()]
        );
        assert_eq!(
            lines_of(&scroll_steps(false, 50)),
            vec!["SCROLL 40".to_string(), "SCROLL 10".to_string()]
        );
        assert_eq!(signed_scroll(4, false), 4);
        assert_eq!(
            lines_of(&scroll_steps(false, signed_scroll(4, true))),
            vec!["SCROLL -4".to_string()]
        );
        assert_eq!(
            lines_of(&scroll_steps(true, signed_scroll(-2, true))),
            vec!["HSCROLL 2".to_string()]
        );
        assert_eq!(signed_scroll(0, true), 0);
    }
}
