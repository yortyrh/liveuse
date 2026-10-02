#!/usr/bin/env python3
"""Run bossac after the XIAO bootloader USB settles. Retry a failed page write."""
import subprocess
import sys
import time

SETTLE_S = 2.0
ATTEMPTS = 3


def main() -> int:
    cmd = sys.argv[1:]
    if not cmd:
        print("usage: upload_bossac.py <bossac> [args...]", file=sys.stderr)
        return 2

    print(f"Waiting {SETTLE_S:.0f}s for the bootloader USB to settle...")
    time.sleep(SETTLE_S)

    last = 1
    for attempt in range(1, ATTEMPTS + 1):
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
