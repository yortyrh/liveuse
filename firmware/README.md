# Live Use firmware

Version **0.4.0**. PlatformIO sketch for the **Seeed XIAO SAMD21** (`ATSAMD21G18A`).

One USB Type-C cable enumerates three interfaces:

- HID mouse, relative reports
- HID keyboard, boot keyboard
- CDC serial, the command plane

A single Arduino `loop()` reads one CDC line and runs it. There is no RTOS. TinyUSB is pumped by the core `yield()` inside `delay()` and `Serial`. The CDC banner is `=== Live Use ===`.

`STATUS` answers `OK uptime_ms=<ms> fw=0.4.0`.

## Flash

Close anything that holds the CDC port first. `liveuse` and a serial monitor block the reset.

```sh
cd firmware
./upload.sh
```

`upload.sh` compiles the `seeed_xiao` environment and uploads. `scripts/settle_upload.py` chooses the XIAO port and skips Bluetooth. It reads the USB product id of `Seeeduino XIAO`:

| `idProduct` | Meaning |
|-------------|---------|
| `32815` (`0x802F`) | The sketch is running |
| `47` (`0x002F`) | SAM-BA bootloader |

When the sketch is running, the script opens that CDC port at **1200 baud** and closes it. TinyUSB reboots into the bootloader when DTR drops while the line rate is already 1200. If the product id is already `47`, it flashes without that touch. `scripts/upload_bossac.py` retries the page write after the bootloader USB settles.

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
| `TYPE <ascii>` | Type printable ASCII (`0x20`–`0x7E`). Success prints nothing |
| `KEY <chord>` | Tap one key, optional modifiers joined by `+` |
| `KDOWN <key>` | Hold a key |
| `KUP <key>` | Release a key |
| `HOTKEY <k1+k2>` | Press the steps in order, then release them in reverse |

`KEY` takes modifiers plus exactly one key, for example `ctrl+c` or `cmd+shift+a`. Letters are case-insensitive and name the key, not Shift: `KEY A` taps `a`. Canonical order is `cmd`, `ctrl`, `alt`, `shift`, then the key. Aliases include `command`/`meta` for `cmd`, `control` for `ctrl`, `opt`/`option` for `alt`, `return` for `enter`, and `escape` for `esc`. A bad chord answers `ERR KEY bad`.

## Layout

```
upload.sh                 — compile, then upload
scripts/settle_upload.py  — pick the XIAO port and enter the bootloader
scripts/upload_bossac.py  — retry bossac after USB settles
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
