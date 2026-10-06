#!/usr/bin/env python3
"""Run bossac after the XIAO bootloader USB settles. Retry a failed page write."""
import os
import subprocess
import sys
import time

SETTLE_S = 2.0
ATTEMPTS = 3


def xiao_ports():
    found = []
    try:
        names = os.listdir("/dev")
    except OSError:
        return found
    for name in names:
        low = name.lower()
        if not name.startswith("cu."):
            continue
        if "bluetooth" in low or low.startswith("cu.blth") or "incoming-port" in low:
            continue
        if "usbmodem" in low or "usbserial" in low:
            found.append(name)
    return sorted(found)


def retarget(cmd):
    ports = xiao_ports()
    if not ports:
        return cmd
    port = ports[0]
    out = []
    i = 0
    replaced = False
    while i < len(cmd):
        arg = cmd[i]
        if arg == "--port" and i + 1 < len(cmd):
            current = cmd[i + 1].strip('"')
            if not os.path.exists("/dev/" + current):
                print(f"bossac port {current} missing, using {port}")
                out.extend(["--port", port])
                replaced = True
                i += 2
                continue
        out.append(arg)
        i += 1
    if not replaced and "--port" not in cmd:
        out.extend(["--port", port])
    return out


def main() -> int:
    cmd = sys.argv[1:]
    if not cmd:
        print("usage: upload_bossac.py <bossac> [args...]", file=sys.stderr)
        return 2

    print(f"Waiting {SETTLE_S:.0f}s for the bootloader USB to settle...")
    time.sleep(SETTLE_S)

    last = 1
    for attempt in range(1, ATTEMPTS + 1):
        cmd = retarget(cmd)
        print(f"bossac attempt {attempt}/{ATTEMPTS}")
        last = subprocess.call(cmd)
        if last == 0:
            return 0
        if attempt < ATTEMPTS:
            print("Page write failed. Waiting for the bootloader USB, then retry.")
            time.sleep(SETTLE_S)
    return last


if __name__ == "__main__":
    sys.exit(main())
