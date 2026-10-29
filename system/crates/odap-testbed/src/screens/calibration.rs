//! The Calibration screen: the device, its density and the scale the window was given.

use std::{rc::Rc, time::Duration};

use blackbird_device::Description;
use slint::{ComponentHandle, ModelRc, Timer, VecModel};

use crate::{
    Testbed,
    device::{calibration, unit},
};

pub(super) fn wire(ui: &Testbed, desc: Option<Description>) {
    let desc = Rc::new(desc);
    // The applied scale is known once the window is on screen: calibrate then, once.
    let weak = ui.as_weak();
    let calib_desc = desc.clone();
    Timer::single_shot(Duration::ZERO, move || {
        let Some(ui) = weak.upgrade() else { return };
        let applied = ui.window().scale_factor();
        ui.set_unit(unit((*calib_desc).as_ref(), applied));
        if let Some(d) = (*calib_desc).as_ref() {
            let (lines, off) = calibration(d, applied);
            ui.set_calibration(ModelRc::new(VecModel::from(lines)));
            ui.set_calibration_off(off);
        }
    });
    let device = match (*desc).as_ref() {
        Some(d) => {
            let size = d
                .display
                .as_ref()
                .map(|x| format!("{} × {} px", x.width_px, x.height_px))
                .unwrap_or_else(|| "no display".into());
            format!("{} · {size}", d.identity.name)
        }
        None => "no device description".into(),
    };
    ui.set_device(device.into());
}
