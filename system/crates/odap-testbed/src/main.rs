//! `odap-testbed`: the UI primitives activity's binary: the window, its arguments and the debug
//! UI flag; the logic is in the library (`wire`).

#[cfg(target_os = "linux")]
use odap_testbed::watch_debug;
use odap_testbed::{Blueprint, SCREENS, Testbed, debug_ui, description, wire};
use slint::ComponentHandle;

/// The usage, with every screen the list has (screens.tsv), wrapped under the argument.
fn usage() -> String {
    let mut out =
        String::from("usage: odap-testbed [SCREEN]\n  SCREEN   open that primitive's screen:");
    let mut width = 80;
    for (i, name) in SCREENS.iter().enumerate() {
        let item = if i + 1 < SCREENS.len() {
            format!("{name},")
        } else {
            name.to_string()
        };
        if width + 1 + item.len() > 80 {
            out.push_str("\n           ");
            width = 11;
        } else {
            out.push(' ');
            width += 1;
        }
        out.push_str(&item);
        width += item.len();
    }
    out
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = Testbed::new()?;
    if let Some(arg) = std::env::args().nth(1) {
        if arg == "-h" || arg == "--help" {
            println!("{}", usage());
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
