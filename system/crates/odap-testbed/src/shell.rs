//! What every screen shares: the system's reduce motion, and Volume stopping at a screen's ends.

use std::{cell::Cell, rc::Rc, time::Duration};

use slint::{ComponentHandle, SharedString, Timer};

use crate::Testbed;

pub(crate) fn wire(ui: &Testbed) {
    // Reduce motion is the system's: every animation reads it from opendap-motion.
    opendap_motion::set_reduce_motion(ui.get_reduce_motion());
    ui.on_reduce_motion_set(opendap_motion::set_reduce_motion);

    // Volume past an end of the screen stays there (input events §3.7): Slint's focus chain wraps
    // around the window, so the screen's end stop sends the opposite key back. An edge reached by
    // that key itself is ignored: a screen with nothing focusable would bounce between its ends.
    let weak = ui.as_weak();
    let bouncing = Rc::new(Cell::new(false));
    ui.on_screen_edge(move |end| {
        if bouncing.get() {
            return;
        }
        let (weak, bouncing) = (weak.clone(), bouncing.clone());
        Timer::single_shot(Duration::ZERO, move || {
            let Some(ui) = weak.upgrade() else { return };
            bouncing.set(true);
            bounce(&ui, end);
            // The key's focus changes run inside its dispatch: an edge it reached has been seen
            // and ignored by now, and the next key, however soon, is handled again.
            bouncing.set(false);
        });
    });
}

/// Sends the key opposite to the one that reached the screen's `end`.
fn bounce(ui: &Testbed, end: bool) {
    // Volume + arrives as Shift+Tab: Shift may still be down, which would turn this Tab
    // backward again.
    let shift: SharedString = slint::platform::Key::Shift.into();
    ui.window()
        .dispatch_event(slint::platform::WindowEvent::KeyReleased { text: shift });
    let text: SharedString = if end {
        slint::platform::Key::Backtab.into()
    } else {
        slint::platform::Key::Tab.into()
    };
    ui.window()
        .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: text.clone() });
    ui.window()
        .dispatch_event(slint::platform::WindowEvent::KeyReleased { text });
}
