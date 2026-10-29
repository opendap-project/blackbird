//! The Turns screen: a dial (the angle around its centre, and the total turn, unwrapped across
//! 0°/360°) and a strip (0 to 1 from its start edge, mirrored right to left).

use std::{cell::RefCell, rc::Rc};

use blackbird_ui_input::{gesture, space};
use slint::ComponentHandle;

use crate::{
    Testbed,
    keys::{Arrows, Router},
};

/// The dial's angle and how far it turned in all.
type Dial = Rc<RefCell<(f32, f32)>>;

pub(super) fn wire(ui: &Testbed, keys: &mut Router<Testbed>) {
    let dial: Dial = Rc::default();
    let dial_k = dial.clone();
    let weak = ui.as_weak();
    ui.on_spaces_dial(move |kind, x, y, w, h| {
        let Some(ui) = weak.upgrade() else { return };
        let (angle, _) = space::polar((w / 2.0, h / 2.0), (x, y));
        let mut d = dial.borrow_mut();
        if kind.as_str() == "down" {
            d.0 = angle;
        } else {
            d.1 += space::angle_delta(d.0, angle);
            d.0 = angle;
        }
        show_dial(&ui, *d);
    });

    let weak = ui.as_weak();
    ui.on_spaces_strip(move |x, w, rtl| {
        let Some(ui) = weak.upgrade() else { return };
        let frame = gesture::Frame {
            rtl,
            ..gesture::Frame::new((w, 32.0), ui.get_unit())
        };
        let (v, _) = space::normalised(&frame, (x, 16.0));
        ui.set_strip_value(v);
        ui.set_strip_text(
            format!(
                "{v:.2} from the {} edge",
                if rtl { "right (start)" } else { "left (start)" }
            )
            .into(),
        );
    });

    keys.add(&["dial", "strip"], Arrows::Values, move |ui, name, key| {
        let step = match key {
            "inc" => 1.0,
            "dec" => -1.0,
            _ => return false,
        };
        if name == "dial" {
            let mut d = dial_k.borrow_mut();
            let to = (d.0 + 15.0 * step).rem_euclid(360.0);
            d.1 += space::angle_delta(d.0, to);
            d.0 = to;
            show_dial(ui, *d);
        } else {
            let v = (ui.get_strip_value() + 0.05 * step).clamp(0.0, 1.0);
            ui.set_strip_value(v);
            ui.set_strip_text(format!("{v:.2} from the start edge").into());
        }
        true
    });
}

fn show_dial(ui: &Testbed, (angle, turned): (f32, f32)) {
    ui.set_dial_text(format!("angle {angle:.0}°\nturned {turned:.0}°").into());
    ui.set_dial_angle(angle);
}
