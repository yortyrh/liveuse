# POC1 — XIAO SAMD21 HID + CDC (FreeRTOS)

PlatformIO firmware for **Seeeduino XIAO / XIAO SAMD21** demonstrating USB **composite HID mouse + CDC serial** with a real FreeRTOS two-task architecture.

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
| `STATUS` | FreeRTOS heap + queue depth |
| `ECHO <text>` | Echo on CDC |
| `MOVE <dx> <dy>` | Smooth relative HID move |
| `DEMO circle\|zigzag\|both` | Built-in smooth patterns |

Any other line is echoed (CDC sanity check). On boot the firmware auto-queues `DEMO both`.

## Layout

```
src/main.cpp       — USB init, queue, task create, vTaskStartScheduler
src/cdc_task.cpp   — Task A: CDC RX / parse → queue
src/hid_task.cpp   — Task B: queue → smooth HID motion
src/motion.cpp     — Relative easing / circle / zigzag
include/cmd_queue.h
```

## Build notes

- `USE_TINYUSB` enables Adafruit TinyUSB in the Seeed SAMD core (composite CDC+HID).
- FreeRTOS via [Seeed_Arduino_FreeRTOS](https://github.com/Seeed-Studio/Seeed_Arduino_FreeRTOS); `tinyusb_task()` pumps USB on a dedicated task.
- Linker flag `-Wl,--allow-multiple-definition` resolves Arduino `yield()` vs FreeRTOS `yield()` (documented Seeed/PlatformIO issue).
