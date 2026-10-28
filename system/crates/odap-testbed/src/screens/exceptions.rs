//! The Exceptions screen: the activity fails on purpose, as the composer must survive.

use crate::Testbed;

pub(super) fn wire(ui: &Testbed) {
    ui.on_exception(|what| match what.as_str() {
        // The composer keeps the last frame and restarts the activity (composer spec §9).
        "panic" => panic!("testbed: a panic, asked for"),
        // An unresponsive activity: the event loop blocks, nothing draws.
        "hang" => std::thread::sleep(std::time::Duration::from_secs(5)),
        _ => std::process::exit(0),
    });
}
