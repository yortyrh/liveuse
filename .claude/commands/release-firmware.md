---
description: Publish the current ATSAMD21G18A firmware to yortyrh/liveuse-releases
---

Publish the firmware version in `firmware/VERSION` to https://github.com/yortyrh/liveuse-releases. That repository receives firmware for the ATSAMD21G18A (Seeed XIAO SAMD21). Today that image is the `seeed_xiao` entry in `firmware/boards.json`.

Firmware for any other microcontroller stays in this private repository. Upload it to `yortyrh/liveuse-releases` only when the developer explicitly authorizes that microcontroller in the request that asks for the publish.

`firmware/CHANGELOG.md` needs a `## [X.Y.Z]` section for the version in `firmware/VERSION`. When that section is missing, write it with the firmware changelog skill. Do not commit or tag that edit yourself.

`firmware/scripts/release.py` rewrites `fw=` and the USB product, mouse, and keyboard names in `firmware/src/main.cpp` and `firmware/README.md` so they match `firmware/VERSION`. When `firmware/VERSION`, `firmware/src/main.cpp`, `firmware/README.md`, or `firmware/CHANGELOG.md` differ from HEAD, it commits those files and creates the annotated tag `firmware-vX.Y.Z` on that commit. It does not push that commit or tag to `yortyrh/liveuse` or `yortyrh/mcp-mouse`. It does not pass `--force`. A published version is not replaced. Any other uncommitted file stops the script.

The script uploads every board in `firmware/boards.json`. Before the command below, read that file and the PlatformIO env of each board. A board belongs in this publish when its microcontroller is the ATSAMD21G18A. When a board uses another microcontroller and this request did not explicitly authorize it for `yortyrh/liveuse-releases`, stop and name that board. Do not run the script.

From the repository root, run:

```sh
python3 firmware/scripts/release.py
```

On Windows, use `python` when `python3` is not on `PATH`.

`--check` and `--dry-run` do not rewrite files, commit, or tag. If the script fails before it prints a release URL, show its message and stop.

Show the release URL it prints, and the commit and tag lines it prints. If it says the release is up but `manifest.json` on `main` was not updated, stop and report that. Do not run the script again unless the user asks to retry with `--point`.
