//! `odap-testbed`: the UI primitives activity (SYS-071), a developer activity included in
//! Blackbird OS that exercises the UI primitives one screen at a time, built from Blackbird UI's
//! components. The debug UI mode (SYS-070) draws over it as over any activity: with the system's
//! flag on, its components draw as blueprints, and the composer draws its regions, the physical
//! controls, touches and gestures above it. It draws only on input, so an idle screen stays idle.
//!
//! The library is laid out by screen: `device` (the description and the calibration it gives),
//! `debug` (the debug UI flag and jump), `keys` (how a control's keys reach the screen that owns
//! it), `shell` (what every screen shares) and `screens/<screen>.rs`, one module per screen.

use opendap_device::Description;
use slint::ComponentHandle;

slint::include_modules!();

mod debug;
mod device;
pub mod keys;
mod screens;
mod shell;

pub use debug::debug_ui;
#[cfg(target_os = "linux")]
pub use debug::watch_debug;
pub use device::description;

/// Connects the activity's logic to its interface: every screen's callbacks, models and timers.
/// The binary calls it on the window it shows; interface tests (ADR 0014, L2) call it on a
/// window of the testing backend, then drive it with keys and touches.
pub fn wire(ui: &Testbed, desc: Option<Description>) {
    shell::wire(ui);
    let mut router = keys::Router::default();
    screens::wire(ui, desc, &mut router);
    let weak = ui.as_weak();
    ui.on_control_key(move |name, key| {
        let Some(ui) = weak.upgrade() else {
            return false;
        };
        router.route(&ui, &name, &key, ui.get_strip_rtl())
    });
}
