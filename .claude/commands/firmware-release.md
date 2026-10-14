---
description: Bump the ATSAMD21G18A firmware version and publish it to yortyrh/liveuse-releases
argument-hint: "[patch|minor|major|X.Y.Z]"
---

Publish the firmware for the ATSAMD21G18A (Seeed XIAO SAMD21) to https://github.com/yortyrh/liveuse-releases. Today that image is the `seeed_xiao` entry in `firmware/boards.json`.

Firmware for any other microcontroller stays in this private repository. Upload it to `yortyrh/liveuse-releases` only when the developer explicitly authorizes that microcontroller in the request that asks for the publish.

The token after this command chooses the version. Pass that token through to the script. The script reads `firmware/VERSION`, calculates the next version, and writes that file. Leave that work to the script.

- no token, or `patch`: increment the patch component
- `minor`: increment the minor component and set patch to 0
- `major` or `mayor`: increment the major component and set minor and patch to 0
- `X.Y.Z`: set that version

When this prompt still shows `$ARGUMENTS`, replace it with the user's token. When the user typed no token, delete `$ARGUMENTS`.

From the repository root, run:

```sh
python3 scripts/version.py firmware $ARGUMENTS
```

On Windows, use `python` when `python3` is not on `PATH`.

Stdout is the version to publish, one `X.Y.Z` line. If the script fails, show its message and stop. Do not publish. Do not edit `firmware/VERSION` yourself.

`current` and `next` only print a version. This command uses `firmware`, which writes.

`firmware/CHANGELOG.md` needs a `## [X.Y.Z]` section for the version the script printed. When that section is missing, write it with the firmware changelog skill. Write the changelog section only. `firmware/scripts/release.py` rewrites `fw=` and the USB product, mouse, and keyboard names in `firmware/src/main.cpp` and `firmware/README.md`. Do not commit or tag that edit yourself.

When `firmware/VERSION`, `firmware/src/main.cpp`, `firmware/README.md`, or `firmware/CHANGELOG.md` differ from HEAD, `firmware/scripts/release.py` commits those files and creates the annotated tag `firmware-vX.Y.Z` on that commit. It does not push that commit or tag to `yortyrh/liveuse` or `yortyrh/mcp-mouse`. It does not pass `--force`. A published version is not replaced. Any other uncommitted file stops the script.

The script uploads every board in `firmware/boards.json`. Before the command below, read that file and the PlatformIO env of each board. A board belongs in this publish when its microcontroller is the ATSAMD21G18A. When a board uses another microcontroller and this request did not explicitly authorize it for `yortyrh/liveuse-releases`, stop and name that board. Do not run the script.

Then run:

```sh
python3 firmware/scripts/release.py
```

`--check` and `--dry-run` do not rewrite files, commit, or tag. If the script fails before it prints a release URL, show its message and stop.

Show the release URL it prints, and the commit and tag lines it prints. If it says the release is up but `manifest.json` on `main` was not updated, stop and report that. Do not run the script again unless the user asks to retry with `--point`.
