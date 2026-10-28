//! The debug UI mode's and the touch indicator's view of contacts.

use super::*;

/// How long the debug UI mode shows a gesture after its contact ends: feedback for the eye, not a
/// message to read (SYS-075). It stays while the contact lasts, and a newer
/// gesture replaces it at once.
pub(super) const GESTURE_SHOWN: std::time::Duration = std::time::Duration::from_millis(800);

/// Tells the debug UI mode, when it is on, of a contact: its crosshair and region, then its
/// gesture, shown for a while.
pub(super) fn debug(composer: &mut Composer, at: Point<f64, Logical>, contact: LayerContact) {
    // A long press is named at LONG_MS while the finger is still down.
    if matches!(contact, LayerContact::Down) && crate::linux::layers::debug_ui() {
        let _ = composer.handle.insert_source(
            calloop::timer::Timer::from_duration(std::time::Duration::from_millis(
                opendap_input::gesture::LONG_MS,
            )),
            |_, _, composer| {
                if let Some(layers) = composer.system.layers.as_mut() {
                    // Shown while the finger stays down; its timeout starts when it lifts.
                    layers.debug_long_press();
                    if let Some(b) = composer.backend.as_mut() {
                        b.schedule();
                    }
                    crate::linux::backend::render_if_idle(composer);
                }
                calloop::timer::TimeoutAction::Drop
            },
        );
    }
    let Some(layers) = composer.system.layers.as_mut() else {
        return;
    };
    let changed = layers.refresh_debug();
    let Some(seq) = layers.debug_contact(at, contact).or(changed.then_some(0)) else {
        if crate::linux::layers::debug_ui() {
            if let Some(backend) = composer.backend.as_mut() {
                backend.schedule();
            }
            crate::linux::backend::render_if_idle(composer);
        }
        return;
    };
    if seq > 0 {
        clear_later(composer, seq);
    }
    if let Some(backend) = composer.backend.as_mut() {
        backend.schedule();
    }
    crate::linux::backend::render_if_idle(composer);
}

/// The gesture's callout leaves after GESTURE_SHOWN.
pub(super) fn clear_later(composer: &mut Composer, seq: u64) {
    {
        let _ = composer.handle.insert_source(
            calloop::timer::Timer::from_duration(GESTURE_SHOWN),
            move |_, _, composer| {
                if let Some(l) = composer.system.layers.as_mut() {
                    l.clear_gesture(seq);
                }
                if let Some(b) = composer.backend.as_mut() {
                    b.schedule();
                }
                crate::linux::backend::render_if_idle(composer);
                calloop::timer::TimeoutAction::Drop
            },
        );
    }
}

/// Tells the touch indicator, when it is on, and draws its change.
pub(super) fn indicate(
    composer: &mut Composer,
    change: impl FnOnce(&mut crate::linux::touches::Touches),
) {
    if !composer.system.touches.enabled() {
        return;
    }
    change(&mut composer.system.touches);
    if let Some(backend) = composer.backend.as_mut() {
        backend.schedule();
    }
    crate::linux::backend::render_if_idle(composer);
}
