use std::io::{self, ErrorKind};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::oneshot;

use crate::protocol::{DeviceStatus, parse_status};

pub const RUNNING_VID: u16 = 0x2886;
pub const RUNNING_PID: u16 = 0x802F;
pub const BOOTLOADER_VID: u16 = 0x2886;
pub const BOOTLOADER_PID: u16 = 0x002F;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoundPort {
    pub path: String,
    pub serial_number: Option<String>,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
}

/// `Some(true)` when the USB strings are LiveUse, `Some(false)` when they name
/// another program, `None` when the host did not report either string.
pub fn liveuse_usb(manufacturer: Option<&str>, product: Option<&str>) -> Option<bool> {
    let present = [manufacturer, product]
        .into_iter()
        .flatten()
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>();
    if present.is_empty() {
        return None;
    }
    Some(present.iter().any(|text| text.contains("LiveUse")))
}

pub fn find_running() -> Option<FoundPort> {
    find_usb(RUNNING_VID, RUNNING_PID)
}

pub fn find_usb(vid: u16, pid: u16) -> Option<FoundPort> {
    let ports = serialport::available_ports().ok()?;
    let mut found = Vec::new();
    for port in ports {
        if let serialport::SerialPortType::UsbPort(info) = port.port_type
            && info.vid == vid
            && info.pid == pid
        {
            found.push(FoundPort {
                path: port.port_name,
                serial_number: info.serial_number,
                manufacturer: info.manufacturer,
                product: info.product,
            });
        }
    }
    found.sort_by_key(|port| callout_rank(&port.path));
    found.into_iter().next()
}

pub fn wait_usb(vid: u16, pid: u16, timeout: Duration) -> Option<FoundPort> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(port) = find_usb(vid, pid) {
            return Some(port);
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn callout_rank(path: &str) -> u8 {
    u8::from(path.contains("/tty."))
}

#[derive(Debug)]
pub enum OpenFailure {
    Unavailable(String),
    NotLiveUse,
}

enum Job {
    Open {
        path: String,
        reply: oneshot::Sender<Result<DeviceStatus, OpenFailure>>,
    },
    Close,
    Line {
        line: String,
        silent_timeout: Option<Duration>,
        reply: oneshot::Sender<Result<String, String>>,
    },
}

#[derive(Clone)]
pub struct Device {
    tx: Sender<Job>,
    path: Arc<Mutex<Option<String>>>,
    broken: Arc<AtomicBool>,
    release: Arc<AtomicBool>,
}

impl Device {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<Job>();
        let path = Arc::new(Mutex::new(None));
        let broken = Arc::new(AtomicBool::new(false));
        let release = Arc::new(AtomicBool::new(false));
        let path_worker = path.clone();
        let broken_worker = broken.clone();
        let release_worker = release.clone();
        std::thread::Builder::new()
            .name("liveuse-cdc".into())
            .spawn(move || worker(rx, path_worker, broken_worker, release_worker))
            .expect("CDC thread");
        Self {
            tx,
            path,
            broken,
            release,
        }
    }

    pub fn path(&self) -> Option<String> {
        self.path.lock().ok().and_then(|guard| guard.clone())
    }

    pub fn is_broken(&self) -> bool {
        self.broken.load(Ordering::Relaxed)
    }

    pub async fn open(&self, path: &str) -> Result<DeviceStatus, OpenFailure> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Job::Open {
                path: path.to_string(),
                reply,
            })
            .map_err(|_| OpenFailure::Unavailable("serial port closed".into()))?;
        rx.await
            .map_err(|_| OpenFailure::Unavailable("serial port closed".into()))?
    }

    /// Drop our CDC handle. The flag is set before the job is queued so a read
    /// already in progress returns instead of holding the port until it times out.
    pub fn close(&self) {
        self.release.store(true, Ordering::SeqCst);
        let _ = self.tx.send(Job::Close);
    }

    pub async fn transact(
        &self,
        line: String,
        silent_timeout: Option<Duration>,
    ) -> Result<String, String> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Job::Line {
                line,
                silent_timeout,
                reply,
            })
            .map_err(|_| "device not connected".to_string())?;
        rx.await.map_err(|_| "device not connected".to_string())?
    }
}

