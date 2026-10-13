//! SAM-BA monitor for the XIAO SAMD21. Writes the application at 0x2000 and never the bootloader.

use std::io::{self, ErrorKind};
use std::thread;
use std::time::{Duration, Instant};

use crate::device::{self, BOOTLOADER_PID, BOOTLOADER_VID, FoundPort, RUNNING_PID, RUNNING_VID};

pub const APP_OFFSET: u32 = 0x2000;
pub const FLASH_END: u32 = 0x40000;
pub const APP_ROOM: usize = (FLASH_END - APP_OFFSET) as usize;
const ROW_SIZE: u32 = 256;
#[cfg(test)]
const PAGE_SIZE: u32 = 64;

/// SRAM bossac uses for the ATSAMD21G18. The bootloader's own data sits below this.
const SRAM_BUF: u32 = 0x2000_4000;
const CHUNK: usize = 4096;
const AIRCR: u32 = 0xE000_ED0C;
const SYSRESETREQ: u32 = 0x05FA_0004;
const ARDUINO_EXT: &str = "[Arduino:XYZ]";
/// Last word of the SAMD21G18A's 32KB SRAM. The Arduino bootloader's double-tap flag lives here, not in flash.
pub const DOUBLE_TAP_ADDR: u32 = 0x2000_7FFC;

/// Row addresses that an image would erase and write. An image that would pass 0x40000,
/// or any offset other than 0x2000, yields no addresses.
pub fn program_plan(offset: u32, image_len: usize) -> Vec<u32> {
    if offset != APP_OFFSET || image_len == 0 || image_len > APP_ROOM {
        return Vec::new();
    }
    let end = APP_OFFSET + image_len as u32;
    let mut rows = Vec::new();
    let mut addr = APP_OFFSET;
    while addr < end {
        rows.push(addr);
        addr += ROW_SIZE;
    }
    rows
}

#[cfg(test)]
pub fn page_images(offset: u32, image: &[u8]) -> Vec<(u32, [u8; 64])> {
    if program_plan(offset, image.len()).is_empty() {
        return Vec::new();
    }
    let mut pages = Vec::new();
    let mut addr = offset;
    let end = offset + image.len() as u32;
    while addr < end {
        let start = (addr - offset) as usize;
        let mut data = [0xFF; 64];
        let count = (image.len() - start).min(PAGE_SIZE as usize);
        data[..count].copy_from_slice(&image[start..start + count]);
        pages.push((addr, data));
        addr += PAGE_SIZE;
    }
    pages
}

pub fn cmd_write_word(addr: u32, value: u32) -> String {
    format!("W{addr:08X},{value:08X}#")
}

pub fn cmd_clear_double_tap() -> String {
    cmd_write_word(DOUBLE_TAP_ADDR, 0)
}

pub fn cmd_read_word(addr: u32) -> String {
    format!("w{addr:08X},4#")
}

pub fn cmd_read(addr: u32, size: u32) -> String {
    format!("R{addr:08X},{size:08X}#")
}

pub fn cmd_erase_application() -> &'static str {
    "X00002000#"
}

pub fn cmd_set_buffer() -> String {
    format!("Y{SRAM_BUF:08X},0#")
}

/// `Y` copies `size` bytes from the SRAM buffer into the application. Nothing below 0x2000.
pub fn cmd_commit(addr: u32, size: u32) -> Option<String> {
    if addr < APP_OFFSET || size == 0 || size % 4 != 0 {
        return None;
    }
    let end = addr as u64 + size as u64;
    if end > FLASH_END as u64 {
        return None;
    }
    Some(format!("Y{addr:08X},{size:08X}#"))
}

/// Pieces of the image the bootloader copies from SRAM. Empty when the image must not be written.
pub fn flash_chunks(image: &[u8]) -> Vec<(u32, Vec<u8>)> {
    if program_plan(APP_OFFSET, image.len()).is_empty() {
        return Vec::new();
    }
    let mut chunks = Vec::new();
    let mut offset = 0;
    while offset < image.len() {
        let end = (offset + CHUNK).min(image.len());
        let mut data = image[offset..end].to_vec();
        while data.len() % 4 != 0 {
            data.push(0xFF);
        }
        let addr = APP_OFFSET + offset as u32;
        if cmd_commit(addr, data.len() as u32).is_none() {
            return Vec::new();
        }
        chunks.push((addr, data));
        offset = end;
    }
    chunks
}

#[derive(Debug)]
enum SessionError {
    Lost(String),
    Rejected(String),
}

struct Samba {
    port: Box<dyn serialport::SerialPort>,
}

