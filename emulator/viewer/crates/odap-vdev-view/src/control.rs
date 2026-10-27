//! The window's control socket (`view.sock`, beside the device's frame): what other tools ask of
//! the window while it runs. QEMU's D-Bus display serves one client peer to peer, and a second
//! one displaces the first (the window would close), so the window is that client and tools ask
//! it instead of attaching themselves (virtual-devices ADR 0003).
//!
//! One JSON request a line, answered with JSON lines:
//!
//! - `{"record": "out" | "in", "seconds": S, "path": "/abs/file.wav", "wait": S}`: the exact bytes
//!   of the guest's output (or of what it is given to record) from their first sound, for
//!   `seconds`. Answered `{"listening": true}` at once, then `{"ok": true, "format": …,
//!   "seconds": …}` or `{"error": …}`; nothing played within `wait` seconds is an error.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crate::sound::{Direction, Sound};

/// The socket's path for the device whose frame is at `frame`.
pub fn path(frame: &Path) -> PathBuf {
    frame.with_file_name("view.sock")
}

/// Serves requests on a thread of its own until the window exits.
pub fn serve(frame: &Path, sound: Option<Arc<Sound>>) {
    let at = path(frame);
    let _ = std::fs::remove_file(&at);
    let listener = match UnixListener::bind(&at) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("odap-vdev-view: control socket {}: {e}", at.display());
            return;
        }
    };
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let sound = sound.clone();
            std::thread::spawn(move || answer(stream, sound));
        }
    });
}

fn reply(stream: &mut UnixStream, v: serde_json::Value) {
    let _ = writeln!(stream, "{v}");
}

fn answer(mut stream: UnixStream, sound: Option<Arc<Sound>>) {
    let mut line = String::new();
    if BufReader::new(&stream).read_line(&mut line).is_err() {
        return;
    }
    let req: serde_json::Value = match serde_json::from_str(&line) {
        Ok(v) => v,
        Err(e) => {
            return reply(
                &mut stream,
                serde_json::json!({ "error": format!("not JSON: {e}") }),
            );
        }
    };
    let Some(which) = req["record"].as_str() else {
        return reply(
            &mut stream,
            serde_json::json!({ "error": "unknown request" }),
        );
    };
    let Some(sound) = sound else {
        return reply(
            &mut stream,
            serde_json::json!({ "error": "the window has no sound (the device was started without its D-Bus audio)" }),
        );
    };
    let direction = if which == "in" {
        Direction::Input
    } else {
        Direction::Output
    };
    let seconds = req["seconds"].as_f64().unwrap_or(5.0);
    let wait = req["wait"].as_f64().unwrap_or(30.0);
    let Some(path) = req["path"]
        .as_str()
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
    else {
        return reply(
            &mut stream,
            serde_json::json!({ "error": "record needs an absolute path" }),
        );
    };
    let done = match sound.record(direction, &path, seconds) {
        Ok(rx) => rx,
        Err(e) => return reply(&mut stream, serde_json::json!({ "error": e })),
    };
    reply(&mut stream, serde_json::json!({ "listening": true }));
    // Nothing that started within `wait`, or a stream that stopped before the end: what was
    // recorded is kept and said.
    let r = done
        .recv_timeout(Duration::from_secs_f64(wait + seconds + 2.0))
        .unwrap_or_else(|_| {
            sound.stop_recording(direction);
            done.recv_timeout(Duration::from_secs(2))
                .unwrap_or(Err("no answer".into()))
        });
    match r {
        Ok((f, secs)) => reply(
            &mut stream,
            serde_json::json!({ "ok": true, "format": f.describe(), "seconds": secs, "path": path }),
        ),
        Err(e) => reply(&mut stream, serde_json::json!({ "error": e })),
    }
}
