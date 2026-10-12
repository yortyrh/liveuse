---
description: Publish the current ATSAMD21G18A firmware to yortyrh/liveuse-releases
---

Publish the firmware version already written in `firmware/VERSION` to https://github.com/yortyrh/liveuse-releases. That repository receives firmware for the ATSAMD21G18A (Seeed XIAO SAMD21). Today that image is the `seeed_xiao` entry in `firmware/boards.json`.

Firmware for any other microcontroller stays in this private repository. Upload it to `yortyrh/liveuse-releases` only when the developer explicitly authorizes that microcontroller in the request that asks for the publish.

Do not change that version, the `fw=` string, `firmware/CHANGELOG.md`, or `firmware/README.md`. A version bump happens before this command, with the firmware changelog skill.

Do not create tags or push `yortyrh/liveuse` or `yortyrh/mcp-mouse`. Do not pass `--force`. A published version is not replaced.

`firmware/scripts/release.py` uploads every board in `firmware/boards.json`. Before the commands below, read that file and the PlatformIO env of each board. A board belongs in this publish when its microcontroller is the ATSAMD21G18A. When a board uses another microcontroller and this request did not explicitly authorize it for `yortyrh/liveuse-releases`, stop and name that board. Do not run the script.

From the repository root, run:

```sh
python3 firmware/scripts/release.py --check
```

On Windows, use `python` when `python3` is not on `PATH`.

If that command fails, show its message and stop. Do not publish.

If it succeeds, run:

```sh
python3 firmware/scripts/release.py
```

Show the release URL it prints. If it says the release is up but `manifest.json` on `main` was not updated, stop and report that. Do not run the script again unless the user asks to retry with `--point`.