impl Samba {
    /// Same open bossac uses: flush, `N#` for binary mode, then `V#`. No block of
    /// zero bytes; those come back later as a `00` reply.
    fn connect_with(path: &str, raise_dtr: bool) -> Result<Self, SessionError> {
        let builder = serialport::new(path, 115_200).timeout(Duration::from_millis(50));
        let builder = if raise_dtr {
            builder
        } else {
            builder.dtr_on_open(false)
        };
        let mut port = builder
            .open()
            .map_err(|err| SessionError::Lost(format!("could not open {path}: {err}")))?;
        if raise_dtr {
            let _ = port.write_data_terminal_ready(true);
            thread::sleep(Duration::from_millis(50));
        }
        drain_until_quiet(
            port.as_mut(),
            Duration::from_millis(50),
            Duration::from_millis(300),
        );
        port.write_all(b"N#").map_err(lost)?;
        port.flush().map_err(lost)?;
        let _ = read_n(port.as_mut(), 2, Duration::from_millis(200));
        port.write_all(b"V#").map_err(lost)?;
        port.flush().map_err(lost)?;
        read_version(port.as_mut())?;
        Ok(Self { port })
    }

    fn write_cmd(&mut self, cmd: &str) -> Result<(), SessionError> {
        self.port.write_all(cmd.as_bytes()).map_err(lost)?;
        self.port.flush().map_err(lost)?;
        Ok(())
    }

    fn write_word(&mut self, addr: u32, value: u32) -> Result<(), SessionError> {
        self.write_cmd(&cmd_write_word(addr, value))
    }

    fn reset(mut self) -> Result<(), SessionError> {
        match self.write_word(AIRCR, SYSRESETREQ) {
            Ok(()) => Ok(()),
            Err(SessionError::Lost(_)) => Ok(()),
            Err(err) => Err(err),
        }
    }
}

fn lost(err: impl ToString) -> SessionError {
    SessionError::Lost(err.to_string())
}

fn timed_out(err: &std::io::Error) -> bool {
    err.kind() == ErrorKind::TimedOut || err.kind() == io::ErrorKind::WouldBlock
}

fn read_n(
    port: &mut dyn serialport::SerialPort,
    count: usize,
    budget: Duration,
) -> Result<(), SessionError> {
    let deadline = Instant::now() + budget;
    let mut left = count;
    let mut buf = [0u8; 8];
    while left > 0 {
        if Instant::now() >= deadline {
            return Err(SessionError::Lost("The bootloader timed out.".into()));
        }
        let take = left.min(buf.len());
        match port.read(&mut buf[..take]) {
            Ok(0) => continue,
            Ok(n) => left = left.saturating_sub(n),
            Err(err) if timed_out(&err) => continue,
            Err(err) => return Err(lost(err)),
        }
    }
    Ok(())
}

