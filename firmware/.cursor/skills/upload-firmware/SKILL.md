---
name: upload-firmware
description: >-
  Flash the Live Use Seeed XIAO SAMD21 firmware with firmware/upload.sh.
  Use when the user asks to upload, flash, program, or subir the XIAO,
  or to run upload.sh.
---

# Upload firmware

Flash only by running `upload.sh`. It compiles the `seeed_xiao` environment, opens the XIAO CDC port at 1200 baud, closes it so TinyUSB reboots into the SAM-BA bootloader, then runs bossac.

Do not open the port at 1200 yourself first, call bossac directly, or send a USB control transfer. A second 1200 touch races the bootloader. Do not upload to a Bluetooth serial port.

## Before running

1. The Seeed XIAO must be plugged in by USB. `/dev/cu.usbmodem*` should exist, or `ioreg` should show `Seeeduino XIAO`.
2. Nothing else may hold that port. `liveuse` and a serial monitor block the 1200 touch. If `lsof` shows a holder, stop and tell the user; do not kill it unless they asked.

## Run

From the firmware directory:

```sh
./upload.sh
```

Allow several minutes. The compile is most of that time. The script changes to its own directory.

## Done

Success includes one of these lines, then a bossac verify:

- `Reset al bootloader: abrir /dev/cu.usbmodem… a 1200 baudios y cerrarlo...`
- `El XIAO ya está en el bootloader.`

Then `Using XIAO port /dev/cu.usbmodem…`, `Verify successful`, and `CPU reset`.

After the script exits, confirm the sketch is back:

```sh
ioreg -p IOUSB -r -n "Seeeduino XIAO" -l | awk '/idProduct/{print}'
```

`32815` (`0x802F`) means the application is running. `47` (`0x002F`) means it is still in the bootloader; report that instead of claiming the flash finished.

## When it fails

- `No se pudo abrir … a 1200 baudios`: the CDC port is busy or gone. Report the holder from `lsof`.
- `Sigue sin puerto del XIAO`: the board is not on USB. Do not pick another serial device.
- `El XIAO no entró al bootloader`: the 1200 touch did not stick. Report the script output. Do not invent another reset.
