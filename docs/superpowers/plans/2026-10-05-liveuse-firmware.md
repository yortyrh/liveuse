# Live Use Firmware Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The XIAO SAMD21 enumerates as a USB mouse and a USB keyboard, and CDC accepts `TYPE` and `KEY`.

Firmware 0.4.0, specified after Tasks 1–3, adds `DOWN`, `UP`, `HSCROLL`, and `HOTKEY` so the host can implement the PyAutoGUI mouse and keyboard contract in the design spec. Tasks 1–3 stay as written.

**Architecture:** A second `Adafruit_USBD_HID` uses the boot keyboard descriptor. Pure C++ in `keymap.cpp` (no Arduino) maps US ANSI and chords. `TYPE` types the whole payload and prints nothing on success. `KEY` prints `KEY <canonical> done`. The boot demo is removed.

**Tech Stack:** PlatformIO, Seeed SAMD21 core, Adafruit TinyUSB, C++ dialect of that core for the sketch, C++17 for `keymap.cpp`.

## Global Constraints

- Firmware version string is `0.3.0` in `firmware/VERSION` and in `STATUS` as `OK uptime_ms=<ms> fw=0.3.0`.
- Two HID interfaces plus CDC on the same Type-C port. Mouse descriptor stays the current relative one. Keyboard descriptor is the boot keyboard (modifier, reserved, six keycodes) and each report uses one keycode.
- CDC is 115200 8N1, LF-terminated, CR ignored. Line buffer stays 96 bytes including the NUL.
- `TYPE` payload is ASCII `0x20`–`0x7E` only. Empty or any other byte is `ERR TYPE bad` and presses nothing.
- `KEY` takes optional left modifiers and exactly one key, joined by `+`. Canonical order is `cmd`, `ctrl`, `alt`, `shift`, then the key. Letters in a chord are case-insensitive and come out lowercase. `KEY A` is the `a` key with no Shift.
- Modifier bits: ctrl `0x01`, shift `0x02`, alt `0x04`, cmd `0x08`. Aliases: `command` and `meta` are `cmd`; `control` is `ctrl`; `opt` and `option` are `alt`; `return` is `enter`; `escape` is `esc`.
- Reject, with no HID report: empty chord, modifiers only, two keys, repeated modifier, unknown name. Response `ERR KEY bad`.
- `TYPE` success prints nothing. `KEY` success is exactly `KEY <canonical> done`. `ERR TYPE bad` and `ERR KEY bad` are the only keyboard errors.
- Do not run the circle/zigzag demo from `loop`. The `DEMO` command stays.
- USB product name does not change.
- `keymap.cpp` has no Arduino includes. The host plan links this same file.

---

### Task 1: US ANSI map and chord parser

**Files:**
- Create: `firmware/include/keymap.h`
- Create: `firmware/src/keymap.cpp`
- Test: `firmware/test/keymap_test.cpp`

**Interfaces:**
- Consumes: nothing
- Produces: `tap_ascii(unsigned char) -> Chord`, `parse_chord(const char*) -> Chord`, `key_error_text(KeyError) -> const char*`. `Chord` holds `KeyError error`, `KeyTap tap` (`modifier`, `usage`), and `char canonical[64]`.

- [ ] **Step 1: Write the failing test**

Create `firmware/test/keymap_test.cpp`:

