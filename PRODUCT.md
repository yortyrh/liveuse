# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Two jobs, equally. The site has to serve both.

- A person installing LiveUse on their own Mac, the one with the board, so an agent can use that mouse and keyboard.
- A person on another computer on the same local network, driving that pointer.

## Product Purpose

LiveUse is the macOS app for a Seeed XIAO SAMD21. With the sketch running, the mouse and keyboard receive commands over serial, and LiveUse publishes that control on the local network as an API and an MCP.

The site exists so someone can download the right Mac installer, check it, and see both jobs: run it on the Mac with the board, and drive that pointer from another computer. Success is a correct download with a public checksum, and a visitor who understands that the token is required in both places.

## Positioning

The mouse and keyboard receive their commands over serial. LiveUse shares that control on the local network through an API and an MCP. A remote machine can use it only with the token. The local machine can use it only with the token too.

## Operating Context

The downloaded product is a macOS app. Closing the window leaves it in the menu bar. Quit in that menu ends the app. A Dock click opens the window again.

The board is a Seeed XIAO SAMD21 with the sketch running, plugged into that Mac. The bootloader does not start the service. The Share tab on that Mac shows the URL, the token, and the MCP configuration. Control on that Mac can drive the pointer when it has the token. On another computer, LiveUse can connect as Remote with the URL and the token, or an agent can call the API or the MCP directly.

macOS asks for screen recording and for incoming connections. Stopping an action is Control, Option, Command, and Escape together, or Stop in the chat.

Installers are one disk image for Intel and one for Apple Silicon. The site reads the public release list from GitHub (`yortyrh/liveuse-releases`) and links each installer there. The download itself happens on GitHub. There is no account and no analytics. Theme choice is stored in the browser.

## Capabilities and Constraints

- Today the installer is macOS, and the board is a Seeed XIAO SAMD21. Desktop builds for Linux and Windows are in progress. Other boards are being explored, including the first steps of a smaller board that plugs straight into the computer. The site must not present those as available.
- Free. No account. Each installer shows its size and SHA256 checksum.
- Anyone with the token on that local network can move the cursor and the keyboard, including the local machine. A new token from the Share tab stops the previous one.
- The site stays in English. Sentences stay short and concrete.
- When a build is not signed, the first open is a right-click and Open.
- Useful remote calls include screenshot, click, move_to, write, and scroll. Coordinates start at the top left of the primary display.

Resolved: the firmware has no RTOS. A single Arduino loop reads one CDC line and runs it. The site says so.

## Related projects

- `yortyrh/liveuse-releases`: app installers, firmware images, `manifest.json`, and the firmware guide (flashing, CDC commands, examples).
- `yortyrh/xiao-samd21-case`: the LuLu case, two printed parts, 25.0 × 21.6 × 8.9 mm, PLA or PETG, no supports. Its renders are the case images on the site and in the app.
- `liveuse-firmware-xiao-samd21`: the firmware source.

## Brand Commitments

The app is LiveUse. The device is LuLu: a printed case with a Seeed XIAO SAMD21 inside. Copy says LuLu, not XIAO; the chip appears only as a spec ("XIAO SAMD21 inside"). Every picture of the device shows it in its case, never the bare board. The image with the chip visible is `web/public/case/board-in-base.png`.

The name is LiveUse. The site voice is English, in short concrete sentences. The palette follows the case: cool blue-gray ground in light mode, navy in dark mode, blue for actions and the word LiveUse. The app and the site share these tokens.

## Evidence on Hand

- Download, setup, and FAQ copy in `web/src/app/page.tsx` and `web/src/lib/faq.ts`.
- Help copy and screenshots: `web/public/help/device.png`, `web/public/help/control-opus.png`, `web/public/help/share.png`, `web/public/help/remote-form.png`, `web/public/help/device-hero.png`.
- LuLu renders from `xiao-samd21-case/images/`: `web/public/case/*.png`. No bare-board photos. Mark: `web/public/logo.svg`.
- Product behavior in `desktop/README.md` and the firmware repository README.
- Public GitHub releases, linked from the download page, including release notes for the version shown.

No testimonials, customers, benchmarks, pricing, or licensing claims exist. Do not invent them.

## Product Principles

- Serve both jobs on the same page: install on the Mac with the board, and drive that pointer from another computer.
- The token is the gate on the local machine and on a remote one.
- Say only what ships today. macOS and the XIAO SAMD21 are available. Linux, Windows, and other boards stay in progress.
- The download stays free, account-free, and checkable against a public checksum.
- Write English in short, concrete sentences.
