//! The Layers screen: a dialog's frosted backdrop, and a popover placed by opendap-input's
//! anchor.

use std::time::Instant;

use opendap_input::anchor;
use slint::ComponentHandle;

use crate::{Backdrop, Testbed};

pub(super) fn wire(ui: &Testbed) {
    // A dialog's frosted backdrop: what the window shows, shrunk four times and blurred once
    // (opendap-media's blur), drawn by Blackbird UI's scrim under a graphite tint.
    let weak = ui.as_weak();
    ui.on_frost(move || {
        let Some(ui) = weak.upgrade() else { return };
        // With reduce transparency the scrim is a solid tint: nothing to blur.
        if ui.global::<Backdrop>().get_reduce_transparency() {
            return;
        }
        let Ok(shot) = ui.window().take_snapshot() else {
            return;
        };
        let (w, h) = (shot.width() as usize, shot.height() as usize);
        let started = Instant::now();
        let (small, sw, sh) = opendap_media::blur::frosted(shot.as_bytes(), w, h, 4, 12);
        let frost = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
            &small, sw as u32, sh as u32,
        );
        ui.global::<Backdrop>()
            .set_frost(slint::Image::from_rgba8(frost));
        eprintln!("frosted {w}x{h} in {:?}", started.elapsed());
    });

    // The popover: below its button, flipped above when it does not fit, slid to stay inside the
    // screen.
    let weak = ui.as_weak();
    ui.on_place_menu(move |x, y, w, h, menu_h| {
        let Some(ui) = weak.upgrade() else { return };
        let size = ui.window().size().to_logical(ui.window().scale_factor());
        let (_, (px, py)) = anchor::place(
            anchor::Rect::new(x, y, w, h),
            (w, menu_h),
            anchor::Side::Below,
            4.0,
            anchor::Rect::new(0.0, 0.0, size.width, size.height),
            ui.get_strip_rtl(),
        );
        ui.set_anchor_x(px);
        ui.set_anchor_y(py);
    });
}
