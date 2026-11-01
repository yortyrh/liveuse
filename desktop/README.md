# LiveUse desktop

macOS app (Tauri 2, React). When the XIAO is running the sketch, it publishes a local-network API and MCP for the mouse and keyboard. Closing the window hides it: the process stays in the menu bar. **Quit** in that menu ends the app. A click on the Dock icon opens it again.

Node.js, pnpm, and Rust (cargo) are required. The first version is for macOS.

## Development

From the repo root:

```sh
cd desktop
pnpm install
pnpm tauri dev
```

`pnpm tauri dev` builds the Rust backend and opens the window. Vite starts first with `pnpm dev`, on port 1420.

Connect the XIAO with the sketch running (`idProduct` `32815`). The bootloader does not start the service. The **Share** tab shows the URL (`http://<lan-ip>:8765`), the token, and the MCP JSON. Those credentials are copied on this machine. Anyone with the token on the local network can move the cursor and the keyboard.

macOS asks for two permissions: screen recording (captures for the agent and for a remote machine) and incoming connections.

## Shortcut

**Ctrl+Option+Cmd+Esc** aborts the current operation and opens the window again. The same action is the **Stop** button in the chat. The agent loop runs in Rust with [Rig](https://rig.rs/docs).

## Build

```sh
cd desktop
pnpm tauri build
```

The installer is in `desktop/src-tauri/target/release/bundle/`.

## Tests

The Rust tests do not need the XIAO:

```sh
cargo test --manifest-path desktop/src-tauri/Cargo.toml
```

The firmware CDC commands are in `README.md` of the `liveuse-firmware-xiao-samd21` repository.
