# Pick the XIAO port before PlatformIO autodetects Bluetooth.
# Opening that CDC port at 1200 baud and closing it reboots the sketch
# into the SAM-BA bootloader. Then retry bossac once USB settles.
Import("env")

import os
import subprocess
import time



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


def wait_port(seconds):
    deadline = time.time() + seconds
    while True:
        found = xiao_ports()
        if found:
            return found[0]
        if time.time() >= deadline:
            return None
        time.sleep(0.25)


# Seeed XIAO SAMD21: 32815 is the running sketch (0x802F), 47 is SAM-BA (0x002F).
BOOTLOADER_PRODUCT = 47


def xiao_product():
    try:
        out = subprocess.check_output(
            ["ioreg", "-p", "IOUSB", "-r", "-n", "Seeeduino XIAO", "-l"],
            text=True,
            stderr=subprocess.DEVNULL,
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    for line in out.splitlines():
        line = line.strip()
        if line.startswith('"idProduct"'):
            try:
                return int(line.split("=", 1)[1])
            except ValueError:
                return None
    return None


def open_1200(port):
    # Opening CDC at 1200 baud and dropping DTR is the bootloader signal.
    # TinyUSB resets only when DTR falls while the line rate is already 1200.
    try:
        import serial
    except ImportError:
        print("Falta pyserial para el reset a 1200 baudios.")
        return False
    print("Reset al bootloader: abrir %s a 1200 baudios y cerrarlo..." % port)
    try:
        link = serial.Serial(port=port, baudrate=1200)
        time.sleep(0.05)
        link.setDTR(False)
        link.close()
    except Exception as exc:  # pylint: disable=broad-except
        print("No se pudo abrir %s a 1200 baudios: %s" % (port, exc))
        return False
    time.sleep(0.4)
    return True


def before_upload(source, target, env):  # pylint: disable=unused-argument
    # This hook picks the XIAO port and does the 1200 baud touch itself.
    # PlatformIO's own touch would also run, and it can grab Bluetooth.
    board = env.BoardConfig()
    board.update("upload.use_1200bps_touch", False)
    board.update("upload.wait_for_upload_port", False)
    board.update("upload.disable_flushing", True)
    if xiao_product() == BOOTLOADER_PRODUCT:
        port = xiao_ports()[0] if xiao_ports() else wait_port(8)
        if port:
            print("El XIAO ya está en el bootloader.")
    else:
        ports = xiao_ports()
        if not ports:
            print("Sigue sin puerto del XIAO. No subo a Bluetooth.")
            env.Exit(1)
        if not open_1200(ports[0]):
            env.Exit(1)
        deadline = time.time() + 8
        while time.time() < deadline and xiao_product() != BOOTLOADER_PRODUCT:
            time.sleep(0.1)
        if xiao_product() != BOOTLOADER_PRODUCT:
            print("El XIAO no entró al bootloader tras cerrar el puerto a 1200 baudios.")
            env.Exit(1)
        port = wait_port(8)
    if not port:
        print("Sigue sin puerto del XIAO. No subo a Bluetooth.")
        env.Exit(1)
    print("Using XIAO port", port)
    env.Replace(UPLOAD_PORT=port)


if not env.get("LIVEUSE_UPLOAD_HOOK"):
    env["LIVEUSE_UPLOAD_HOOK"] = True
    env.AddPreAction("upload", before_upload)

cmd = env.get("UPLOADCMD")

# Post script only (UPLOADCMD exists after the platform script). Raise
# TinyUSB's HID slot count without forcing the header onto the Arduino core.
if cmd and not env.get("LIVEUSE_HID_COUNT"):
    env["LIVEUSE_HID_COUNT"] = True
    Import("projenv")
    hid_header = os.path.join(env.subst("$PROJECT_DIR"), "include", "hid_count.h")

    def force_two_hid(target_env):
        if "hid_count.h" in str(target_env.get("CCFLAGS", "")):
            return
        target_env.Append(CCFLAGS=["-include", hid_header])

    force_two_hid(projenv)
    for lb in env.GetLibBuilders():
        label = "%s %s" % (lb.name, lb.path)
        if "TinyUSB" in label:
            force_two_hid(lb.env)
            print("CFG_TUD_HID=2 for", lb.name)
if cmd:
    text = cmd if isinstance(cmd, str) else " ".join(str(part) for part in cmd)
    uploader = env.get("UPLOADER", "")
    uploader_text = uploader if isinstance(uploader, str) else " ".join(str(part) for part in uploader)
    if "upload_bossac.py" not in text and ("bossac" in text or "bossac" in uploader_text):
        env.Replace(
            UPLOADCMD='python3 "$PROJECT_DIR/scripts/upload_bossac.py" ' + text
        )
