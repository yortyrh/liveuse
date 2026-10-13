//! Vendored [BOSSA](https://github.com/shumatech/BOSSA) `bossac`, the flasher
//! PlatformIO runs for the XIAO. The executable is compiled by `build.rs` and
//! embedded in this binary.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::device::{self, BOOTLOADER_PID, BOOTLOADER_VID};
use crate::samba::APP_OFFSET;

const BOSSAC_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/bossa-bin/bossac"));

/// Flags match PlatformIO's sam-ba upload for the Seeed SAMD21 (`-e -w -v -R -U`),
/// plus `--offset 0x2000`. BOSSA 1.9 erases from that offset; omitting it starts
/// at 0 and removes the bootloader.
pub fn flash_args(port: &str, image: &Path) -> Vec<String> {
    ["-p", port, "-U", "-e", "-w", "-v", "-R", "-o", "0x2000"]
        .into_iter()
        .map(str::to_string)
        .chain(std::iter::once(image.display().to_string()))
        .collect()
}

pub fn flash(image: &[u8], progress: &mut dyn FnMut(&str, u8)) -> Result<(), String> {
    let bin = materialize()?;
    let image_path = write_image(image)?;
    let mut last = "The bootloader port disappeared.".to_string();
    let result = (|| {
        for attempt in 1..=3 {
            progress("write", 12);
            let Some(port) = device::find_usb(BOOTLOADER_VID, BOOTLOADER_PID) else {
                last = "The bootloader port disappeared.".into();
                std::thread::sleep(Duration::from_secs(2));
                continue;
            };
            if attempt > 1 {
                std::thread::sleep(Duration::from_secs(2));
            }
            match run(&bin, &port.path, &image_path) {
                Ok(()) => return Ok(()),
                Err(message) => last = message,
            }
        }
        Err(last)
    })();
    let _ = fs::remove_file(&image_path);
    result
}

fn materialize() -> Result<PathBuf, String> {
    let path = std::env::temp_dir().join(format!("liveuse-bossac-{}", BOSSAC_BYTES.len()));
    if !path.is_file() {
        let mut file =
            fs::File::create(&path).map_err(|err| format!("could not write bossac: {err}"))?;
        file.write_all(BOSSAC_BYTES)
            .map_err(|err| format!("could not write bossac: {err}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path)
                .map_err(|err| err.to_string())?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&path, perms).map_err(|err| err.to_string())?;
        }
    }
    Ok(path)
}

fn write_image(image: &[u8]) -> Result<PathBuf, String> {
    let path = std::env::temp_dir().join(format!(
        "liveuse-flash-{}-{}.bin",
        std::process::id(),
        APP_OFFSET
    ));
    fs::write(&path, image).map_err(|err| format!("could not write the firmware image: {err}"))?;
    Ok(path)
}

fn run(bin: &Path, port: &str, image: &Path) -> Result<(), String> {
    let output = Command::new(bin)
        .args(flash_args(port, image))
        .output()
        .map_err(|err| format!("could not run bossac: {err}"))?;
    if output.status.success() {
        return Ok(());
    }
    let mut text = String::from_utf8_lossy(&output.stderr).into_owned();
    if !text.is_empty() && !output.stdout.is_empty() {
        text.push('\n');
    }
    text.push_str(String::from_utf8_lossy(&output.stdout).trim());
    let text = text.trim();
    if text.is_empty() {
        Err(format!("bossac failed ({})", output.status))
    } else {
        Err(text.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::flash_args;

    #[test]
    fn flash_starts_at_the_application_and_uses_platformio_flags() {
        let args = flash_args("/dev/cu.usbmodem1", std::path::Path::new("image.bin"));
        assert!(
            args.windows(2)
                .any(|pair| pair[0] == "-o" && pair[1] == "0x2000")
        );
        for flag in ["-U", "-e", "-w", "-v", "-R", "-p"] {
            assert!(args.iter().any(|arg| arg == flag), "{flag}");
        }
        assert_eq!(args.last().map(String::as_str), Some("image.bin"));
    }
}
