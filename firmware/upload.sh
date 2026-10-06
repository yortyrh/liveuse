#!/bin/sh
# Compile, then flash. settle_upload.py opens the CDC port at 1200 baud
# and closes it so the XIAO reboots into the SAM-BA bootloader.
set -eu
cd "$(dirname "$0")"

echo "Compilando..."
pio run -e seeed_xiao

exec pio run -e seeed_xiao -t upload
