# LiveUse

This repository contains the LiveUse macOS app in `desktop/`. The firmware lives in its own repository, `liveuse-firmware-xiao-samd21`.

The firmware targets a Seeed XIAO SAMD21 (`ATSAMD21G18A`) with PlatformIO (`pio`). The app is Tauri 2. It starts a local API and MCP when the XIAO is running the sketch. How to run it is in `desktop/README.md`.

```sh
cd desktop
pnpm install
pnpm tauri dev
```
