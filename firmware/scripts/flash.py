#!/usr/bin/env python3
"""Compile and flash the connected XIAO. The caller never names a port.

pio resets the board. The seeed_xiao manifest has upload.use_1200bps_touch,
so the upload opens the port at 1200 baud and waits for the bootloader.
This script only checks that one board is connected and its port is free.
"""

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

FIRMWARE = Path(__file__).resolve().parent.parent
ENV = "seeed_xiao"


def xiao_ports():
    found = []
    try:
        names = os.listdir("/dev")
    except OSError:
        return found
    for name in names:
        if not name.startswith("cu."):
            continue
        low = name.lower()
        if "bluetooth" in low or low.startswith("cu.blth") or "incoming-port" in low:
            continue
        if "usbmodem" in low or "usbserial" in low:
            found.append("/dev/" + name)
    return sorted(found)


def port_holders(port):
    """Programs that already have the board's serial device open."""
    paths = [port]
    sibling = port.replace("/cu.", "/tty.", 1)
    if sibling != port:
        paths.append(sibling)
    try:
        proc = subprocess.run(
            ["lsof", "-n", "-P", "-F", "pc", *paths],
            capture_output=True,
            text=True,
        )
    except OSError:
        return []
    names = []
    pid = None
    command = None
    me = {os.getpid(), os.getppid()}

    def remember():
        if pid in me or not command:
            return
        if command not in names:
            names.append(command)

    for line in proc.stdout.splitlines():
        if not line:
            continue
        kind, value = line[0], line[1:]
        if kind == "p":
            remember()
            pid = int(value) if value.isdigit() else None
            command = None
        elif kind == "c":
            command = value
    remember()
    return names


def join_names(names):
    if len(names) == 1:
        return names[0]
    if len(names) == 2:
        return "%s and %s" % (names[0], names[1])
    return "%s, and %s" % (", ".join(names[:-1]), names[-1])


def fail(message):
    print(message, file=sys.stderr)
    raise SystemExit(1)


def require_board():
    ports = xiao_ports()
    if not ports:
        fail(
            "Can't find the board. Connect it over USB and run this command again.\n"
            "If it is already connected, short the two reset contacts (RST) twice in a row "
            "and try again."
        )
    if len(ports) > 1:
        fail(
            "More than one board port is connected. Leave only the XIAO you want to flash "
            "and run this command again."
        )
    port = ports[0]
    holders = port_holders(port)
    if holders:
        fail(
            "Can't flash: %s has the board port open. "
            "Close it and run this command again." % join_names(holders)
        )
    return port


def run_pio(args):
    return subprocess.call(["pio", *args], cwd=FIRMWARE)


def main(argv=None):
    parser = argparse.ArgumentParser(
        prog="flash",
        description=(
            "Compile the firmware and flash it onto the board connected over USB. "
            "You do not name the port. pio resets the board when flashing."
        ),
    )
    parser.parse_args(argv)

    print("Looking for the board...")
    require_board()
    print("Found the board.")

    if shutil.which("pio") is None:
        fail(
            "Can't find pio (PlatformIO). Install it and run this command again."
        )

    print("Compiling...")
    if run_pio(["run", "-e", ENV]) != 0:
        fail("The build failed. Nothing was flashed.")

    port = require_board()
    print("Flashing...")
    if run_pio(["run", "-e", ENV, "-t", "upload", "--upload-port", port]) != 0:
        fail("Flashing failed.")
    print("Done. The firmware is on the board.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
