# mcp-mouse

Physical USB HID mouse on a **Seeeduino XIAO / XIAO SAMD21**, driven over **CDC serial** on the same Type‑C. Host-side C++ service + MCP come in later POCs.

## Primary path — POC1 (SAMD21 + FreeRTOS)

| Item | Value |
|------|--------|
| Board | Seeeduino XIAO / XIAO SAMD21 (`ATSAMD21G18A`) |
| USB | **Composite**: HID mouse + CDC serial (TinyUSB) |
| Concurrency | **FreeRTOS** — `cdcRecvTask` → queue → `hidMotionTask` |
| Firmware | [`firmware/poc1-samd21-hid-cdc/`](firmware/poc1-samd21-hid-cdc/) |

### Flash (PlatformIO)

1. Install [PlatformIO Core](https://platformio.org/install/cli).
2. Plug the **XIAO SAMD21** with a **data** USB‑C cable.
3. `cd firmware/poc1-samd21-hid-cdc`
4. `pio run -t upload` (if needed: double-tap RESET for bootloader, retry).
5. Open a serial monitor at **115200** baud (`pio device monitor`).

**Expect:** OS lists a new mouse *and* a serial port; banner on CDC; boot demo moves the cursor (circle then zigzag). Try `HELP`, `ECHO hi`, `MOVE 80 0`, `DEMO circle`.

More detail: Agent Store doc `docs/poc1-samd21-notes.md` (flash checklist + FreeRTOS diagram).

## Historical note — ESP32-C3 (dead end for HID)

An earlier POC1 attempt targeted **XIAO ESP32-C3**. That path is a **hardware dead end**: C3 USB is fixed Serial/JTAG and cannot enumerate as a TinyUSB HID mouse. Stub firmware may still exist on branch `cursor/poc1-hid-mouse-e17e` for context only — **do not use C3 as the HID target**.

## Scope (POC1)

- **In:** SAMD21 composite HID+CDC, FreeRTOS two-task design, smooth relative demo / `MOVE`
- **Out:** Host C++ service, MCP, Wi‑Fi, keyboard, LLM

## License

Unless noted otherwise, code in this repo is for mcp-mouse project experiments.
