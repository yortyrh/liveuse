# macOS app releases

macOS builds of the LiveUse desktop app are published to the public repository https://github.com/yortyrh/liveuse-releases. That repository has no source code. Firmware stays on tags `firmware-v*` and in `manifest.json`. The app stays on tags `app-v*` and in `apps.json`. This script does not write `manifest.json` and does not mark its release as latest, so the firmware backup on `releases/latest` stays where the firmware script left it.

The tag `app-vX.Y.Z` is created only in that repository. One release holds both installers. This script does not tag or push `yortyrh/liveuse` or `yortyrh/mcp-mouse`. A published version is not replaced. The next version is a new tag.

`LIVEUSE_RELEASE_REPO` overrides the repository. It has to look like `owner/name`. The default branch of that repository has to be `main`, because the manifest URL is fixed:

```
https://raw.githubusercontent.com/yortyrh/liveuse-releases/main/apps.json
```

Each installer is a release asset. Its URL is fixed once the version is chosen:

```
https://github.com/yortyrh/liveuse-releases/releases/download/app-v0.1.0/LiveUse_0.1.0_x64.dmg
https://github.com/yortyrh/liveuse-releases/releases/download/app-v0.1.0/LiveUse_0.1.0_aarch64.dmg
```

The site reads `apps.json`. It does not call `releases/latest`.

## Publish

`desktop/package.json` and `desktop/src-tauri/tauri.conf.json` have to use the same `X.Y.Z` string. The working tree has to be clean. This script does not bump the version. Set it first with `python3 scripts/version.py app`. `patch`, `minor`, `major`, and `mayor` increment that component. With no part, the patch number increments. `X.Y.Z` sets that version. The build runs on macOS and needs the Rust targets `x86_64-apple-darwin` and `aarch64-apple-darwin`.

From the repository root:

```sh
python3 desktop/scripts/release.py --check
python3 desktop/scripts/release.py
```

On Windows, use `python` when `python3` is not on `PATH`. `--check` can run there. The compile cannot.

`--check` compares those version strings. It does not compile and it does not use the network.

`--dry-run` compiles both architectures and prints `apps.json`. It does not upload.

`--point` copies `apps.json` from an existing release onto `main`. Use it when the release was created and that write failed. It does not replace `main` with an older version, and it does not write `manifest.json`.

Without a Developer ID Application certificate, the build passes `--no-sign` and `signed` is `false`.

## Manifest

`schema` is `1`. `version` and `tag` name the release. `signed` is true only when a Developer ID Application certificate was present at build time. `downloads` has one entry per installer. Intel is `macos-x64`. Apple Silicon is `macos-aarch64`.

```json
{
  "schema": 1,
  "version": "0.1.0",
  "tag": "app-v0.1.0",
  "signed": false,
  "downloads": [
    {
      "id": "macos-x64",
      "os": "macos",
      "arch": "x64",
      "name": "macOS Intel",
      "file": "LiveUse_0.1.0_x64.dmg",
      "url": "https://github.com/yortyrh/liveuse-releases/releases/download/app-v0.1.0/LiveUse_0.1.0_x64.dmg",
      "sha256": "<hex>",
      "size": 0
    },
    {
      "id": "macos-aarch64",
      "os": "macos",
      "arch": "aarch64",
      "name": "macOS Apple Silicon",
      "file": "LiveUse_0.1.0_aarch64.dmg",
      "url": "https://github.com/yortyrh/liveuse-releases/releases/download/app-v0.1.0/LiveUse_0.1.0_aarch64.dmg",
      "sha256": "<hex>",
      "size": 0
    }
  ]
}
```

`sha256` is the hex digest of the asset. `size` is its length in bytes. The file name is `{productName}_{version}_{arch}.dmg`.

The releases repository has to stay public. A private repository cannot be read without a token.
