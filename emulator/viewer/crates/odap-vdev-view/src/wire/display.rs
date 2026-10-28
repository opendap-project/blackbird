//! The device's display: QEMU's D-Bus display pushes each change as the guest draws it
//! (virtual-devices ADR 0002); the frame is drawn on the event loop.

use super::super::*;

/// Attached over the window's QMP socket before any key uses it (QMP serves one client at a
/// time; the keys open their session on first use); the pointer over it follows.
pub(crate) fn display(ui: &View, qmp: &std::path::Path) -> Arc<opendap_qemu_display::Display> {
    let weak = ui.as_weak();
    // ODAP_VIEW_FRAMES_LOG=FILE: one line per change the window receives (seconds since it opened,
    // and the rectangle), to measure the window's own frame rate (odapkit motion measures the
    // device's). Off unless asked for.
    let frames_log = std::env::var_os("ODAP_VIEW_FRAMES_LOG")
        .and_then(|p| std::fs::File::create(p).ok())
        .map(|f| std::sync::Mutex::new(std::io::LineWriter::new(f)));
    let opened = std::time::Instant::now();
    let display = match opendap_qemu_display::attach(qmp, move |frame, change| {
        if let Some(log) = &frames_log {
            use std::io::Write as _;
            let at = opened.elapsed().as_secs_f64();
            let line = match change {
                opendap_qemu_display::Change::Pixels { x, y, w, h } => {
                    format!("{at:.4} {x} {y} {w} {h}")
                }
                c => format!("{at:.4} {c:?}"),
            };
            let _ = writeln!(log.lock().unwrap_or_else(|p| p.into_inner()), "{line}");
        }
        if change == opendap_qemu_display::Change::Disabled {
            return;
        }
        let (w, h) = (frame.width, frame.height);
        let mut rgba = Vec::with_capacity(frame.bgrx.len());
        for p in frame.bgrx.as_chunks::<4>().0 {
            rgba.extend_from_slice(&[p[2], p[1], p[0], 255]);
        }
        let weak = weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = weak.upgrade() {
                let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&rgba, w, h);
                ui.set_screen(slint::Image::from_rgba8(buffer));
            }
        });
    }) {
        Ok(d) => Arc::new(d),
        Err(e) => usage(&format!("display: {e}")),
    };
    {
        // The pointer over the display, pressed or not, so the composer's touch indicator follows it.
        let display = display.clone();
        ui.on_screen_pointer(move |kind, x, y| {
            let pressed = match kind.as_str() {
                "down" => Some(true),
                "up" => Some(false),
                _ => None,
            };
            let _ = display.pointer(x.max(0.0) as u32, y.max(0.0) as u32, pressed);
        });
    }
    display
}

/// The device stopped: the window goes with it.
pub(crate) fn close_with(display: &Arc<opendap_qemu_display::Display>) {
    let display = display.clone();
    std::thread::spawn(move || {
        display.wait_closed();
        let _ = slint::invoke_from_event_loop(|| {
            let _ = slint::quit_event_loop();
        });
    });
}

/// ODAP_VIEW_SNAPSHOT=FILE.png: the window as drawn, saved once the device has drawn, for checks
/// without a person (an agent, a test) and with the host's screen locked.
pub(crate) fn snapshot(ui: &View) {
    if let Some(path) = std::env::var_os("ODAP_VIEW_SNAPSHOT") {
        let weak = ui.as_weak();
        slint::Timer::single_shot(std::time::Duration::from_secs(3), move || {
            let Some(ui) = weak.upgrade() else { return };
            match ui.window().take_snapshot() {
                Ok(shot) => {
                    let saved = save_png(
                        &PathBuf::from(&path),
                        shot.width(),
                        shot.height(),
                        shot.as_bytes(),
                    );
                    if let Err(e) = saved {
                        eprintln!("odap-vdev-view: snapshot: {e}");
                    }
                }
                Err(e) => eprintln!("odap-vdev-view: snapshot: {e}"),
            }
        });
    }
}