/// Drop bytes left by an earlier command. A word read answers with raw memory,
/// often starting at 0x00, and that byte is still here when the next command starts.
fn drain_until_quiet(port: &mut dyn serialport::SerialPort, quiet: Duration, overall: Duration) {
    let deadline = Instant::now() + overall;
    let mut last = Instant::now();
    let mut buf = [0u8; 64];
    while Instant::now() < deadline {
        match port.read(&mut buf) {
            Ok(0) => {
                if last.elapsed() >= quiet {
                    break;
                }
            }
            Ok(_) => last = Instant::now(),
            Err(err) if timed_out(&err) => {
                if last.elapsed() >= quiet {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

fn read_version(port: &mut dyn serialport::SerialPort) -> Result<String, SessionError> {
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        if Instant::now() >= deadline {
            return Err(SessionError::Rejected(
                "The bootloader did not answer.".into(),
            ));
        }
        match port.read(&mut byte) {
            Ok(0) => continue,
            Ok(_) if byte[0] == b'\r' => continue,
            Ok(_) if byte[0] == b'\n' => {
                let line = String::from_utf8_lossy(&buf).trim().to_string();
                buf.clear();
                if line.contains(ARDUINO_EXT) {
                    return Ok(line);
                }
            }
            Ok(_) => {
                buf.push(byte[0]);
                if buf.len() > 80 {
                    buf.clear();
                }
            }
            Err(err) if timed_out(&err) => continue,
            Err(err) => return Err(lost(err)),
        }
    }
}

/// Clear the double-tap latch and reset. A valid sketch starts. A blank chip stays in the bootloader.
/// DTR stays low and the baud rate stays 115200, so this is not a 1200-baud touch.
pub fn leave_bootloader(path: &str) -> Result<(), String> {
    let mut samba = Samba::connect_with(path, false).map_err(session_message)?;
    samba
        .write_word(DOUBLE_TAP_ADDR, 0)
        .map_err(session_message)?;
    samba.reset().map_err(session_message)?;
    Ok(())
}

fn session_message(err: SessionError) -> String {
    match err {
        SessionError::Lost(message) | SessionError::Rejected(message) => message,
    }
}

pub fn pulse_1200(path: &str) -> Result<(), String> {
    let mut port = serialport::new(path, 1200)
        .timeout(Duration::from_millis(50))
        .open()
        .map_err(|err| format!("could not open {path} at 1200 baud: {err}"))?;
    port.write_data_terminal_ready(true)
        .map_err(|err| format!("could not raise DTR: {err}"))?;
    thread::sleep(Duration::from_millis(50));
    port.write_data_terminal_ready(false)
        .map_err(|err| format!("could not drop DTR: {err}"))?;
    drop(port);
    thread::sleep(Duration::from_millis(400));
    Ok(())
}

pub fn wait_running(timeout: Duration) -> Result<FoundPort, String> {
    device::wait_usb(RUNNING_VID, RUNNING_PID, timeout)
        .ok_or_else(|| "The board did not return after the write.".into())
}

pub fn wait_bootloader(timeout: Duration) -> Result<FoundPort, String> {
    device::wait_usb(BOOTLOADER_VID, BOOTLOADER_PID, timeout)
        .ok_or_else(|| "The board did not enter the bootloader.".into())
}

#[cfg(test)]
mod tests {
    use super::{
        APP_OFFSET, DOUBLE_TAP_ADDR, FLASH_END, cmd_clear_double_tap, cmd_commit,
        cmd_erase_application, cmd_read, cmd_read_word, cmd_set_buffer, cmd_write_word,
        flash_chunks, page_images, program_plan,
    };

    #[test]
    fn image_of_300_bytes_uses_rows_0x2000_and_0x2100() {
        let rows = program_plan(APP_OFFSET, 300);
        assert_eq!(rows, vec![0x2000, 0x2100]);
        assert!(rows.iter().all(|addr| *addr >= 0x2000 && *addr < 0x2200));

        let image = vec![0x11u8; 300];
        let pages = page_images(APP_OFFSET, &image);
        assert!(
            pages
                .iter()
                .all(|(addr, _)| *addr >= 0x2000 && *addr < 0x2200)
        );
        assert!(pages.iter().all(|(addr, data)| {
            data.iter().enumerate().all(|(index, byte)| {
                let absolute = *addr as usize + index;
                if absolute < 0x2000 + 300 {
                    *byte == 0x11
                } else {
                    *byte == 0xFF
                }
            })
        }));
    }

    #[test]
    fn double_tap_clear_targets_sram_not_application_flash() {
        assert!(DOUBLE_TAP_ADDR > FLASH_END);
        assert_ne!(DOUBLE_TAP_ADDR, APP_OFFSET);
        assert_eq!(cmd_clear_double_tap(), "W20007FFC,00000000#");
    }

    #[test]
    fn image_past_flash_or_other_offset_has_no_addresses() {
        assert!(program_plan(APP_OFFSET, 0x40000).is_empty());
        assert!(program_plan(APP_OFFSET, (0x40000 - 0x2000) + 1).is_empty());
        assert!(program_plan(0x0, 300).is_empty());
        assert!(program_plan(0x1000, 300).is_empty());
        assert!(page_images(0x0, &[0u8; 300]).is_empty());
    }

    #[test]
    fn monitor_commands_are_ascii() {
        assert_eq!(cmd_write_word(0x2000, 0xAABB_CCDD), "W00002000,AABBCCDD#");
        assert_eq!(cmd_read_word(0x4100_4014), "w41004014,4#");
        assert_eq!(cmd_read(0x2000, 64), "R00002000,00000040#");
        assert_eq!(cmd_erase_application(), "X00002000#");
        assert_eq!(cmd_set_buffer(), "Y20004000,0#");
        assert_eq!(cmd_commit(0x2000, 300), Some("Y00002000,0000012C#".into()));
        assert_eq!(cmd_commit(0, 256), None);
    }

    #[test]
    fn image_of_300_bytes_is_one_commit_at_0x2000() {
        let chunks = flash_chunks(&[0x11u8; 300]);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].0, 0x2000);
        assert_eq!(chunks[0].1.len(), 300);
        assert!(
            chunks.iter().all(|(addr, data)| {
                *addr >= 0x2000 && (*addr as usize + data.len()) <= 0x40000
            })
        );
    }
}
