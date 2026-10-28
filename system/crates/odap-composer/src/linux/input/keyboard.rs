//! Keys to the activity: one key, and an attached keyboard's held key repeated.

use super::*;

/// One key to the activity (xkb keycodes are Linux codes plus 8).
pub(super) fn send_key(
    composer: &mut Composer,
    code: u32,
    state: KeyState,
    time: smithay::backend::input::InputTime,
) {
    let Some(keyboard) = composer.seat.get_keyboard() else {
        return;
    };
    keyboard.input::<(), _>(
        composer,
        Keycode::new(code + 8),
        state,
        SERIAL_COUNTER.next_serial(),
        time,
        |_, _, _| FilterResult::Forward,
    );
}

/// An attached keyboard's key held: repeated by the composer (the seat's client-side repeat is off,
/// so the focus keys the composer sends never repeat twice), after the usual 600 ms at 25 per second
/// (X11's and GNOME's defaults). Modifiers never repeat, nor Escape, Enter and Space.
pub(super) const KEY_REPEAT_DELAY: std::time::Duration =
    std::time::Duration::from_millis(opendap_input::thresholds::KEY_REPEAT_DELAY_MS);
pub(super) const KEY_REPEAT_EVERY: std::time::Duration =
    std::time::Duration::from_millis(opendap_input::thresholds::KEY_REPEAT_EVERY_MS);
pub(super) const MODIFIERS: &[u32] = &[29, 42, 54, 56, 97, 100, 125, 126, 58];
/// Keys whose meaning never repeats (input events §5.1: activate, primary and back): Escape,
/// Enter, Space, the keypad's Enter.
pub(super) const NO_REPEAT: &[u32] = &[1, 28, 57, 96];

pub(super) fn keyboard_repeat(composer: &mut Composer, code: u32, pressed: bool) {
    composer.keys.key_repeat_gen += 1;
    if !pressed || MODIFIERS.contains(&code) || NO_REPEAT.contains(&code) {
        return;
    }
    let generation = composer.keys.key_repeat_gen;
    let _ = composer.handle.insert_source(
        calloop::timer::Timer::from_duration(KEY_REPEAT_DELAY),
        move |_, _, composer| {
            if composer.keys.key_repeat_gen != generation {
                return calloop::timer::TimeoutAction::Drop;
            }
            let now = smithay::backend::input::InputTime::now();
            send_key(composer, code, KeyState::Released, now);
            send_key(composer, code, KeyState::Pressed, now);
            calloop::timer::TimeoutAction::ToDuration(KEY_REPEAT_EVERY)
        },
    );
}
