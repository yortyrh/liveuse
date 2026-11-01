---
description: Bump the macOS app version and publish it to yortyrh/liveuse-releases
argument-hint: "[patch|minor|major|X.Y.Z]"
---

Publish one macOS app release, `app-vX.Y.Z`, to https://github.com/yortyrh/liveuse-releases. That release holds the Intel DMG and the Apple Silicon DMG.

The token after this command chooses the version. Pass that token through to the script. The script reads the current version, calculates the next one, writes it, and commits only the app version files. Leave that work to the script.

- no token, or `patch`: increment the patch component
- `minor`: increment the minor component and set patch to 0
- `major` or `mayor`: increment the major component and set minor and patch to 0
- `X.Y.Z`: set that version

When this prompt still shows `$ARGUMENTS`, replace it with the user's token. When the user typed no token, delete `$ARGUMENTS`.

From the repository root, run:

```sh
python3 scripts/version.py app $ARGUMENTS
```

On Windows, use `python` when `python3` is not on `PATH`.

Stdout is the version to publish, one `X.Y.Z` line. If the script fails, show its message and stop. Do not publish. Do not edit `desktop/package.json`, `desktop/src-tauri/tauri.conf.json`, `desktop/src-tauri/Cargo.toml`, `desktop/src-tauri/Cargo.lock`, or `desktop/src-tauri/src/mcp.rs` yourself, and do not commit them again.

`current` and `next` only print a version. This command uses `app`, which writes.

That repository also holds firmware. This command does not write `manifest.json`, does not create a `firmware-v*` tag, and does not mark the app release as latest.

After the upload, the script tags HEAD in `yortyrh/liveuse` as `app-vX.Y.Z`, pushes only that tag, and creates a GitHub release there with the same notes and a link to the download. That release has no assets and is not marked as latest. Do not create tags or push anything else, and do not touch `yortyrh/mcp-mouse`. Do not pass `--force`. A published version is not replaced.

The build runs on macOS. It needs both Rust targets (`x86_64-apple-darwin` and `aarch64-apple-darwin`). Without a Developer ID Application certificate, the script passes `--no-sign` and records `"signed": false`.

Then run:

```sh
python3 desktop/scripts/release.py --check
```

`--check` only compares the version strings. It does not compile and it does not use the network.

If that command fails, show its message and stop. Do not publish.

If it succeeds, run:

```sh
python3 desktop/scripts/release.py
```

Show the release URL it prints. If it says the release is up but `apps.json` on `main` was not updated, stop and report that. Do not run the script again unless the user asks to retry with `--point`. Also show the `yortyrh/liveuse` release URL. If it says the release is up but not registered on `yortyrh/liveuse`, stop and report that. Do not run the script again unless the user asks to retry with `--register`.
