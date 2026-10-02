# After the 1200 bps reset, macOS mounts the XIAO bootloader disk and
# bossac's page write dies with "SAM-BA operation failed".
# Sleep once the port exists, then retry the write.
Import("env")

_orig = env["UPLOADCMD"]
if not isinstance(_orig, str):
    _orig = " ".join(_orig)

env.Replace(
    UPLOADCMD='python3 "$PROJECT_DIR/scripts/upload_bossac.py" ' + _orig
)
