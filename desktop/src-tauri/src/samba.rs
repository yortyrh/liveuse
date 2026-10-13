//! SAM-BA monitor for the XIAO SAMD21. Leaves the bootloader and pulses 1200 baud.
//! bossac writes the application; this module does not.

use std::io::{self, ErrorKind};
use std::thread;
use std::time::{Duration, Instant};

use crate::device::{self, BOOTLOADER_PID, BOOTLOADER_VID, FoundPort, RUNNING_PID, RUNNING_VID};

pub const APP_OFFSET: u32 = 0x2000;
const FLASH_END: u32 = 0x40000;
pub const APP_ROOM: usize = (FLASH_END - APP_OFFSET) as usize;
const AIRCR: u32 = 0xE000_ED0C;
const SYSRESETREQ: u32 = 0x05FA_0004;
const ARDUINO_EXT: &str = "[Arduino:XYZ]";
/// Last word of the SAMD21G18A's 32KB SRAM. The Arduino bootloader's double-tap flag lives here, not in flash.
const DOUBLE_TAP_ADDR: u32 = 0x2000_7FFC;

fn cmd_write_word(addr: u32, value: u32) -> String {
    format!("W{addr:08X},{value:08X}#")
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
    use super::{APP_OFFSET, DOUBLE_TAP_ADDR, FLASH_END, cmd_write_word};

    #[test]
    fn double_tap_clear_targets_sram_not_application_flash() {
        assert!(DOUBLE_TAP_ADDR > FLASH_END);
        assert_ne!(DOUBLE_TAP_ADDR, APP_OFFSET);
        assert_eq!(cmd_write_word(DOUBLE_TAP_ADDR, 0), "W20007FFC,00000000#");
    }
}
