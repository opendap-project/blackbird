//! `odap-testbed`: the UI primitives activity's binary: the window, its arguments and the debug
//! UI flag; the logic is in the library (`wire`).

#[cfg(target_os = "linux")]
use odap_testbed::watch_debug;
use odap_testbed::{Blueprint, Testbed, debug_ui, description, wire};
use slint::ComponentHandle;

const USAGE: &str = "usage: odap-testbed [SCREEN]
  SCREEN   open that primitive's screen (Calibration, Targets, Gestures, Controls, Motion,
           Opacity, Exceptions)";

fn main() -> Result<(), slint::PlatformError> {
    let ui = Testbed::new()?;
    if let Some(arg) = std::env::args().nth(1) {
        if arg == "-h" || arg == "--help" {
            println!("{USAGE}");
            return Ok(());
        }
        ui.set_screen(arg.into());
    }
    ui.global::<Blueprint>().set_on(debug_ui());
    // The flag is watched (inotify, no polling): the blueprint switches in place, over whatever
    // state is on screen (screen, focus, scroll, locks), never restarting anything.
    #[cfg(target_os = "linux")]
    {
        let weak = ui.as_weak();
        std::thread::spawn(move || watch_debug(weak));
    }
    wire(&ui, description());
    ui.run()
}
