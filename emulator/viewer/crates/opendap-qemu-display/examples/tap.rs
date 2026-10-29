//! Attaches to a QEMU's D-Bus display, taps a pixel with the absolute pointer, and prints the
//! frame's size and how many changes arrived in the next second.
//!
//! cargo run --example tap -- QMP_SOCKET X Y

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

fn main() -> std::io::Result<()> {
    let mut args = std::env::args().skip(1);
    let qmp = args.next().expect("QMP socket");
    let x: u32 = args.next().and_then(|v| v.parse().ok()).expect("X");
    let y: u32 = args.next().and_then(|v| v.parse().ok()).expect("Y");
    let changes = Arc::new(AtomicUsize::new(0));
    let counted = changes.clone();
    let d = blackbird_emulator_qemu_display::attach(qmp.as_ref(), move |_, _| {
        counted.fetch_add(1, Ordering::Relaxed);
    })?;
    std::thread::sleep(Duration::from_millis(300));
    let f = d.frame();
    println!("frame {} x {}", f.width, f.height);
    changes.store(0, Ordering::Relaxed);
    d.pointer(x, y, Some(true))?;
    std::thread::sleep(Duration::from_millis(60));
    d.pointer(x, y, Some(false))?;
    std::thread::sleep(Duration::from_secs(1));
    println!(
        "{} changes in the second after the tap",
        changes.load(Ordering::Relaxed)
    );
    Ok(())
}
