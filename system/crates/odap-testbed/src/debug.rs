//! The debug UI mode's flag (SYS-070) and the debug jump, watched while the testbed runs.

#[cfg(target_os = "linux")]
use crate::{Blueprint, FocusVisible, Testbed};
#[cfg(target_os = "linux")]
use slint::ComponentHandle;

/// The debug UI mode's flag (SYS-070), as the composer reads it: the file, or the environment.
pub fn debug_ui() -> bool {
    // The switch file overrides the environment's default: `1` on, `0` off (a simulator's
    // switch can turn off what the board's support package turned on).
    match std::fs::read_to_string("/run/opendap/debug-ui") {
        Ok(v) => v.trim() != "0",
        Err(_) => std::env::var("ODAP_DEBUG_UI").is_ok_and(|v| v == "1"),
    }
}

/// The debug jump's request (`watch_debug`).
#[cfg(target_os = "linux")]
const DEBUG_JUMP: &str = "/run/opendap/debug-jump";

#[cfg(target_os = "linux")]
pub fn watch_debug(weak: slint::Weak<Testbed>) {
    // SAFETY: inotify calls with a valid C string; a blocking read into a buffer of its size.
    unsafe {
        let fd = libc::inotify_init1(libc::IN_CLOEXEC);
        if fd < 0 {
            return;
        }
        let dir = std::ffi::CString::new("/run/opendap").expect("no NUL");
        libc::inotify_add_watch(
            fd,
            dir.as_ptr(),
            libc::IN_CLOSE_WRITE | libc::IN_CREATE | libc::IN_DELETE | libc::IN_MOVED_TO,
        );
        let mut buf = [0u8; 4096];
        let mut last = debug_ui();
        while libc::read(fd, buf.as_mut_ptr().cast(), buf.len()) > 0 {
            jump(&weak);
            let on = debug_ui();
            if on != last {
                last = on;
                let weak = weak.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = weak.upgrade() {
                        ui.global::<Blueprint>().set_on(on);
                    }
                });
            }
        }
    }
}

/// The debug jump: `start`, `end` or an index written to /run/opendap/debug-jump puts the focus
/// on that item of the long list (the Virtual screen's), shown, the view there at once; read
/// once, then removed. For tests that need the middle or the end of a hundred thousand rows
/// without walking there.
#[cfg(target_os = "linux")]
fn jump(weak: &slint::Weak<Testbed>) {
    let Ok(to) = std::fs::read_to_string(DEBUG_JUMP) else {
        return;
    };
    let _ = std::fs::remove_file(DEBUG_JUMP);
    let kind = match to.trim() {
        "start" => Some("home".to_string()),
        "end" => Some("end".to_string()),
        n if n.parse::<usize>().is_ok() => Some(format!("jump:{n}")),
        _ => None,
    };
    if let Some(kind) = kind {
        let weak = weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = weak.upgrade() {
                ui.global::<FocusVisible>().set_on(true);
                ui.invoke_vkey(kind.into());
            }
        });
    }
}
