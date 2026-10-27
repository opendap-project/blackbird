//! The device around its screen: the front, drawn once, and one key per role.

use super::super::*;

/// The front, drawn at twice the logical density so it stays sharp on a Retina display: the keys'
/// rectangles and sprites where it draws them (none without a frame).
pub(crate) fn front(
    ui: &View,
    doc: &FrameDoc,
    framed: bool,
) -> (Vec<(String, frame::Rect)>, Vec<frame::Sprite>) {
    let px_mm = doc.display.width_px as f32 / doc.display.width_mm;
    let framed = framed && doc.case.is_some() && doc.front.is_some();
    ui.set_framed(framed);
    let mut key_rects: Vec<(String, frame::Rect)> = Vec::new();
    let mut sprites: Vec<frame::Sprite> = Vec::new();
    if let (true, Some(case), Some(front)) = (framed, doc.case.as_ref(), doc.front.as_ref()) {
        let panel = frame::Panel {
            width_mm: doc.display.width_mm,
            height_mm: doc.display.height_mm,
            round: doc.display.shape == "round",
        };
        let drawn = frame::draw(case, front, &panel, px_mm, 2.0);
        let buffer = SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
            &drawn.rgba,
            drawn.width,
            drawn.height,
        );
        ui.set_front(slint::Image::from_rgba8_premultiplied(buffer));
        ui.set_front_w(drawn.size.0);
        ui.set_front_h(drawn.size.1);
        ui.set_screen_x(drawn.screen.x);
        ui.set_screen_y(drawn.screen.y);
        ui.set_led_x(drawn.led.0);
        ui.set_led_y(drawn.led.1);
        ui.set_led_d(drawn.led.2);
        key_rects = drawn.keys;
        sprites = drawn.sprites;
    }
    (key_rects, sprites)
}

/// One key per role, with the shape the concept art gives it (T1-059); and each role's QEMU key.
pub(crate) fn keys(
    ui: &View,
    doc: &FrameDoc,
    key_rects: &[(String, frame::Rect)],
    sprites: &[frame::Sprite],
) -> (Rc<VecModel<Key>>, Vec<(String, String)>) {
    let mut keys: Vec<Key> = Vec::new();
    let mut qcode_of: Vec<(String, String)> = Vec::new();
    for c in &doc.controls {
        let Some(q) = c.qcode.clone() else { continue };
        if keys.iter().any(|k| k.role == c.role.as_str()) {
            continue;
        }
        let r = key_rects
            .iter()
            .find(|(role, _)| *role == c.role)
            .map(|(_, r)| *r)
            .unwrap_or_default();
        let mut key = Key {
            role: c.role.clone().into(),
            x: r.x,
            y: r.y,
            w: r.w,
            h: r.h,
            pressed: false,
            ..Default::default()
        };
        if let Some(sp) = sprites.iter().find(|s| s.role == c.role) {
            let buffer = SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
                &sp.rgba, sp.width, sp.height,
            );
            key.sprite = slint::Image::from_rgba8_premultiplied(buffer);
            key.has_sprite = true;
            (key.sx, key.sy, key.sw, key.sh, key.travel) =
                (sp.rect.x, sp.rect.y, sp.rect.w, sp.rect.h, sp.travel);
        }
        keys.push(key);
        qcode_of.push((c.role.clone(), q));
    }
    let keys_model = Rc::new(VecModel::from(keys));
    ui.set_keys(ModelRc::from(keys_model.clone()));
    (keys_model, qcode_of)
}
