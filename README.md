# LiveUse

This repository contains the LiveUse firmware in `firmware/`.

The sketch targets a Seeed XIAO SAMD21 (`ATSAMD21G18A`) with PlatformIO (`pio`).

## Build

```sh
pio run -e seeed_xiao -d firmware
```

The image is `firmware/.pio/build/seeed_xiao/firmware.bin`.

## Flash

Plug in the XIAO, then:

```sh
firmware/scripts/flash
```

It compiles, finds the board, and uploads with `pio`, which restarts the board at 1200 baud. The CDC commands and the file layout are in `firmware/README.md`.
