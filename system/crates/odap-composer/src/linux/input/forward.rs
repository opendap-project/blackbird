//! A control to the activity: the keys its press sends (crate::forward decides them), held while
//! it is, repeated where its meaning repeats, and every one released when the system takes over.

use super::*;

use crate::forward::KEY_TAB;

/// A held focus step's repeat accelerates exponentially and stops where it is released, with no
/// inertia (SYS-073): after Android's
/// key-repeat delay, the first step comes after REPEAT_FIRST, and each next interval is
/// REPEAT_FACTOR of the last, down to REPEAT_FASTEST. Precise at first, fast through long lists.
/// SYS-066 asks for these to be adjustable.
pub(super) const REPEAT_DELAY: std::time::Duration =
    std::time::Duration::from_millis(opendap_input::thresholds::REPEAT_DELAY_MS);
pub(super) const REPEAT_FIRST: f64 = opendap_input::thresholds::REPEAT_FIRST_MS;
pub(super) const REPEAT_FACTOR: f64 = opendap_input::thresholds::REPEAT_FACTOR;
pub(super) const REPEAT_FASTEST: f64 = opendap_input::thresholds::REPEAT_FASTEST_MS;

/// One more Tab while a volume button is held in focus navigation: Tab released and pressed
/// again, Shift (for Volume +) still down.
pub(super) fn step_again(composer: &mut Composer) {
    let Some(keyboard) = composer.seat.get_keyboard() else {
        return;
    };
    for state in [KeyState::Released, KeyState::Pressed] {
        keyboard.input::<(), _>(
            composer,
            Keycode::new(KEY_TAB + 8),
            state,
            SERIAL_COUNTER.next_serial(),
            smithay::backend::input::InputTime::now(),
            |_, _, _| FilterResult::Forward,
        );
    }
}

/// Milliseconds on a monotonic clock, for the control primitives' state machines.
pub(super) fn now_ms() -> u64 {
    static EPOCH: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    EPOCH
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_millis() as u64
}

/// Whether sound is playing: any ALSA playback stream running (the cards' `status` files).
pub(super) fn sound_playing() -> bool {
    let Ok(cards) = std::fs::read_dir("/proc/asound") else {
        return false;
    };
    cards
        .flatten()
        .filter(|c| c.file_name().to_string_lossy().starts_with("card"))
        .flat_map(|c| std::fs::read_dir(c.path()).into_iter().flatten().flatten())
        .filter(|p| p.file_name().to_string_lossy().ends_with('p'))
        .flat_map(|p| std::fs::read_dir(p.path()).into_iter().flatten().flatten())
        .any(|s| {
            std::fs::read_to_string(s.path().join("status")).is_ok_and(|t| t.contains("RUNNING"))
        })
}

/// A role the system combinations left to the activity, pressed: crate::forward decides the keys
/// it sends and whether it repeats; the keys chosen at the press are the ones released
/// ([`release_forwarded`]).
pub(super) fn forward(
    composer: &mut Composer,
    role: opendap_device::Role,
    time: smithay::backend::input::InputTime,
) {
    use opendap_input::semantic::Semantic;
    let display_on = !composer.backend.as_ref().is_some_and(|b| !b.display_on());
    let sound = display_on && sound_playing();
    let Some(press) = crate::forward::press(
        role,
        display_on,
        sound,
        &mut composer.keys.volume_presses,
        now_ms(),
    ) else {
        return;
    };
    match press.meaning {
        // No audio engine yet to take these (system ADR 0002 odap-audiod).
        Semantic::PlayPause => info!("play/pause, display off"),
        // The volume: a step as the button goes down, repeated while it is held; a second press
        // within the double-press time takes the first one's step back and changes the track, as
        // media keys to the activity until the audio engine takes them.
        Semantic::VolumeUp | Semantic::VolumeDown if press.track.is_none() => {
            info!(?role, "volume step")
        }
        _ => {}
    }
    if let Some(key) = press.track {
        info!(?role, "volume step undone");
        info!(?role, "track, double press");
        let now = smithay::backend::input::InputTime::now();
        send_key(composer, key, KeyState::Pressed, now);
        send_key(composer, key, KeyState::Released, now);
        composer.keys.forwarded.insert(role, &[]);
        return;
    }
    composer.keys.forwarded.insert(role, press.keys);
    if press.repeats {
        repeat_held(composer, role, press.meaning);
    }
    for &code in press.keys {
        send_key(composer, code, KeyState::Pressed, time);
    }
}