```cpp
#include "keymap.h"

#include <cstdio>
#include <cstring>

static int g_fails = 0;

static void check(bool cond, const char *what) {
  if (!cond) {
    std::fprintf(stderr, "FAIL %s\n", what);
    ++g_fails;
  }
}

int main() {
  const Chord letter = tap_ascii('a');
  check(letter.error == KeyError::Ok && letter.tap.modifier == 0 && letter.tap.usage == 0x04, "a");
  const Chord upper = tap_ascii('A');
  check(upper.error == KeyError::Ok && upper.tap.modifier == 0x02 && upper.tap.usage == 0x04, "A");
  const Chord digit = tap_ascii('2');
  check(digit.error == KeyError::Ok && digit.tap.modifier == 0 && digit.tap.usage == 0x1F, "2");
  const Chord at = tap_ascii('@');
  check(at.error == KeyError::Ok && at.tap.modifier == 0x02 && at.tap.usage == 0x1F, "@");
  check(tap_ascii('\n').error == KeyError::BadChar, "newline");

  const Chord space = parse_chord("cmd+space");
  check(space.error == KeyError::Ok && std::strcmp(space.canonical, "cmd+space") == 0, "cmd+space");
  check(space.tap.modifier == 0x08 && space.tap.usage == 0x2C, "cmd+space bits");

  const Chord shot = parse_chord("shift+cmd+4");
  check(shot.error == KeyError::Ok && std::strcmp(shot.canonical, "cmd+shift+4") == 0, "canonical order");
  check(shot.tap.modifier == (0x08 | 0x02) && shot.tap.usage == 0x21, "cmd+shift+4 bits");

  const Chord aye = parse_chord("A");
  check(aye.error == KeyError::Ok && std::strcmp(aye.canonical, "a") == 0 && aye.tap.modifier == 0, "KEY A");

  check(parse_chord("").error == KeyError::Empty, "empty");
  check(parse_chord("cmd").error == KeyError::BadChord, "modifier only");
  check(parse_chord("a+b").error == KeyError::BadChord, "two keys");
  check(parse_chord("cmd+cmd+a").error == KeyError::BadChord, "repeat modifier");
  check(parse_chord("foo").error == KeyError::BadChord, "unknown");
  return g_fails == 0 ? 0 : 1;
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `c++ -std=c++17 -I firmware/include firmware/test/keymap_test.cpp firmware/src/keymap.cpp -o /tmp/keymap_test`

Expected: compile error, `keymap.h` or `keymap.cpp` missing.

- [ ] **Step 3: Write the parser**

Create `firmware/src/keymap.cpp`:

```cpp
#include "keymap.h"

#include <cctype>
#include <cstring>

namespace {

struct Named {
  const char *name;
  uint8_t usage;
};

const Named kNamed[] = {
    {"enter", 0x28},       {"return", 0x28}, {"esc", 0x29},
    {"escape", 0x29},      {"backspace", 0x2A}, {"tab", 0x2B},
    {"space", 0x2C},       {"minus", 0x2D},  {"equal", 0x2E},
    {"leftbracket", 0x2F}, {"rightbracket", 0x30}, {"backslash", 0x31},
    {"semicolon", 0x33},   {"quote", 0x34},  {"grave", 0x35},
    {"comma", 0x36},       {"dot", 0x37},    {"slash", 0x38},
    {"delete", 0x4C},      {"right", 0x4F},  {"left", 0x50},
    {"down", 0x51},        {"up", 0x52},
};

bool named_usage(const char *token, uint8_t *usage) {
  if (token[0] == 'f' && token[1] >= '1' && token[1] <= '9' && token[2] == '\0') {
    *usage = static_cast<uint8_t>(0x3A + (token[1] - '1'));
    return true;
  }
  if (std::strcmp(token, "f10") == 0) { *usage = 0x43; return true; }
  if (std::strcmp(token, "f11") == 0) { *usage = 0x44; return true; }
  if (std::strcmp(token, "f12") == 0) { *usage = 0x45; return true; }
  for (const Named &row : kNamed) {
    if (std::strcmp(token, row.name) == 0) {
      *usage = row.usage;
      return true;
    }
  }
  return false;
}

Chord fail(KeyError err) {
  Chord out{};
  out.error = err;
  return out;
}

}  // namespace

const char *key_error_text(KeyError err) {
  switch (err) {
    case KeyError::Ok: return "ok";
    case KeyError::Empty: return "empty";
    case KeyError::BadChar: return "bad char";
    case KeyError::BadChord: return "bad chord";
  }
  return "bad chord";
}

