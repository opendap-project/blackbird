//! The window's wiring: each part of the window connected to the device, one function each, so
//! `main` reads as the list of what the window does.

mod audio;
mod controls;
mod display;
mod front;
mod sidebar;
mod system;

pub(crate) use audio::{controls as sound_controls, prefs, sound};
pub(crate) use controls::{host_keys, mouse, press, sender};
pub(crate) use display::{close_with, display, snapshot};
pub(crate) use front::{front, keys};
pub(crate) use sidebar::sidebar;
pub(crate) use system::system;