fn worker(
    rx: mpsc::Receiver<Job>,
    path: Arc<Mutex<Option<String>>>,
    broken: Arc<AtomicBool>,
    release: Arc<AtomicBool>,
) {
    let mut port: Option<Box<dyn serialport::SerialPort>> = None;
    while let Ok(job) = rx.recv() {
        match job {
            Job::Close => {
                port = None;
                if let Ok(mut guard) = path.lock() {
                    *guard = None;
                }
                broken.store(false, Ordering::Relaxed);
                release.store(false, Ordering::SeqCst);
            }
            Job::Open { path: next, reply } => {
                port = None;
                let opened =
                    open_port(&next)
                        .map_err(OpenFailure::Unavailable)
                        .and_then(|mut serial| {
                            drain(serial.as_mut(), Duration::from_millis(400), &release);
                            write_line(serial.as_mut(), "STATUS")
                                .map_err(OpenFailure::Unavailable)?;
                            let line = wait_match(
                                serial.as_mut(),
                                Duration::from_secs(2),
                                &release,
                                |line| parse_status(line).is_some(),
                            )
                            .map_err(|err| {
                                if err == "device timed out" {
                                    OpenFailure::NotLiveUse
                                } else {
                                    OpenFailure::Unavailable(err)
                                }
                            })?;
                            let status = parse_status(&line).ok_or(OpenFailure::NotLiveUse)?;
                            Ok((serial, status))
                        });
                match opened {
                    Ok((serial, status)) => {
                        port = Some(serial);
                        if let Ok(mut guard) = path.lock() {
                            *guard = Some(next);
                        }
                        broken.store(false, Ordering::Relaxed);
                        let _ = reply.send(Ok(status));
                    }
                    Err(err) => {
                        if let Ok(mut guard) = path.lock() {
                            *guard = None;
                        }
                        broken.store(true, Ordering::Relaxed);
                        let _ = reply.send(Err(err));
                    }
                }
            }
            Job::Line {
                line,
                silent_timeout,
                reply,
            } => {
                let Some(serial) = port.as_mut() else {
                    let _ = reply.send(Err("device not connected".into()));
                    continue;
                };
                let result = transact_line(serial.as_mut(), &line, silent_timeout, &release);
                if result.is_err() {
                    broken.store(true, Ordering::Relaxed);
                }
                let _ = reply.send(result);
            }
        }
    }
}

fn open_port(path: &str) -> Result<Box<dyn serialport::SerialPort>, String> {
    let mut port = serialport::new(path, 115_200)
        .timeout(Duration::from_millis(30))
        .open()
        .map_err(|err| format!("no se pudo abrir {path}: {err}"))?;
    let _ = port.write_data_terminal_ready(true);
    Ok(port)
}

fn write_line(port: &mut dyn serialport::SerialPort, line: &str) -> Result<(), String> {
    port.write_all(format!("{line}\n").as_bytes())
        .map_err(|err| err.to_string())?;
    port.flush().map_err(|err| err.to_string())?;
    Ok(())
}

/// STATUS answers `OK uptime_ms=… fw=…` and does not end in "done".
/// Every other command finishes on "done" or "ERR".
fn reply_completes(command: &str, line: &str) -> bool {
    if line.starts_with("ERR") || line.ends_with(" done") {
        return true;
    }
    let verb = command.split_whitespace().next().unwrap_or("");
    verb == "STATUS" && parse_status(line).is_some()
}

fn transact_line(
    port: &mut dyn serialport::SerialPort,
    line: &str,
    silent_timeout: Option<Duration>,
    release: &AtomicBool,
) -> Result<String, String> {
    write_line(port, line)?;
    if let Some(timeout) = silent_timeout {
        let deadline = Instant::now() + timeout;
        loop {
            match read_line(port, deadline, release)? {
                None => return Ok(String::new()),
                Some(line) if line.starts_with("ERR") => return Err(line),
                Some(_) => continue,
            }
        }
    }
    wait_match(port, Duration::from_secs(8), release, |reply| {
        reply_completes(line, reply)
    })
    .and_then(|reply| {
        if reply.starts_with("ERR") {
            Err(reply)
        } else {
            Ok(reply)
        }
    })
}

