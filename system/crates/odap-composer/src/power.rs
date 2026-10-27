//! Requests to the power service (blackbird-os `odap-power.socket`): the composer's user cannot power
//! off or pick a boot mode itself; the service, as root, runs `odap-reboot` for it.
//!
//! Nothing here blocks the caller (T1-047): power-off and reboot are sent from a thread of their own,
//! and the boot modes are fetched in the background and answered from a cache.

use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread::JoinHandle,
    time::Duration,
};

use tracing::warn;

#[cfg(target_os = "linux")]
const SOCKET: &str = opendap_platform::run::POWER_SOCKET;
const TIMEOUT: Duration = Duration::from_secs(2);

/// The power service at one socket path.
pub struct PowerClient {
    socket: PathBuf,
    modes: Arc<Mutex<Option<Vec<String>>>>,
}

/// The client of the device's power service.
#[cfg(target_os = "linux")]
pub fn client() -> &'static PowerClient {
    static CLIENT: std::sync::OnceLock<PowerClient> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| PowerClient::new(SOCKET))
}

impl PowerClient {
    pub fn new(socket: impl Into<PathBuf>) -> Self {
        Self {
            socket: socket.into(),
            modes: Arc::new(Mutex::new(None)),
        }
    }

    /// Fetches the boot modes in the background; `modes` answers from what it brings.
    pub fn prefetch_modes(&self) -> JoinHandle<()> {
        let (socket, modes) = (self.socket.clone(), Arc::clone(&self.modes));
        std::thread::spawn(move || {
            let got =
                request(&socket, "list").map(|s| s.split_whitespace().map(String::from).collect());
            if let Some(list) = got {
                *modes.lock().unwrap_or_else(|e| e.into_inner()) = Some(list);
            }
        })
    }

    /// The boot modes the device has, as far as already known: empty until the service answered.
    pub fn modes(&self) -> Vec<String> {
        self.modes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .unwrap_or_default()
    }

    pub fn poweroff(&self) -> JoinHandle<()> {
        self.send("poweroff".to_string())
    }

    pub fn reboot(&self, mode: &str) -> JoinHandle<()> {
        self.send(format!("reboot {mode}"))
    }

    fn send(&self, line: String) -> JoinHandle<()> {
        let socket = self.socket.clone();
        std::thread::spawn(move || {
            request(&socket, &line);
        })
    }
}

/// One request (`list`, `poweroff`, `reboot MODE`) and the service's answer.
fn request(socket: &std::path::Path, line: &str) -> Option<String> {
    let answer = (|| {
        let mut s = UnixStream::connect(socket)?;
        s.set_read_timeout(Some(TIMEOUT))?;
        s.write_all(format!("{line}\n").as_bytes())?;
        s.shutdown(std::net::Shutdown::Write)?;
        let mut out = String::new();
        s.read_to_string(&mut out)?;
        Ok::<_, std::io::Error>(out)
    })();
    match answer {
        Ok(out) if out.starts_with("error") => {
            warn!(request = line, answer = out.trim(), "power service");
            None
        }
        Ok(out) => Some(out),
        Err(err) => {
            warn!(request = line, %err, "power service");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{os::unix::net::UnixListener, time::Instant};

    /// A power service that answers every request after `delay`, recording what it was asked.
    fn slow_service(delay: Duration, answer: &'static str) -> (PathBuf, Arc<Mutex<Vec<String>>>) {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // Unix socket paths are short (104 bytes on macOS): a fixed short folder, not the temp dir.
        let dir = PathBuf::from(format!("/tmp/odap-pw-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("power.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&seen);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let mut s = stream.unwrap();
                let mut line = String::new();
                s.read_to_string(&mut line).unwrap();
                log.lock().unwrap().push(line.trim().to_string());
                std::thread::sleep(delay);
                let _ = s.write_all(answer.as_bytes());
            }
        });
        (path, seen)
    }

    #[test]
    fn poweroff_and_reboot_return_at_once_while_the_service_is_slow() {
        let (path, seen) = slow_service(Duration::from_millis(800), "ok\n");
        let power = PowerClient::new(&path);
        let start = Instant::now();
        let a = power.poweroff();
        let b = power.reboot("recovery");
        assert!(
            start.elapsed() < Duration::from_millis(50),
            "caller waited {:?}",
            start.elapsed()
        );
        a.join().unwrap();
        b.join().unwrap();
        let mut got = seen.lock().unwrap().clone();
        got.sort();
        assert_eq!(got, ["poweroff", "reboot recovery"]);
    }

    #[test]
    fn modes_answer_from_the_cache_without_waiting() {
        let (path, _) = slow_service(Duration::from_millis(500), "normal recovery\n");
        let power = PowerClient::new(&path);
        let fetch = power.prefetch_modes();
        let start = Instant::now();
        assert!(power.modes().is_empty());
        assert!(start.elapsed() < Duration::from_millis(50));
        fetch.join().unwrap();
        assert_eq!(power.modes(), ["normal", "recovery"]);
    }

    #[test]
    fn a_missing_service_leaves_no_modes() {
        let power = PowerClient::new("/nonexistent/odap-power.sock");
        power.prefetch_modes().join().unwrap();
        assert!(power.modes().is_empty());
    }
}