Chord tap_ascii(unsigned char c) {
  if (c >= 'a' && c <= 'z') {
    Chord out{};
    out.tap = {0, static_cast<uint8_t>(0x04 + (c - 'a'))};
    return out;
  }
  if (c >= 'A' && c <= 'Z') {
    Chord out{};
    out.tap = {0x02, static_cast<uint8_t>(0x04 + (c - 'A'))};
    return out;
  }
  if (c >= '1' && c <= '9') {
    Chord out{};
    out.tap = {0, static_cast<uint8_t>(0x1E + (c - '1'))};
    return out;
  }
  Chord out{};
  switch (c) {
    case ' ': out.tap = {0, 0x2C}; break;
    case '!': out.tap = {0x02, 0x1E}; break;
    case '"': out.tap = {0x02, 0x34}; break;
    case '#': out.tap = {0x02, 0x20}; break;
    case '$': out.tap = {0x02, 0x21}; break;
    case '%': out.tap = {0x02, 0x22}; break;
    case '&': out.tap = {0x02, 0x24}; break;
    case '\'': out.tap = {0, 0x34}; break;
    case '(': out.tap = {0x02, 0x26}; break;
    case ')': out.tap = {0x02, 0x27}; break;
    case '*': out.tap = {0x02, 0x25}; break;
    case '+': out.tap = {0x02, 0x2E}; break;
    case ',': out.tap = {0, 0x36}; break;
    case '-': out.tap = {0, 0x2D}; break;
    case '.': out.tap = {0, 0x37}; break;
    case '/': out.tap = {0, 0x38}; break;
    case '0': out.tap = {0, 0x27}; break;
    case ':': out.tap = {0x02, 0x33}; break;
    case ';': out.tap = {0, 0x33}; break;
    case '<': out.tap = {0x02, 0x36}; break;
    case '=': out.tap = {0, 0x2E}; break;
    case '>': out.tap = {0x02, 0x37}; break;
    case '?': out.tap = {0x02, 0x38}; break;
    case '@': out.tap = {0x02, 0x1F}; break;
    case '[': out.tap = {0, 0x2F}; break;
    case '\\': out.tap = {0, 0x31}; break;
    case ']': out.tap = {0, 0x30}; break;
    case '^': out.tap = {0x02, 0x23}; break;
    case '_': out.tap = {0x02, 0x2D}; break;
    case '`': out.tap = {0, 0x35}; break;
    case '{': out.tap = {0x02, 0x2F}; break;
    case '|': out.tap = {0x02, 0x31}; break;
    case '}': out.tap = {0x02, 0x30}; break;
    case '~': out.tap = {0x02, 0x35}; break;
    default: return fail(KeyError::BadChar);
  }
  return out;
}

