---
description: Publish the current firmware version to yortyrh/liveuse-releases
---

Publish the firmware version already written in `firmware/VERSION`. Do not change that version, the `fw=` string, `firmware/CHANGELOG.md`, or `firmware/README.md`. A version bump happens before this command, with the firmware changelog skill.

Do not create tags or push `yortyrh/liveuse` or `yortyrh/mcp-mouse`. Do not pass `--force`. A published version is not replaced.

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
