# Live Use

Computer use on the real machine. A Seeed XIAO SAMD21, plugged into the Mac by USB, is that machine's mouse and keyboard. One C++ daemon on the Mac owns the serial port, the screenshot, the MCP endpoint, and a small web bench.

## Run

```sh
cmake -S host -B host/build
cmake --build host/build
./host/build/liveuse init --user ada
./host/build/liveuse
```

The daemon listens on `0.0.0.0:7395`. The first non-loopback IPv4 is the URL shown in the web UI: `http://<ip>:7395/mcp`.

Cursor on another computer on the LAN:

```json
{
  "mcpServers": {
    "liveuse": {
      "url": "http://<ip-del-mini>:7395/mcp",
      "headers": {
        "Authorization": "Bearer <token>"
      }
    }
  }
}
```

The token is printed in the web UI after login. Do not commit it.

Web UI, during development:

```sh
pnpm install
pnpm web
```

Vite proxies `/api` and `/mcp` to `127.0.0.1:7395`. A production build belongs in `web/` next to the `liveuse` binary, or in the directory named by `LIVEUSE_WEB_ROOT`.

```sh
pnpm --filter liveuse build
cp -R apps/web/dist host/build/web
```

## What the agent can do

Mouse, keyboard, and screenshot follow the PyAutoGUI surface: move, drag, click, scroll, write, press, hotkey, capture, pixel, and image search. The XIAO sends relative HID reports. Position, absolute moves, and pixels are resolved on the Mac.

Firmware is PlatformIO, `firmware/`, version 0.4.0. Close anything holding the CDC port, then flash with the XIAO plugged in:

```sh
firmware/upload.sh
```

That script opens the port at 1200 baud so the board reboots into the bootloader. Details are in `firmware/README.md`.

## Tests

```sh
ctest --test-dir host/build --output-on-failure
pnpm test:web
```
