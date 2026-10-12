# LiveUse app icon

## Goal

Replace the abstract cyan-and-amber mark with two drawings of the same idea: a pointer held at the tail by a spark. The Dock and the bundled app icon use the color version. The menu bar uses a black template of the same drawing, so macOS can invert it.

The window title stays LiveUse. There is no in-window logo to change.

## App icon

Source file: `desktop/src-tauri/icons/icon.svg`, viewBox `0 0 128 128`.

- Plate: rounded rect, `rx="28"`, fill `#111111`.
- Pointer: `M26 18 L26 84 L44 66 L58 94 L68 89 L54 61 L78 61 Z`, fill `#3EE0E8`, stroke `#071316`, stroke-width `4`, stroke-linejoin `round`.
- Spark, translated to `(52, 98)`, sitting on the pointer tail:
  - Outer four-point star, fill `#F5B429`: `0,-18 3.2,-3.2 18,0 3.2,3.2 0,18 -3.2,3.2 -18,0 -3.2,-3.2`
  - Inner four-point star, fill `#FFF4CC`: `0,-8 1.6,-1.6 8,0 1.6,1.6 0,8 -1.6,1.6 -8,0 -1.6,-1.6`

macOS paints an `.icns` at the size of its pixels. A plate that touches the canvas edge looks larger than neighboring Dock icons, so the plate sits at 80% of the canvas, centered: `translate(64 64) scale(0.8) translate(-64 -64)`. Inside that, the pointer and spark are centered on the plate with `translate(64 64) scale(0.64) translate(-52 -67)`. Their unscaled bounds are centered at `(52, 67)`.

From `desktop/`, generate the bundle set with the Tauri icon command and no extra flags:

```sh
pnpm tauri icon src-tauri/icons/icon.svg
```

The command accepts a square PNG or SVG and writes the desktop icons next to `tauri.conf.json`. Do not pass `--png` (that skips the default set) and do not pass an iOS color (that writes mobile assets this app does not use).

That regenerates the files already listed in `desktop/src-tauri/tauri.conf.json` (`32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.icns`, `icon.ico`) plus `icon.png` and the `Square*Logo.png` / `StoreLogo.png` files. Leave the `bundle.icon` array as it is.

## Menu bar icon

Source file: `desktop/src-tauri/icons/tray.svg`. Raster file the app embeds: `desktop/src-tauri/icons/trayTemplate.png`, 32×32, RGBA.

The drawing is the pointer and the spark only. Both shapes are RGB `0,0,0`. Transparency is the alpha channel, including anti-aliased edges. There is no plate, no cyan, no amber, and no light core: a template image is a black mask, and macOS tints it.

Paths, drawn in an 18×18 space and then scaled to fill the status item:

- Pointer: `M3 1.5 L3 12.5 L6 9.5 L8.2 14 L10 13.1 L7.8 8.6 L11.5 8.6 Z`.
- Spark: `12.2,11.2 12.7,12.6 14.2,13.1 12.7,13.6 12.2,15 11.7,13.6 10.2,13.1 11.7,12.6`.

The group is `translate(1.35 1.94) scale(1.704)`. A glyph that only occupies the middle of the 32×32 bitmap looks tiny in the menu bar, because macOS scales the whole bitmap into the status item. The outer four pixels may carry a faint antialiased fringe (alpha at most 80). Rasterize `tray.svg` to `trayTemplate.png` with a tool that keeps the alpha and does not add a background. Both files stay in the repo. `pnpm tauri icon` does not produce this file and must not be pointed at `tray.svg`. It also must leave `icon.svg` and `tray.svg` in place.

In `install_tray` (`desktop/src-tauri/src/lib.rs`), load `trayTemplate.png` instead of `icons/32x32.png`, and call `icon_as_template(true)` on the `TrayIconBuilder`. That method is macOS-only. The menu, the status item, and the open/quit handlers stay as they are.

## Out of scope

- Firmware, the in-window title, and the product name.
- A separate Windows tray drawing. The first version is macOS; the Windows bundle icons stay the color set the Tauri command writes.

## Check

- `32x32.png` shows the cyan pointer on the dark plate. The amber spark is counted on `128x128.png`, because the 32×32 raster keeps only a couple of amber pixels after the inset.
- `trayTemplate.png` is a black pointer and spark on a transparent background, and no second color.
- `cargo test --manifest-path desktop/src-tauri/Cargo.toml` still passes. A missing tray PNG fails the build at `include_bytes!`; there is no runtime fallback.
- With the app running on macOS, the Dock icon is the color mark, and the menu-bar glyph is dark on a light bar and light on a dark bar.