fn wait_match(
    port: &mut dyn serialport::SerialPort,
    budget: Duration,
    release: &AtomicBool,
    pred: impl Fn(&str) -> bool,
) -> Result<String, String> {
    let deadline = Instant::now() + budget;
    loop {
        match read_line(port, deadline, release)? {
            None => return Err("device timed out".into()),
            Some(line) if pred(&line) => return Ok(line),
            Some(_) => continue,
        }
    }
}

fn drain(port: &mut dyn serialport::SerialPort, budget: Duration, release: &AtomicBool) {
    let deadline = Instant::now() + budget;
    while Instant::now() < deadline {
        if release.load(Ordering::Relaxed) {
            break;
        }
        if read_line(port, Instant::now() + Duration::from_millis(40), release)
            .ok()
            .flatten()
            .is_none()
        {
            break;
        }
    }
}

fn read_line(
    port: &mut dyn serialport::SerialPort,
    deadline: Instant,
    release: &AtomicBool,
) -> Result<Option<String>, String> {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        if release.load(Ordering::Relaxed) {
            return Err("closed".into());
        }
        if Instant::now() >= deadline {
            return Ok(None);
        }
        match port.read(&mut byte) {
            Ok(0) => continue,
            Ok(_) if byte[0] == b'\r' => continue,
            Ok(_) if byte[0] == b'\n' => {
                return Ok(Some(String::from_utf8_lossy(&buf).trim().to_string()));
            }
            Ok(_) => buf.push(byte[0]),
            Err(err)
                if err.kind() == ErrorKind::TimedOut || err.kind() == io::ErrorKind::WouldBlock =>
            {
                continue;
            }
            Err(err) => return Err(err.to_string()),
        }
        if buf.len() > 512 {
            return Err("CDC line too long".into());
        }
    }
}

/// Identification open. 115200 baud with DTR high, the same open the status
/// poll uses. The bootloader touch is a separate 1200-baud open whose DTR fall
/// resets the board; asserting DTR here does not.
pub fn read_firmware_version(path: &str) -> Result<Option<String>, String> {
    let mut port = serialport::new(path, 115_200)
        .timeout(Duration::from_millis(30))
        .open()
        .map_err(|err| format!("could not open {path}: {err}"))?;
    let _ = port.write_data_terminal_ready(true);
    let release = AtomicBool::new(false);
    drain(port.as_mut(), Duration::from_millis(400), &release);
    write_line(port.as_mut(), "STATUS")?;
    match wait_match(port.as_mut(), Duration::from_secs(2), &release, |line| {
        parse_status(line).is_some()
    }) {
        Ok(line) => Ok(parse_status(&line).map(|status| status.firmware)),
        Err(err) if err == "device timed out" => Ok(None),
        Err(err) => Err(err),
    }
}

