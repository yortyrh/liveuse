# LiveUse

LiveUse drives **LuLu**, a USB device: a printed case with a Seeed XIAO SAMD21 inside, running the LiveUse firmware. Users see LuLu; the chip is a spec, not the product name. In every picture, show the board inside the case.

## How the projects connect

LiveUse is four repositories that work together:

| Repository | Visibility | What it holds |
|---|---|---|
| [`liveuse`](https://github.com/yortyrh/liveuse) (this one) | Public | The macOS app in `desktop/` (Tauri 2) and the website in `web/` (Next.js) |
| [`liveuse-firmware-xiao-samd21`](https://github.com/yortyrh/liveuse-firmware-xiao-samd21) | Private | Firmware source for the XIAO SAMD21: USB HID mouse and keyboard driven by CDC serial commands |
| [`liveuse-releases`](https://github.com/yortyrh/liveuse-releases) | Public | Published app installers and firmware images, `manifest.json`, `apps.json`, and the firmware guide: flashing, CDC commands, and examples |
| [`xiao-samd21-case`](https://github.com/yortyrh/xiao-samd21-case) | Public | The LuLu case: parametric, 3D-printable, with 3MF and STL files and the renders |

```
liveuse-firmware-xiao-samd21 ──build──▶ liveuse-releases ◀──reads manifest.json / apps.json── liveuse (app + web)
                                         (firmware .bin,                                            ▲
                                          app installers)                        renders ──── xiao-samd21-case
```

- The firmware repository builds the `.bin` and publishes it as a release in `liveuse-releases`, with its `manifest.json` entry.
- The app in this repository releases its installers to `liveuse-releases` too, listed in `apps.json`.
- The app and the website read `manifest.json` and `apps.json` from `liveuse-releases`, and the app flashes that firmware onto LuLu.
- The case renders in `web/public/case/` and `desktop/src/assets/case/` come from `xiao-samd21-case/images/`.

Hardware:

- Board: [Seeed Studio XIAO SAMD21 (Seeeduino XIAO)](https://wiki.seeedstudio.com/Seeeduino-XIAO/), `ATSAMD21G18A`.
- Case: [`xiao-samd21-case` on Creality Cloud](https://www.crealitycloud.com/model-detail/xiao-samd21-case), ready to print.

The firmware targets the XIAO SAMD21 with PlatformIO (`pio`). A single Arduino loop reads one CDC line and runs it; there is no RTOS.

## Run it

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

## License

[MIT](LICENSE)
