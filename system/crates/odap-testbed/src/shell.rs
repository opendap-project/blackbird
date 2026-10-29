//! What every screen shares: the system's reduce motion, and Volume stopping at a screen's ends.

use std::{cell::Cell, rc::Rc, time::Duration};

use slint::platform::{Key, WindowEvent};
use slint::{ComponentHandle, SharedString, Timer};

use crate::Testbed;

pub(crate) fn wire(ui: &Testbed) {
    // Reduce motion is the system's: every animation reads it from blackbird-ui-motion.
    blackbird_ui_motion::set_reduce_motion(ui.get_reduce_motion());
    ui.on_reduce_motion_set(blackbird_ui_motion::set_reduce_motion);

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

/// Sends the key opposite to the one that reached the screen's `end`. Past the last element that
/// is Shift+Tab, which goes back whatever Shift is doing. Before the first it is a plain Tab, which
/// Shift would turn backward: Volume + and Shift+Tab come with Shift down, so Shift is let go for
/// that Tab and put back down after, when it is still down (Blackbird UI `FocusNav.shift`), or a
/// Volume + still held would step forward from then on.
fn bounce(ui: &Testbed, end: bool) {
    let window = ui.window();
    let key = |text: SharedString, pressed: bool| {
        window.dispatch_event(if pressed {
            WindowEvent::KeyPressed { text }
        } else {
            WindowEvent::KeyReleased { text }
        })
    };
    let tap = |k: Key| {
        key(k.into(), true);
        key(k.into(), false);
    };
    if end {
        tap(Key::Backtab);
        return;
    }
    let shift = ui.global::<crate::FocusNav>().get_shift();
    key(Key::Shift.into(), false);
    tap(Key::Tab);
    if shift {
        key(Key::Shift.into(), true);
    }
}
