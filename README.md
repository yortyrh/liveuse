# LiveUse

This repository contains the LiveUse firmware in `firmware/`.

The sketch targets a Seeed XIAO SAMD21 (`ATSAMD21G18A`) with PlatformIO (`pio`).

## Build

```sh
pio run -e seeed_xiao -d firmware
```

The image is `firmware/.pio/build/seeed_xiao/firmware.bin`.

## Flash

Plug in the XIAO and close anything holding its CDC port, then:

```sh
firmware/upload.sh
```

`upload.sh` compiles the `seeed_xiao` environment and uploads. The CDC commands, the 1200-baud bootloader reset, and the file layout are in `firmware/README.md`.
