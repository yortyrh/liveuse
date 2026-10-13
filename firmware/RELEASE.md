# Firmware releases

Firmware versions for the ATSAMD21G18A (Seeed XIAO SAMD21) are published to the public repository https://github.com/yortyrh/liveuse-releases. That repository has no source code. Firmware releases use tags `firmware-v*` and `manifest.json` on `main`. Desktop app releases use tags `app-v*` and `apps.json` on `main`; this script does not write that file. Each firmware version is one GitHub release. The desktop app reads `manifest.json` for firmware.

An image for any other microcontroller stays in this private repository. It is uploaded to `yortyrh/liveuse-releases` only when the developer explicitly authorizes that microcontroller.

The tag `firmware-vX.Y.Z` is created only in that repository. This script does not tag or push `yortyrh/liveuse` or `yortyrh/mcp-mouse`. A published version is not replaced. The next version is a new tag.

`LIVEUSE_RELEASE_REPO` overrides the repository. It has to look like `owner/name`. The default branch of that repository has to be `main`, because the manifest URL is fixed:

```
https://raw.githubusercontent.com/yortyrh/liveuse-releases/main/manifest.json
```

The binary for a board is a release asset. Its URL is fixed once the version is chosen:

```
https://github.com/yortyrh/liveuse-releases/releases/download/firmware-v0.4.1/liveuse-seeed_xiao-0.4.1.bin
```

GitHub picks `releases/latest` from the date of the tagged commit, not from the day the release was published. The file on `main` is the one to read. The release is still marked latest, as a backup.

## Publish

`firmware/VERSION`, the `fw=` string in `src/main.cpp`, the version line and the `fw=` example in `README.md`, and the newest `CHANGELOG.md` heading have to be the same `X.Y.Z` string. The working tree has to be clean. The script does not bump the version.

From the repository root:

```sh
python3 firmware/scripts/release.py --check
python3 firmware/scripts/release.py
```

On Windows, use `python` when `python3` is not on `PATH`.

`--check` compares those version strings and reads `firmware/boards.json`. It does not compile and it does not use the network.

`--dry-run` compiles every board in the catalog and prints the manifest. It does not upload.

`--point` copies `manifest.json` from an existing release onto `main`. Use it when the release was created and that write failed. It does not replace `main` with an older version.

## Manifest

`schema` is `1`. `version` and `tag` name the release. `notes` is the changelog section for that version. `boards` is one entry per image.

```json
{
  "schema": 1,
  "version": "0.4.1",
  "tag": "firmware-v0.4.1",
  "notes": "## [0.4.1] - 2026-10-11\n...",
  "boards": [
    {
      "id": "seeed_xiao",
      "name": "Seeed XIAO SAMD21",
      "file": "liveuse-seeed_xiao-0.4.1.bin",
      "url": "https://github.com/yortyrh/liveuse-releases/releases/download/firmware-v0.4.1/liveuse-seeed_xiao-0.4.1.bin",
      "sha256": "<hex>",
      "size": 0,
      "usb": {
        "running": { "vid": "0x2886", "pid": "0x802F" },
        "bootloader": { "vid": "0x2886", "pid": "0x002F" }
      },
      "flash": {
        "protocol": "sam-ba",
        "enter": "1200bps-touch",
        "offset": "0x2000"
      }
    }
  ]
}
```

`sha256` is the hex digest of the asset. `size` is its length in bytes. `usb` and `flash` are copied from `firmware/boards.json`. Another ATSAMD21G18A board can be added to that catalog without changing the script. A board with a different microcontroller is left out of this publish unless the developer explicitly authorizes that microcontroller for `yortyrh/liveuse-releases`.

The SAMD21 image is the application. `offset` is `0x2000` because that is where the sketch starts. A bossac 1.9 that is run without `--offset` writes at `0x0` and erases the bootloader. The flasher has to pass this offset.

## Boards

`firmware/boards.json` lists the images this script builds and uploads. The public entry is the PlatformIO env `seeed_xiao`, the ATSAMD21G18A. The running sketch uses USB `0x2886`/`0x802F`. The SAM-BA bootloader uses `0x2886`/`0x002F`.

Support for another microcontroller can live in this private repository. Its image stays out of `yortyrh/liveuse-releases`, and out of the catalog this script uploads, until the developer explicitly authorizes that microcontroller. The Seeed XIAO ESP32-S3 is one such board: it is not built yet, and building it does not publish it. When that private image exists, the update file is `firmware.bin` at `0x10000` (the application), not the bootloader or the partition table, and `flash.protocol` is `esptool`. The PlatformIO board manifest on a machine that already has the Espressif platform lists USB `0x2886`/`0x8056` and `0x2886`/`0x0056`, 8MB flash, mode `qio`. Those values describe the board. They do not authorize a public upload. The script copies `flash_mode`, `flash_freq`, and `flash_size` through with the rest of `flash` only for a board that is allowed in this publish.

## What the app reads

The desktop app reads `fw=` from `STATUS`, fetches `manifest.json` over HTTPS, and compares `major.minor.patch` as integers. When the manifest is newer and `usb.running` matches the connected board, it downloads that board's `url` and checks `sha256` and `size`. The app writes the image itself, through the SAM-BA monitor, at offset `0x2000` after the 1200 baud pulse. It does not invoke a `bossac` on the system. The manifest lists the ATSAMD21G18A. An image for another microcontroller is absent unless the developer explicitly authorized publishing it. The USB id is matched from the serial port list, which names the port `COM3`, `/dev/ttyACM0`, or `/dev/cu.usbmodem…` depending on the system. An image for a different board is not offered.

The releases repository has to stay public. A private repository cannot be read without a token.