/// After a reset the sketch may not answer STATUS on the first open.
pub fn read_firmware_version_settled(path: &str) -> Result<Option<String>, String> {
    let deadline = Instant::now() + Duration::from_secs(4);
    let mut opened = false;
    let mut last_open = String::new();
    while Instant::now() < deadline {
        match read_firmware_version(path) {
            Ok(Some(version)) => return Ok(Some(version)),
            Ok(None) => opened = true,
            Err(err) => last_open = err,
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    if opened { Ok(None) } else { Err(last_open) }
}

pub fn sibling_callout(path: &str) -> Option<String> {
    if let Some(rest) = path.strip_prefix("/dev/cu.") {
        Some(format!("/dev/tty.{rest}"))
    } else {
        path.strip_prefix("/dev/tty.")
            .map(|rest| format!("/dev/cu.{rest}"))
    }
}

pub fn parse_lsof(output: &str, skip_pids: &[u32]) -> Vec<String> {
    let mut names = Vec::new();
    let mut pid = None;
    let mut command = None;
    let remember =
        |pid: &mut Option<u32>, command: &mut Option<String>, names: &mut Vec<String>| {
            let Some(pid) = pid.take() else {
                *command = None;
                return;
            };
            let Some(command) = command.take() else {
                return;
            };
            if skip_pids.contains(&pid) || names.iter().any(|have| have == &command) {
                return;
            }
            names.push(command);
        };
    for line in output.lines() {
        let Some(kind) = line.chars().next() else {
            continue;
        };
        let value = &line[kind.len_utf8()..];
        match kind {
            'p' => {
                remember(&mut pid, &mut command, &mut names);
                pid = value.parse().ok();
            }
            'c' => command = Some(value.to_string()),
            _ => {}
        }
    }
    remember(&mut pid, &mut command, &mut names);
    names
}

pub fn port_holders(path: &str) -> Vec<String> {
    let mut paths = vec![path.to_string()];
    if let Some(sibling) = sibling_callout(path) {
        paths.push(sibling);
    }
    let output = Command::new("lsof")
        .args(["-n", "-P", "-F", "pc"])
        .args(&paths)
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    parse_lsof(&text, &[std::process::id()])
}

fn join_names(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [first, second] => format!("{first} and {second}"),
        many => {
            let last = many.last().map_or("", String::as_str);
            format!("{}, and {last}", many[..many.len() - 1].join(", "))
        }
    }
}

pub fn holders_error(paths: &[String]) -> Option<String> {
    let mut names = Vec::new();
    for path in paths {
        for name in port_holders(path) {
            if !names.iter().any(|have: &String| have == &name) {
                names.push(name);
            }
        }
    }
    if names.is_empty() {
        None
    } else {
        Some(format!(
            "Can't flash: {} has the board port open. Close it and try again.",
            join_names(&names)
        ))
    }
}

fn serials_compatible(before: Option<&str>, after: Option<&str>) -> bool {
    match (before, after) {
        (Some(left), Some(right)) => left == right,
        _ => true,
    }
}

fn endpoint_present(path: &str) -> bool {
    [find_running(), find_usb(BOOTLOADER_VID, BOOTLOADER_PID)]
        .into_iter()
        .flatten()
        .any(|port| port.path == path)
}

/// After a software reset, wait until the previous USB device drops and the same
/// chip comes back as the sketch or the bootloader. A port that never drops did
/// not reset.
pub fn wait_reenumerate(previous: &FoundPort) -> Result<(), String> {
    let drop_deadline = Instant::now() + Duration::from_secs(3);
    let mut dropped = false;
    while Instant::now() < drop_deadline {
        if !endpoint_present(&previous.path) {
            dropped = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    if !dropped {
        return Err("The board did not reset.".into());
    }
    let back_deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < back_deadline {
        let found = find_running().or_else(|| find_usb(BOOTLOADER_VID, BOOTLOADER_PID));
        if let Some(port) = found
            && serials_compatible(
                previous.serial_number.as_deref(),
                port.serial_number.as_deref(),
            )
        {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err("The board did not return.".into())
}

#[cfg(test)]
mod tests {
    use super::{liveuse_usb, parse_lsof, reply_completes, sibling_callout};

    #[test]
    fn status_reply_completes_without_done() {
        assert!(reply_completes("STATUS", "OK uptime_ms=1519 fw=0.5.1"));
        assert!(!reply_completes("MOVE 1 2", "OK uptime_ms=1519 fw=0.5.1"));
        assert!(reply_completes("MOVE 1 2", "MOVE done"));
        assert!(reply_completes("STATUS", "ERR STATUS bad"));
    }

    #[test]
    fn usb_strings_distinguish_liveuse() {
        assert_eq!(
            liveuse_usb(Some("LiveUse"), Some("LiveUse 0.5.1")),
            Some(true)
        );
        assert_eq!(liveuse_usb(None, Some("Seeeduino XIAO")), Some(false));
        assert_eq!(liveuse_usb(None, None), None);
        assert_eq!(liveuse_usb(Some(""), Some("")), None);
    }

    #[test]
    fn callout_sibling_swaps_cu_and_tty() {
        assert_eq!(
            sibling_callout("/dev/cu.usbmodem123"),
            Some("/dev/tty.usbmodem123".into())
        );
        assert_eq!(
            sibling_callout("/dev/tty.usbmodem123"),
            Some("/dev/cu.usbmodem123".into())
        );
        assert_eq!(sibling_callout("COM3"), None);
    }

    #[test]
    fn lsof_names_skip_our_pid_and_repeat() {
        let output = "\
p10
cScreen
p20
cpio
p20
cpio
p30
cLiveUse
";
        assert_eq!(parse_lsof(output, &[10, 30]), vec!["pio".to_string()]);
    }
}
