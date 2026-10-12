---
description: Publish the current macOS app to yortyrh/liveuse-releases
---

Publish the macOS app version already written in `desktop/package.json` and `desktop/src-tauri/tauri.conf.json` to https://github.com/yortyrh/liveuse-releases. Those two version strings have to match. This publishes one release, `app-vX.Y.Z`, with the Intel DMG and the Apple Silicon DMG.

That repository also holds firmware. This command does not write `manifest.json`, does not create a `firmware-v*` tag, and does not mark the app release as latest.

Do not change the version. Do not create tags or push `yortyrh/liveuse` or `yortyrh/mcp-mouse`. Do not pass `--force`. A published version is not replaced.

The build runs on macOS. It needs both Rust targets (`x86_64-apple-darwin` and `aarch64-apple-darwin`). Without a Developer ID Application certificate, the script passes `--no-sign` and records `"signed": false`.

From the repository root, run:

```sh
python3 desktop/scripts/release.py --check
```

On Windows, use `python` when `python3` is not on `PATH`. `--check` only compares the version strings. It does not compile and it does not use the network.

If that command fails, show its message and stop. Do not publish.

If it succeeds, run:

```sh
python3 desktop/scripts/release.py
```

Show the release URL it prints. If it says the release is up but `apps.json` on `main` was not updated, stop and report that. Do not run the script again unless the user asks to retry with `--point`.