/// Held, a button keeps its meaning: the focus keeps moving, the volume keeps stepping, after
/// REPEAT_DELAY, accelerating down to REPEAT_FASTEST, until it is released (T1-062).
fn repeat_held(
    composer: &mut Composer,
    role: opendap_device::Role,
    meaning: opendap_input::semantic::Semantic,
) {
    use opendap_input::semantic::Semantic;
    composer.keys.repeat_gen += 1;
    let generation = composer.keys.repeat_gen;
    let mut interval = REPEAT_FIRST;
    let _ = composer.handle.insert_source(
        calloop::timer::Timer::from_duration(REPEAT_DELAY),
        move |_, _, composer| {
            if composer.keys.repeat_gen != generation
                || !composer.keys.forwarded.contains_key(&role)
            {
                return calloop::timer::TimeoutAction::Drop;
            }
            match meaning {
                Semantic::VolumeUp | Semantic::VolumeDown => info!(?role, "volume step, held"),
                _ => {
                    step_again(composer);
                    debug!(interval_ms = interval as u64, "focus step, held");
                }
            }
            let next = std::time::Duration::from_millis(interval as u64);
            interval = (interval * REPEAT_FACTOR).max(REPEAT_FASTEST);
            calloop::timer::TimeoutAction::ToDuration(next)
        },
    );
}

/// A role released: the keys its press sent are released, in reverse, and its repeat stops.
/// The focus stays where it is, with no step after (SYS-074).
pub(super) fn release_forwarded(
    composer: &mut Composer,
    role: opendap_device::Role,
    time: smithay::backend::input::InputTime,
) {
    let Some(keys) = composer.keys.forwarded.remove(&role) else {
        return;
    };
    composer.keys.repeat_gen += 1;
    crate::forward::release(role, &mut composer.keys.volume_presses, now_ms());
    for &code in keys.iter().rev() {
        send_key(composer, code, KeyState::Released, time);
    }
}

/// The system takes over (a combination, the power menu, the display going off): every key the
/// activity holds is released, every repeat stops, and its touches are cancelled (input events
/// §3.1: `pointercancel`; Wayland's `wl_touch.cancel`), so nothing that started acts.
pub fn release_all_forwarded(composer: &mut Composer) {
    let roles: Vec<_> = composer.keys.forwarded.keys().copied().collect();
    for role in roles {
        release_forwarded(composer, role, smithay::backend::input::InputTime::now());
    }
    composer.keys.key_repeat_gen += 1;
}

pub fn system_takeover(composer: &mut Composer) {
    release_all_forwarded(composer);
    // No double press spans the system's turn.
    composer
        .keys
        .volume_presses
        .iter_mut()
        .for_each(|p| p.reset());
    if let Some(touch) = composer.seat.get_touch() {
        touch.cancel(composer);
    }
    if let Some(layers) = composer.system.layers.as_mut() {
        let _ = layers.debug_contact(composer.contacts.last_touch, LayerContact::Cancel);
    }
}

/// The keyboard leaves the activity while the display is off, and comes back to it when it is on
/// again (Wayland's `wl_keyboard.leave` and `enter`): the activity's focus then releases every level
/// it had engaged (input events §3.7), and its focus is where it was when the display comes back.
pub fn activity_keyboard(composer: &mut Composer, on: bool) {
    let Some(keyboard) = composer.seat.get_keyboard() else {
        return;
    };
    let surface = if on {
        composer
            .space
            .elements()
            .last()
            .and_then(|w| w.toplevel().map(|t| t.wl_surface().clone()))
    } else {
        None
    };
    keyboard.set_focus(composer, surface, SERIAL_COUNTER.next_serial());
}
