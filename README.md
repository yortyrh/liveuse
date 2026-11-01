# LiveUse

LiveUse drives **LuLu**, a USB device: a printed case with a Seeed XIAO SAMD21 inside, running the LiveUse firmware. Users see LuLu; the chip is a spec, not the product name. In every picture, show the board inside the case.

LiveUse is three projects that work together:

| Project | What it holds |
|---|---|
| This repository | The macOS app in `desktop/` (Tauri 2) and the website in `web/` (Next.js) |
| [`liveuse-releases`](https://github.com/yortyrh/liveuse-releases) | Published app installers and firmware images, `manifest.json`, `apps.json`, and the firmware guide: flashing, CDC commands, and examples |
| [`xiao-samd21-case`](https://github.com/yortyrh/xiao-samd21-case) | The LuLu case: parametric, 3D-printable, with 3MF and STL files and the renders |

The firmware source lives in its own repository, `liveuse-firmware-xiao-samd21`. It targets a Seeed XIAO SAMD21 (`ATSAMD21G18A`) with PlatformIO (`pio`). A single Arduino loop reads one CDC line and runs it; there is no RTOS.

The app starts a local API and MCP when LuLu is connected and running the firmware. The Device tab reads the public `manifest.json` and writes new firmware onto the board. How to run it is in `desktop/README.md`.

```sh
cd desktop
pnpm install
pnpm tauri dev
```

The website reads `apps.json` and `manifest.json` from `liveuse-releases`. The case renders in `web/public/case/` and `desktop/src/assets/case/` come from `xiao-samd21-case/images/`.

```sh
cd web
pnpm install
pnpm dev
```
