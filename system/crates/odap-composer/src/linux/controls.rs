//! The system combinations on `main`, carried out: crate::combinations decides (its table is T1
//! controls and status LED spec §1.3); this side lights the debug marks, arms the hold timers and
//! runs the result. The composer owns `main`, and logind handles none of its presses
//! (blackbird-os's logind drop-in). Forced power-off is the PMIC's, in hardware, whatever the OS
//! does.

use std::time::Instant;

use blackbird_device::Role;
use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use tracing::info;

use super::state::Composer;
use crate::combinations::{Kind, Outcome, Short};
use crate::power_menu::Request;

/// A control changed. Returns whether it was part of a system combination.
pub fn control(composer: &mut Composer, role: Role, pressed: bool) -> bool {
    // The debug UI mode lights the control's mark while it is down, whatever it does.
    if let Some(layers) = composer.system.layers.as_mut() {
        layers.refresh_debug();
        if super::layers::debug_ui() {
            layers.control(role.as_str(), pressed);
            redraw(composer);
        }
    }
    let menu = menu_open(composer);
    let display_on = composer.backend.as_ref().is_some_and(|b| b.display_on());
    match composer
        .system
        .controls
        .control(role, pressed, menu, display_on)
    {
        Outcome::Pass => false,
        Outcome::Taken => true,
        Outcome::Started { id, kind } => {
            arm_hold(composer, id, kind);
            true
        }
        Outcome::MenuMove(by) => {
            if let Some(layers) = composer.system.layers.as_mut() {
                layers.move_focus(by);
            }
            layer_requests(composer);
            redraw(composer);
            true
        }
        Outcome::MenuActivate => {
            if let Some(layers) = composer.system.layers.as_mut() {
                layers.activate_focus();
            }
            layer_requests(composer);
            redraw(composer);
            true
        }
        Outcome::Short(short) => {
            run_short(composer, short);
            true
        }
    }
}

fn redraw(composer: &mut Composer) {
    if let Some(backend) = composer.backend.as_mut() {
        backend.schedule();
    }
    super::backend::render_if_idle(composer);
}

/// The press's held action, when it is still down after its hold time.
fn arm_hold(composer: &mut Composer, id: u64, kind: Kind) {
    let _ = composer.handle.insert_source(
        Timer::from_deadline(Instant::now() + kind.hold()),
        move |_, _, composer| {
            if let Some(kind) = composer.system.controls.held(id) {
                info!(combination = kind.name(), "power menu");
                open_power_menu(composer);
            }
            TimeoutAction::Drop
        },
    );
}

fn run_short(composer: &mut Composer, short: Short) {
    match short {
        Short::PowerOff => {
            info!("power off, confirmed with main");
            crate::power::client().poweroff();
        }
        Short::Display { on } => {
            info!(display = if on { "on" } else { "off" }, "main pressed");
            if !on {
                super::input::system_takeover(composer);
            }
            set_display(composer, on);
        }
        Short::Screenshot => {
            if let Some(backend) = composer.backend.as_mut() {
                backend.request_screenshot();
            }
            super::backend::render_if_idle(composer);
        }
    }
}

/// Opens the power menu beside `main`, with the display on; recovery only if the device has it.
fn open_power_menu(composer: &mut Composer) {
    // A system view takes over: the activity's keys and touches end here.
    super::input::system_takeover(composer);
    // From the cache the composer filled at start: never wait for the power service here (T1-047).
    let can_recovery = crate::power::client()
        .modes()
        .iter()
        .any(|m| m == "recovery");
    if let Some(backend) = composer.backend.as_mut() {
        backend.set_display(true);
        backend.schedule();
    }
    // Opened with the display off: the activity gets its keyboard back for when the menu closes.
    super::input::activity_keyboard(composer, true);
    if let Some(layers) = composer.system.layers.as_mut() {
        layers.open_power(can_recovery);
    }
    super::backend::render_if_idle(composer);
}

/// What the system layers asked for (layers.rs), after their input.
pub fn layer_requests(composer: &mut Composer) {
    let Some(layers) = composer.system.layers.as_mut() else {
        return;
    };
    for request in layers.take_requests() {
        match request {
            Request::Restart => {
                info!("restart, from the power menu");
                crate::power::client().reboot("normal");
            }
            Request::RestartToRecovery => {
                info!("restart to recovery, from the power menu");
                crate::power::client().reboot("recovery");
            }
            Request::Cancel => layers.close(),
        }
    }
}

fn menu_open(composer: &Composer) -> bool {
    composer
        .system
        .layers
        .as_ref()
        .is_some_and(|l| l.power_open())
}

/// The display off or on again, and the activity's keyboard with it (composer spec §2).
pub fn set_display(composer: &mut Composer, on: bool) {
    if let Some(backend) = composer.backend.as_mut() {
        backend.set_display(on);
    }
    super::input::activity_keyboard(composer, on);
    super::backend::render_if_idle(composer);
}
