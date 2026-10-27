//! The screens' logic, one module per screen: each wires its own callbacks, models and timers,
//! and registers the controls it owns with the key router.

use opendap_device::Description;

use crate::{Testbed, keys::Router};

mod arena;
mod calibration;
mod exceptions;
mod gestures;
mod keyboard;
mod layers;
mod media;
mod navigation;
mod spaces;
mod transitions;
mod turns;
mod virtual_list;

/// Wires every screen that has logic of its own (the others are the interface alone).
pub(crate) fn wire(ui: &Testbed, desc: Option<Description>, keys: &mut Router<Testbed>) {
    calibration::wire(ui, desc);
    spaces::wire(ui, keys);
    turns::wire(ui, keys);
    arena::wire(ui, keys);
    virtual_list::wire(ui);
    media::wire(ui, keys);
    navigation::wire(ui, keys);
    keyboard::wire(ui);
    layers::wire(ui);
    transitions::wire(ui, keys);
    gestures::wire(ui);
    exceptions::wire(ui);
}
