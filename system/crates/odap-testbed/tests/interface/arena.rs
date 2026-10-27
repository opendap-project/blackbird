//! The Arena screen: touch after the buttons.

use super::*;

pub fn touch_after_buttons(window: &Testbed, run: &mut Run) {
    let ui = window.clone_strong();
    // Touch after the buttons: a drag on the nested areas hides the ring and scrolls, the first
    // drag included (it showed the ring and scrolled nothing).
    open(&ui, "Arena");
    // The ring shown on the screen's entry (the pad), not on the nested areas: the drag's press is
    // the first thing to reach them.
    key(&ui, Key::Tab);
    let text0 = ui.get_nested_text().to_string();
    let rows = ElementHandle::find_by_accessible_label(&ui, "outer row 0").next();
    if let Some(r) = rows {
        let p = r.absolute_position();
        let (x, y0) = (p.x + 100.0, p.y + 10.0);
        let at = |y: f32| slint::LogicalPosition::new(x, y);
        use slint::platform::PointerEventButton as B;
        ui.window()
            .dispatch_event(WindowEvent::PointerMoved { position: at(y0) });
        ui.window().dispatch_event(WindowEvent::PointerPressed {
            position: at(y0),
            button: B::Left,
        });
        tick(16);
        for i in 1..=10 {
            ui.window().dispatch_event(WindowEvent::PointerMoved {
                position: at(y0 - i as f32 * 5.0),
            });
            tick(16);
        }
        ui.window().dispatch_event(WindowEvent::PointerReleased {
            position: at(y0 - 50.0),
            button: B::Left,
        });
        tick(16);
        let text1 = ui.get_nested_text().to_string();
        run.check("touch after buttons: the ring hides", !ring(&ui), || {
            "the ring shows".into()
        });
        run.check(
            "touch after buttons: the first drag scrolls",
            text1 != text0,
            || format!("{text0} → {text1}"),
        );
    } else {
        run.check(
            "touch after buttons: the outer list is there",
            false,
            || "no 'outer row 0'".into(),
        );
    }
}
