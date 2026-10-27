//! The States screen: disabled components.

use super::*;

/// The States screen's components that are off, as a screen reader sees them, and their boxes.
fn off_all(ui: &Testbed) -> Vec<(String, f32, f32, f32, f32)> {
    ElementQuery::from_root(ui)
        .match_predicate(|e| {
            e.accessible_label().is_some_and(|l| l.starts_with("Off "))
                && e.accessible_role().is_some()
        })
        .find_all()
        .into_iter()
        .map(|e| {
            let (p, z) = (e.absolute_position(), e.size());
            (
                e.accessible_label().unwrap().to_string(),
                p.x,
                p.y,
                z.width,
                z.height,
            )
        })
        .filter(|e| e.3 > 0.0 && e.4 > 0.0)
        .collect()
}

pub fn disabled_walk(window: &Testbed, run: &mut Run) {
    let ui = window.clone_strong();
    // comp-disabled-each: on the States screen every focusable component appears disabled. The
    // buttons never land on one, a touch on one takes no focus, and each is dimmed once (its own
    // opacity, not compounded by a disabled parent).
    open(&ui, "States");
    // The query sees what the scroll area shows: the gallery, at the screen's end, comes into view
    // as the walk reveals what follows the focus; a focus landing on one would be revealed too.
    key(&ui, Key::Tab);
    let mut landed = Vec::new();
    for _ in 0..60 {
        let (y, h) = focus(&ui);
        for e in off_all(&ui) {
            if (y - e.2).abs() < 1.0 && (h - e.4).abs() < 1.0 {
                landed.push(e.0.clone());
            }
        }
        let serial = ui.global::<FocusReveal>().get_serial();
        key(&ui, Key::Tab);
        if ui.global::<FocusReveal>().get_serial() == serial {
            break;
        }
    }
    run.check(
        "disabled: the buttons skip every one",
        landed.is_empty(),
        || landed.join(", "),
    );
    // Nothing focusable follows the error state, so the buttons stop there; a finger scrolls the
    // rest into view (the gallery), then touches each.
    for _ in 0..6 {
        if off_all(&ui)
            .iter()
            .filter(|e| e.2 >= 0.0 && e.2 + e.4 <= 480.0)
            .count()
            >= 6
        {
            break;
        }
        use slint::platform::PointerEventButton as B;
        let p = |y: f32| slint::LogicalPosition::new(300.0, y);
        ui.window()
            .dispatch_event(WindowEvent::PointerMoved { position: p(420.0) });
        ui.window().dispatch_event(WindowEvent::PointerPressed {
            position: p(420.0),
            button: B::Left,
        });
        for i in 1..=12 {
            ui.window().dispatch_event(WindowEvent::PointerMoved {
                position: p(420.0 - i as f32 * 25.0),
            });
            tick(16);
        }
        ui.window().dispatch_event(WindowEvent::PointerReleased {
            position: p(120.0),
            button: B::Left,
        });
        tick(400);
    }
}

pub fn disabled_touch(window: &Testbed, run: &mut Run) {
    let ui = window.clone_strong();
    let visible: Vec<_> = off_all(&ui)
        .into_iter()
        .filter(|e| e.2 >= 0.0 && e.2 + e.4 <= 480.0)
        .collect();
    run.check(
        "disabled: the gallery in view at the end",
        visible.len() >= 4,
        || format!("{} in view", visible.len()),
    );
    let mut acted = Vec::new();
    for e in &visible {
        let serial = ui.global::<FocusReveal>().get_serial();
        let at = slint::LogicalPosition::new(e.1 + e.3 / 2.0, e.2 + e.4 / 2.0);
        use slint::platform::PointerEventButton as B;
        ui.window()
            .dispatch_event(WindowEvent::PointerMoved { position: at });
        ui.window().dispatch_event(WindowEvent::PointerPressed {
            position: at,
            button: B::Left,
        });
        tick(30);
        ui.window().dispatch_event(WindowEvent::PointerReleased {
            position: at,
            button: B::Left,
        });
        tick(30);
        if ui.global::<FocusReveal>().get_serial() != serial {
            acted.push(e.0.clone());
        }
    }
    run.check(
        "disabled: a touch gives none the focus",
        acted.is_empty(),
        || acted.join(", "),
    );
}

pub fn disabled_dims(window: &Testbed, run: &mut Run) {
    let ui = window.clone_strong();
    let dims: Vec<String> = ElementQuery::from_root(&ui)
        .match_predicate(|e| {
            e.accessible_label().is_some_and(|l| l.starts_with("Off "))
                && e.accessible_role().is_some()
        })
        .find_all()
        .into_iter()
        .filter_map(|e| {
            // Dimmed as a whole, or part by part (a line's label, a toggle's switch): each part once.
            let o = e.computed_opacity();
            let parts: Vec<f32> = e
                .query_descendants()
                .match_predicate(|c| {
                    c.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::Text)
                })
                .find_all()
                .into_iter()
                .map(|c| c.computed_opacity())
                .collect();
            let whole = (o - 0.45).abs() <= 0.02;
            let by_parts = (o - 1.0).abs() <= 0.02
                && !parts.is_empty()
                && parts.iter().all(|p| (p - 0.45).abs() <= 0.02);
            (!whole && !by_parts)
                .then(|| format!("{} {o:.2} parts {parts:?}", e.accessible_label().unwrap()))
        })
        .collect();
    run.check("disabled: each dimmed once (0.45)", dims.is_empty(), || {
        dims.join(", ")
    });
}
