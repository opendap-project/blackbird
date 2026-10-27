//! The Controls screen: an adjustable, and a range across the window's focus.

use super::*;

pub fn adjustable(window: &Testbed, run: &mut Run) {
    let ui = window.clone_strong();
    // An adjustable (Blackbird UI's Adjustable): Action locks it, locked Volume + raises it,
    // Escape unlocks, unlocked Volume moves the focus on; ← → step it, mirrored right to left.
    open(&ui, "Controls");
    key(&ui, Key::Tab); // shows the focus, on the slider (focus on entry)
    let l0 = ui.get_level();
    let at = focus(&ui);
    back_key(&ui);
    run.check(
        "slider: unlocked, Volume moves the focus, not the value",
        ui.get_level() == l0,
        || format!("{l0} → {}", ui.get_level()),
    );
    key(&ui, Key::Tab);
    run.check("slider: back on it", focus(&ui) == at, || {
        format!("{at:?} → {:?}", focus(&ui))
    });
    key(&ui, Key::Return);
    back_key(&ui);
    run.check(
        "slider: locked, Volume + raises it 5",
        (ui.get_level() - (l0 + 5.0)).abs() < 0.01,
        || format!("{l0} → {}", ui.get_level()),
    );
    key(&ui, Key::Tab);
    run.check(
        "slider: locked, Volume − lowers it 5",
        (ui.get_level() - l0).abs() < 0.01,
        || format!("{l0} → {}", ui.get_level()),
    );
    key(&ui, Key::Escape);
    run.check(
        "slider: Escape unlocks, the screen stays",
        ui.get_screen() == "Controls",
        || ui.get_screen().to_string(),
    );
    key(&ui, Key::Tab);
    run.check(
        "slider: unlocked again, Volume moves on",
        focus(&ui) != at && (ui.get_level() - l0).abs() < 0.01,
        || format!("{:?}, level {}", focus(&ui), ui.get_level()),
    );
    back_key(&ui);
    key(&ui, Key::RightArrow);
    run.check(
        "slider: → raises it left to right",
        (ui.get_level() - (l0 + 5.0)).abs() < 0.01,
        || format!("{}", ui.get_level()),
    );
    ui.global::<Layout>().set_rtl(true);
    key(&ui, Key::RightArrow);
    run.check(
        "slider: → lowers it right to left",
        (ui.get_level() - l0).abs() < 0.01,
        || format!("{}", ui.get_level()),
    );
    ui.global::<Layout>().set_rtl(false);
}

pub fn window_focus_loss(window: &Testbed, run: &mut Run) {
    let ui = window.clone_strong();
    // The window losing and getting back the focus (the display off and on) leaves a range on the
    // knob it was on: after it, → moves the high knob, as before.
    open(&ui, "Controls");
    key(&ui, Key::Tab);
    key(&ui, Key::Tab); // the range, low knob
    key(&ui, Key::Tab); // its high knob
    let (lo, hi) = (ui.get_range_low(), ui.get_range_high());
    ui.window()
        .dispatch_event(WindowEvent::WindowActiveChanged(false));
    tick(16);
    ui.window()
        .dispatch_event(WindowEvent::WindowActiveChanged(true));
    tick(16);
    key(&ui, Key::RightArrow);
    run.check(
        "range: the window's focus back keeps the knob",
        ui.get_range_low() == lo && ui.get_range_high() > hi,
        || {
            format!(
                "low {lo} → {}, high {hi} → {}",
                ui.get_range_low(),
                ui.get_range_high()
            )
        },
    );
}
