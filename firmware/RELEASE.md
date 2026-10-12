# Firmware releases

Firmware versions are published to the public repository `yortyrh/liveuse-releases`. That repository has no source code and no desktop app releases. Each version is one GitHub release. The `main` branch holds the latest `manifest.json` and nothing else that the app needs.

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

`sha256` is the hex digest of the asset. `size` is its length in bytes. `usb` and `flash` are copied from `firmware/boards.json`, so a new board does not need a change to the script.

The SAMD21 image is the application. `offset` is `0x2000` because that is where the sketch starts. A bossac 1.9 that is run without `--offset` writes at `0x0` and erases the bootloader. The flasher has to pass this offset.

## Boards

`firmware/boards.json` lists the images to build. Today that is the PlatformIO env `seeed_xiao`. The running sketch uses USB `0x2886`/`0x802F`. The SAM-BA bootloader uses `0x2886`/`0x002F`.

The Seeed XIAO ESP32-S3 is not built yet. When a `seeed_xiao_esp32s3` env exists, add an object to the catalog. The update image is `firmware.bin` at `0x10000` (the application), not the bootloader or the partition table. `flash.protocol` is `esptool`. The PlatformIO board manifest on a machine that already has the Espressif platform lists USB `0x2886`/`0x8056` and `0x2886`/`0x0056`, 8MB flash, mode `qio`. Put those values in the catalog when that board is real. The script copies `flash_mode`, `flash_freq`, and `flash_size` through with the rest of `flash`.

## What the app reads later

This repository does not check or flash a board. A later desktop build can do that on Windows, Linux, and macOS from this manifest.

Read `fw=` from `STATUS`. Fetch `manifest.json` over HTTPS. Compare `major.minor.patch` as integers. If the manifest is newer and `usb.running` matches the connected board, download that board's `url` and check `sha256` and `size`. `flash.protocol`, `enter`, and `offset` say how to write the image: for the SAMD21, a 1200 baud pulse, then bossac with `--offset=0x2000`; for the ESP32-S3, once it exists, the application image at `0x10000`. The USB id is matched from the serial port list, which names the port `COM3`, `/dev/ttyACM0`, or `/dev/cu.usbmodem…` depending on the system. An image for a different board is not offered.

The releases repository has to stay public. A private repository cannot be read without a token.
