# LiveUse firmware

Version **0.5.3**. PlatformIO sketch for the **Seeed XIAO SAMD21** (`ATSAMD21G18A`).

One USB Type-C cable enumerates three interfaces:

- HID mouse, relative reports, named `LiveUse Mouse 0.5.3`
- HID keyboard, boot keyboard, named `LiveUse Keyboard 0.5.3`
- CDC serial, the command plane

The device itself is manufacturer `LiveUse`, product `LiveUse 0.5.3`. Those strings use the same version as `fw=`. The SAM-BA bootloader keeps the board name `Seeeduino XIAO`.

A single Arduino `loop()` reads one CDC line and runs it. There is no RTOS. TinyUSB is pumped by the core `yield()` inside `delay()` and `Serial`. The CDC banner is `=== LiveUse ===`.

`STATUS` answers `OK uptime_ms=<ms> fw=0.5.3`.

## Flash

Plug in the XIAO and run:

```sh
cd firmware
scripts/flash
```

It finds the board on its own. If another program has the serial port open, it stops and names that program. The upload itself is `pio run -t upload`. The `seeed_xiao` board manifest has `upload.use_1200bps_touch`, so pio opens that port at 1200 baud and waits for the bootloader. `scripts/upload_bossac.py` waits for that USB port to settle and retries the page write, which otherwise dies mid-transfer on macOS.

The USB product id shows which mode the board is in. The running sketch reports product `LiveUse 0.5.3`; the bootloader still reports `Seeeduino XIAO`:

| `idProduct` | Meaning |
|-------------|---------|
| `32815` (`0x802F`) | The sketch is running |
| `47` (`0x002F`) | SAM-BA bootloader |

After a successful flash the board resets and `idProduct` returns to `32815`.

A serial monitor is 115200 baud:

```sh
pio device monitor -b 115200
```

## CDC commands

Lines end in LF or CRLF. The command word is case-insensitive. Motion and key actions block the loop until they finish; the next line is read after that. An unknown line is echoed back.

| Command | Action |
|---------|--------|
| `HELP` or `?` | List commands |
| `STATUS` | Uptime and firmware version |
| `ECHO <text>` | Echo the text |
| `MOVE <dx> <dy>` | Relative glide, small steps. Each axis is clamped to ±2000 |
| `CLICK left\|right\|double` | Click. `double` is two left clicks |
| `MDOWN left\|right\|middle` | Hold a button. Later moves keep it down |
| `MUP left\|right\|middle` | Release a button |
| `SCROLL <steps>` | Vertical wheel. Positive is up, negative is down |
| `HSCROLL <steps>` | Horizontal wheel. Positive is right, negative is left |
| `TYPE <ascii>` | Type printable ASCII (`0x20`–`0x7E`). One space separates the verb; any further spaces are typed. Success prints nothing |
| `KEY <chord>` | Tap one key, optional modifiers joined by `+` |
| `KDOWN <key>` | Hold a key |
| `KUP <key>` | Release a key |
| `HOTKEY <k1+k2>` | Press the steps in order, then release them in reverse |

`KEY` takes modifiers plus exactly one key, for example `ctrl+c` or `cmd+shift+a`. Letters are case-insensitive and name the key, not Shift: `KEY A` taps `a`. Canonical order is `cmd`, `ctrl`, `alt`, `shift`, then the key. Aliases include `command`/`meta` for `cmd`, `control` for `ctrl`, `opt`/`option` for `alt`, `return` for `enter`, and `escape` for `esc`. A bad chord answers `ERR KEY bad`.

## Layout

```
scripts/flash             — compile, then upload through pio
scripts/flash.py          — finds the board and checks that its port is free
scripts/hid_count.py      — compile TinyUSB with two HID interfaces, and retry bossac
scripts/upload_bossac.py  — wait for the bootloader USB, then retry a failed page write
src/main.cpp              — USB init, CDC parse, command dispatch
src/motion.cpp            — relative mouse glide, click, and wheel
src/keyboard.cpp          — TYPE, KEY, KDOWN, KUP, HOTKEY
src/keymap.cpp            — US ANSI names and chords
src/hid_reports.cpp       — per-interface HID report descriptors
include/
```

## Build notes

- `USE_TINYUSB` turns on Adafruit TinyUSB in the Seeed SAMD core.
- The sketch adds a second HID interface after the core opens USB, then re-enumerates so CDC, mouse, and keyboard configure together.
- `include/hid_count.h` forces `CFG_TUD_HID` to 2 for the TinyUSB library.