Chord parse_chord(const char *text) {
  if (text == nullptr || text[0] == '\0') {
    return fail(KeyError::Empty);
  }
  uint8_t modifier = 0;
  uint8_t seen_mod_bits = 0;
  uint8_t usage = 0;
  bool have_key = false;
  char key_name[32] = {};
  const char *cursor = text;
  while (*cursor) {
    char token[32] = {};
    size_t n = 0;
    while (*cursor && *cursor != '+') {
      if (n + 1 >= sizeof(token)) {
        return fail(KeyError::BadChord);
      }
      token[n++] = static_cast<char>(std::tolower(static_cast<unsigned char>(*cursor)));
      ++cursor;
    }
    token[n] = '\0';
    if (*cursor == '+') {
      ++cursor;
    }
    if (n == 0) {
      return fail(KeyError::BadChord);
    }
    uint8_t bit = 0;
    const char *canon = nullptr;
    if (std::strcmp(token, "cmd") == 0 || std::strcmp(token, "command") == 0 ||
        std::strcmp(token, "meta") == 0) {
      bit = 0x08;
      canon = "cmd";
    } else if (std::strcmp(token, "ctrl") == 0 || std::strcmp(token, "control") == 0) {
      bit = 0x01;
      canon = "ctrl";
    } else if (std::strcmp(token, "alt") == 0 || std::strcmp(token, "opt") == 0 ||
               std::strcmp(token, "option") == 0) {
      bit = 0x04;
      canon = "alt";
    } else if (std::strcmp(token, "shift") == 0) {
      bit = 0x02;
      canon = "shift";
    }
    if (bit != 0) {
      if ((seen_mod_bits & bit) != 0) {
        return fail(KeyError::BadChord);
      }
      seen_mod_bits = static_cast<uint8_t>(seen_mod_bits | bit);
      modifier = static_cast<uint8_t>(modifier | bit);
      (void)canon;
      continue;
    }
    if (have_key) {
      return fail(KeyError::BadChord);
    }
    uint8_t named = 0;
    if (n == 1 && token[0] >= 'a' && token[0] <= 'z') {
      usage = static_cast<uint8_t>(0x04 + (token[0] - 'a'));
    } else if (n == 1 && token[0] >= '1' && token[0] <= '9') {
      usage = static_cast<uint8_t>(0x1E + (token[0] - '1'));
    } else if (n == 1 && token[0] == '0') {
      usage = 0x27;
    } else if (!named_usage(token, &named)) {
      return fail(KeyError::BadChord);
    } else {
      usage = named;
    }
    std::memcpy(key_name, token, n + 1);
    if (std::strcmp(token, "return") == 0) {
      std::strcpy(key_name, "enter");
    } else if (std::strcmp(token, "escape") == 0) {
      std::strcpy(key_name, "esc");
    }
    have_key = true;
  }
  if (!have_key) {
    return fail(KeyError::BadChord);
  }
  Chord out{};
  out.tap = {modifier, usage};
  char *dst = out.canonical;
  const char *parts[] = {"cmd", "ctrl", "alt", "shift"};
  const uint8_t bits[] = {0x08, 0x01, 0x04, 0x02};
  for (int i = 0; i < 4; ++i) {
    if ((modifier & bits[i]) == 0) {
      continue;
    }
    if (dst != out.canonical) {
      *dst++ = '+';
    }
    std::strcpy(dst, parts[i]);
    dst += std::strlen(parts[i]);
  }
  if (dst != out.canonical) {
    *dst++ = '+';
  }
  std::strcpy(dst, key_name);
  return out;
}
```

Create `firmware/include/keymap.h`:

```cpp
#pragma once

#include <stdint.h>

struct KeyTap {
  uint8_t modifier;
  uint8_t usage;
};

enum class KeyError : int {
  Ok = 0,
  Empty,
  BadChar,
  BadChord,
};

struct Chord {
  KeyError error;
  KeyTap tap;
  char canonical[64];
};

Chord tap_ascii(unsigned char c);
Chord parse_chord(const char *text);
const char *key_error_text(KeyError err);
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `c++ -std=c++17 -I firmware/include firmware/test/keymap_test.cpp firmware/src/keymap.cpp -o /tmp/keymap_test && /tmp/keymap_test`

Expected: exit 0, no `FAIL` lines.

- [ ] **Step 5: Commit**

```bash
git add firmware/include/keymap.h firmware/src/keymap.cpp firmware/test/keymap_test.cpp
git commit -m "$(cat <<'EOF'
feat: add US ANSI keymap and KEY chord parser

Host and firmware share one table so @ and cmd+shift+4 cannot drift.

EOF
)"
```

---

### Task 2: Keyboard HID reports and CDC lines

**Files:**
- Create: `firmware/include/keyboard.h`
- Create: `firmware/src/keyboard.cpp`
- Modify: `firmware/src/main.cpp`
- Modify: `firmware/VERSION`
- Modify: `firmware/platformio.ini` (comment only: mouse + keyboard + CDC)

**Interfaces:**
- Consumes: `tap_ascii`, `parse_chord`, `key_error_text` from Task 1. `Adafruit_USBD_HID::keyboardReport(uint8_t report_id, uint8_t modifier, uint8_t keycode[6])` and `ready()`.
- Produces: `keyboardType(Adafruit_USBD_HID&, const char* payload)`, `keyboardChord(Adafruit_USBD_HID&, const char* chord)`. `keyboardType` prints only on error. `keyboardChord` prints the CDC result. `main.cpp` does not print a `TYPE` success line.

- [ ] **Step 1: Write the HID helpers and wire the commands**

`firmware/include/keyboard.h`:

```cpp
#pragma once

#include <Adafruit_TinyUSB.h>

void keyboardType(Adafruit_USBD_HID &hid, const char *payload);
void keyboardChord(Adafruit_USBD_HID &hid, const char *chord);
```

`firmware/src/keyboard.cpp` taps one key then releases it:

```cpp
static bool tap(Adafruit_USBD_HID &hid, uint8_t modifier, uint8_t usage) {
  uint8_t keys[6] = {usage, 0, 0, 0, 0, 0};
  if (!hid.ready()) {
    return false;
  }
  if (!hid.keyboardReport(0, modifier, keys)) {
    return false;
  }
  delay(12);
  uint8_t up[6] = {0, 0, 0, 0, 0, 0};
  if (!hid.keyboardReport(0, 0, up)) {
    return false;
  }
  delay(12);
  return true;
}
```

`keyboardType`: if `payload` is null or empty, print `ERR TYPE bad` and return before any report. Scan every byte with `tap_ascii` first. On any `BadChar`, print `ERR TYPE bad` and return before any HID report. Then tap every character. On a failed `tap`, print `ERR TYPE bad` and return. On success, print nothing.

`keyboardChord`: `parse_chord`. On error, print `ERR KEY bad` and do not call `tap`. On success, `tap` then print `KEY `, the canonical text, and ` done`.

In `setup()`, after the mouse HID `begin()`:

```cpp
static uint8_t const desc_keyboard[] = {TUD_HID_REPORT_DESC_KEYBOARD()};
static Adafruit_USBD_HID g_usbKeyboard;

g_usbKeyboard.setPollInterval(2);
g_usbKeyboard.setReportDescriptor(desc_keyboard, sizeof(desc_keyboard));
g_usbKeyboard.begin();
```

Declare `g_usbKeyboard` at file scope next to `g_usbHid`.

`STATUS` prints `OK uptime_ms=<millis> fw=0.3.0` with a single space before `fw`.

`HELP` adds two lines: `TYPE <ascii>` and `KEY <chord>`.

`handleLine` for `TYPE`: the payload is everything after the first run of spaces that follows the verb. Do not trim that payload. Call `keyboardType(g_usbKeyboard, rest)`.

`handleLine` for `KEY`: call `keyboardChord(g_usbKeyboard, rest)`.

Delete the boot-demo block in `loop` (`bootDemoDone`, `runDemoBoth()` on a timer). Keep `runDemoBoth` for the `DEMO` command.

Banner text: `USB: composite HID mouse + keyboard + CDC serial`.

`firmware/VERSION` contents: `0.3.0` plus a trailing newline.

- [ ] **Step 2: Build the firmware**

Run: `pio run -e seeed_xiao` from `firmware/`

Expected: exit 0. The log shows success for `seeed_xiao`.

- [ ] **Step 3: Commit**

```bash
git add firmware/include/keyboard.h firmware/src/keyboard.cpp firmware/src/main.cpp firmware/VERSION firmware/platformio.ini
git commit -m "$(cat <<'EOF'
feat: add keyboard HID plus TYPE and KEY on CDC

The dongle no longer moves the pointer by itself when the port opens.

EOF
)"
```

---

### Task 3: Manual check on the board

**Files:**
- Modify: none

**Interfaces:**
- Consumes: the CDC lines from Task 2

- [ ] **Step 1: Flash**

Run: `pio run -e seeed_xiao -t upload` from `firmware/`

Expected: exit 0.

- [ ] **Step 2: Open the CDC port at 115200 and send these lines**

| Send | Expect |
| --- | --- |
| `STATUS` | one line `OK uptime_ms=<digits> fw=0.3.0` |
| `HELP` | the list includes `TYPE <ascii>` and `KEY <chord>` |
| `TYPE ` (nothing after the space) | `ERR TYPE bad` and no keystrokes |
| `TYPE ab` | no `TYPE done` line, and the focused field receives `ab` |
| `KEY A` | `KEY a done` (lowercase a, no Shift) |
| `KEY cmd+space` | `KEY cmd+space done` |
| `KEY cmd` | `ERR KEY bad` |
| opening the port | no `boot DEMO` line and the pointer stays still |

This check is not part of CI.
