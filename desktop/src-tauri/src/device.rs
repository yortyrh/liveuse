use std::io::{self, ErrorKind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::oneshot;

use crate::protocol::{DeviceStatus, parse_status};

const VID: u16 = 0x2886;
const PID: u16 = 0x802F;

pub fn find_xiao() -> Option<String> {
    let ports = serialport::available_ports().ok()?;
    for port in ports {
        if let serialport::SerialPortType::UsbPort(info) = port.port_type {
            if info.vid == VID && info.pid == PID {
                return Some(port.port_name);
            }
        }
    }
    None
}

enum Job {
    Open {
        path: String,
        reply: oneshot::Sender<Result<DeviceStatus, String>>,
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
}

impl Device {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<Job>();
        let path = Arc::new(Mutex::new(None));
        let broken = Arc::new(AtomicBool::new(false));
        let path_worker = path.clone();
        let broken_worker = broken.clone();
        std::thread::Builder::new()
            .name("liveuse-cdc".into())
            .spawn(move || worker(rx, path_worker, broken_worker))
            .expect("CDC thread");
        Self { tx, path, broken }
    }

    pub fn path(&self) -> Option<String> {
        self.path.lock().ok().and_then(|guard| guard.clone())
    }

    pub fn is_broken(&self) -> bool {
        self.broken.load(Ordering::Relaxed)
    }

    pub async fn open(&self, path: &str) -> Result<DeviceStatus, String> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(Job::Open {
                path: path.to_string(),
                reply,
            })
            .map_err(|_| "serial port closed".to_string())?;
        rx.await.map_err(|_| "serial port closed".to_string())?
    }

    pub fn close(&self) {
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

fn worker(rx: mpsc::Receiver<Job>, path: Arc<Mutex<Option<String>>>, broken: Arc<AtomicBool>) {
    let mut port: Option<Box<dyn serialport::SerialPort>> = None;
    while let Ok(job) = rx.recv() {
        match job {
            Job::Close => {
                port = None;
                if let Ok(mut guard) = path.lock() {
                    *guard = None;
                }
                broken.store(false, Ordering::Relaxed);
            }
            Job::Open { path: next, reply } => {
                port = None;
                let opened = open_port(&next).and_then(|mut serial| {
                    drain(serial.as_mut(), Duration::from_millis(400));
                    write_line(serial.as_mut(), "STATUS")?;
                    let line = wait_match(serial.as_mut(), Duration::from_secs(2), |line| {
                        parse_status(line).is_some()
                    })?;
                    let status =
                        parse_status(&line).ok_or_else(|| format!("STATUS inesperado: {line}"))?;
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
                let result = transact_line(serial.as_mut(), &line, silent_timeout);
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

fn transact_line(
    port: &mut dyn serialport::SerialPort,
    line: &str,
    silent_timeout: Option<Duration>,
) -> Result<String, String> {
    write_line(port, line)?;
    if let Some(timeout) = silent_timeout {
        let deadline = Instant::now() + timeout;
        loop {
            match read_line(port, deadline)? {
                None => return Ok(String::new()),
                Some(line) if line.starts_with("ERR") => return Err(line),
                Some(_) => continue,
            }
        }
    }
    wait_match(port, Duration::from_secs(8), |line| {
        line.starts_with("ERR") || line.ends_with(" done")
    })
    .and_then(|line| {
        if line.starts_with("ERR") {
            Err(line)
        } else {
            Ok(line)
        }
    })
}

fn wait_match(
    port: &mut dyn serialport::SerialPort,
    budget: Duration,
    pred: impl Fn(&str) -> bool,
) -> Result<String, String> {
    let deadline = Instant::now() + budget;
    loop {
        match read_line(port, deadline)? {
            None => return Err("device timed out".into()),
            Some(line) if pred(&line) => return Ok(line),
            Some(_) => continue,
        }
    }
}

fn drain(port: &mut dyn serialport::SerialPort, budget: Duration) {
    let deadline = Instant::now() + budget;
    while Instant::now() < deadline {
        if read_line(port, Instant::now() + Duration::from_millis(40))
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
) -> Result<Option<String>, String> {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
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
