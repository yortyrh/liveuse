# Changelog

Changes to the LiveUse firmware for the Seeed XIAO SAMD21. The version is the string in `firmware/VERSION`. `STATUS` prints that same string as `fw=`.

## [0.5.0] - 2026-10-12

### Changed

- The USB device identifies as LiveUse 0.5.0, made by LiveUse. The mouse interface is LiveUse Mouse 0.5.0 and the keyboard interface is LiveUse Keyboard 0.5.0.

## [0.4.1] - 2026-10-11

### Fixed

- `TYPE` types the spaces in the text. One space separates the command from the payload; every space after that is typed.
- The right GUI key is named `cmdright`. The spelling `cmdrigh` never matched.

## [0.4.0] - 2026-10-06

### Added

- `KDOWN` and `KUP` hold and release a key. `HOTKEY` presses its steps in order and releases them in reverse.
- `MDOWN` and `MUP` hold or release a mouse button. Later `MOVE` lines keep that button down.
- `HSCROLL` moves the horizontal wheel. Positive is right, negative is left.
- Upload reaches the SAM-BA bootloader by opening the running sketch's CDC port at 1200 baud. `scripts/upload_bossac.py` retries the page write after that USB port settles.
- `scripts/flash` compiles, finds the XIAO, and stops when another program holds the serial port. It replaces `upload.sh` (2026-10-09).

### Changed

- `DEMO` is gone, including `circle`, `zigzag`, and `both`. The sketch only moves or types when a CDC line asks it to.
- The CDC banner is `=== LiveUse ===`.

### Fixed

- A CDC line that does not fit the buffer answers `ERR line too long` and does not run (2026-10-10).
- `CLICK` no longer releases buttons that `MDOWN` is holding (2026-10-10).

## [0.3.0] - 2026-10-05

### Added

- A second HID interface: a boot keyboard beside the relative mouse.
- `TYPE` sends printable ASCII (`0x20`–`0x7E`). Every byte is checked before any report goes out. Success prints nothing.
- `KEY` taps one key plus optional modifiers, using the US ANSI keymap. A bad chord answers `ERR KEY bad`.
- `CLICK` (`left`, `right`, or `double`) and `SCROLL` (positive is up, negative is down).
- `STATUS` reports the firmware version as `fw=`.

### Removed

- Opening the CDC port no longer runs a pointer demo. The boot `DEMO both` is gone.

## Before 0.3.0

No version string. The sketch lived in `firmware/poc1-samd21-hid-cdc` (2026-10-02).

### Added

- One Type-C port enumerates a relative HID mouse and CDC serial.
- CDC commands were `HELP`, `STATUS` (uptime only), `ECHO`, `MOVE`, and `DEMO circle|zigzag|both`.
- About two seconds after boot the firmware ran `DEMO both`.

### Changed

- Two FreeRTOS tasks and a command queue became one Arduino `loop()`. The core `yield()` pumps TinyUSB.
- The banner prints again each time the CDC port opens.
- `scripts/upload_bossac.py` retries a page write after the bootloader disk appears.
