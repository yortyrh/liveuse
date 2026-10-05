# POC1 — XIAO SAMD21 HID + CDC

PlatformIO firmware for **Seeeduino XIAO / XIAO SAMD21**: USB **composite HID mouse + CDC serial** on one Arduino `loop()`.

## Quick flash

```bash
cd firmware/poc1-samd21-hid-cdc
pio run -t upload
pio device monitor -b 115200
```

Double-tap the RESET button if the board does not enter the SAM-BA bootloader for upload.

## Serial commands

| Command | Action |
|---------|--------|
| `HELP` | List commands |
| `STATUS` | Uptime |
| `ECHO <text>` | Echo on CDC |
| `MOVE <dx> <dy>` | Smooth relative HID move |
| `DEMO circle\|zigzag\|both` | Built-in smooth patterns |

Any other line is echoed (CDC sanity check). About 2 s after boot the firmware runs `DEMO both`. Motion blocks the loop until it finishes; the next CDC line is read after that.

## Layout

```
src/main.cpp    — USB init, CDC parse, command dispatch
src/motion.cpp  — Relative easing / circle / zigzag
include/motion.h
```

## Build notes

- `USE_TINYUSB` enables Adafruit TinyUSB in the Seeed SAMD core (composite CDC+HID).
- The core `yield()` inside `delay()` and `Serial` pumps TinyUSB. No RTOS.
