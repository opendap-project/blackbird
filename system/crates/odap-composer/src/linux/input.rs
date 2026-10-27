//! Input: touch goes to the surface under the finger; buttons become roles; an absolute pointer
//! (a simulator window, a USB tablet or mouse on any board) acts as one finger: the press is the
//! contact. Clients get no cursor; the touch indicator (touches.rs), when on, shows the pointer
//! and every contact.
//!
//! The device description maps Linux codes to roles (hardware support spec §4, "semantic
//! input"). This module only knows the Linux names of codes, the kernel's own table
//! (`linux/input-event-codes.h`), never which device has which button.

use opendap_input::arena::{DoubleTap, Input, Recognizer, Verdict};
use smithay::{
    backend::{
        input::{
            AbsolutePositionEvent, ButtonState, Event, InputEvent, KeyState, KeyboardKeyEvent,
            PointerButtonEvent, TouchEvent, TouchSlot,
        },
        libinput::LibinputInputBackend,
    },
    input::{
        keyboard::{FilterResult, Keycode},
        touch::{DownEvent, MotionEvent, UpEvent},
    },
    utils::{Logical, Point, SERIAL_COUNTER},
};
use tracing::{debug, info};

use super::{layers::Contact as LayerContact, state::Composer, touches::Source};

/// A contact that starts in the status bar is the system's (its swipe down, input events §3.2.1)
/// and never reaches the activity, down to its end: the content area begins at the status bar's
/// bottom line (input events §6.2), even while the activity's surface still covers the screen.
fn system_contact(
    composer: &mut Composer,
    slot: TouchSlot,
    at: Point<f64, Logical>,
    contact: LayerContact,
) -> bool {
    match contact {
        LayerContact::Down => {
            let unit = composer.layers.as_ref().map_or(1.0, |l| l.unit());
            let in_bar = composer.layers.as_ref().is_some_and(|l| l.status_shown())
                && (at.y as f32) < opendap_input::gesture::STATUS_BAR_PX * unit;
            if in_bar {
                composer.system_contacts.push(slot);
            }
            in_bar
        }
        LayerContact::Moved => composer.system_contacts.contains(&slot),
        LayerContact::Up | LayerContact::Cancel => {
            let len = composer.system_contacts.len();
            composer.system_contacts.retain(|s| *s != slot);
            composer.system_contacts.len() != len
        }
    }
}

/// How long an activity may leave a ping unanswered before it counts as not responding.
const UNRESPONSIVE: std::time::Duration = std::time::Duration::from_millis(500);

/// Whether the activity responds: a ping (xdg_wm_base) sent after a key it was given and left
/// unanswered past `UNRESPONSIVE` means it is busy with that key (a hang), and the next key is
/// dropped rather than queued, so nothing acts seconds later by surprise
/// (SYS-072). The ping goes after the key, so the activity
/// meets the key first. Nothing is sent at rest: no keys, no pings.
fn activity_responds(composer: &Composer) -> bool {
    composer
        .ping_since
        .is_none_or(|since| since.elapsed() < UNRESPONSIVE)
}

/// Pings the activity a moment after a key (unless a ping is out): a client answers pings while it
/// reads its events, before its handlers run, so a ping sent with the key would be answered before
/// the handler that hangs; 100 ms later, a responsive activity answers within milliseconds and a
/// hung one does not. The timer exists only after input: nothing runs at rest.
fn ping_activity(composer: &mut Composer) {
    if composer.ping_since.is_some() || composer.ping_scheduled {
        return;
    }
    composer.ping_scheduled = true;
    let _ = composer.handle.insert_source(
        calloop::timer::Timer::from_duration(std::time::Duration::from_millis(100)),
        |_, _, composer| {
            composer.ping_scheduled = false;
            if composer.ping_since.is_none() {
                let client = composer
                    .space
                    .elements()
                    .last()
                    .and_then(|w| w.toplevel().map(|t| t.client()));
                if let Some(client) = client
                    && client.send_ping(SERIAL_COUNTER.next_serial()).is_ok()
                {
                    composer.ping_since = Some(std::time::Instant::now());
                    composer.pinged = Some(client);
                }
            }
            calloop::timer::TimeoutAction::Drop
        },
    );
}

/// The display-off double tap: one finger (the first down), recognized by opendap-input's
/// `DoubleTap`, with its timing and slop (input events §7).
#[derive(Default)]
pub struct Wake {
    tap: Option<DoubleTap>,
    finger: Option<TouchSlot>,
    at: (f32, f32),
}

fn wake_gesture(composer: &mut Composer, input: &Seen) {
    let unit = composer.layers.as_ref().map_or(1.0, |l| l.unit());
    let ms = |t: &smithay::backend::input::InputTime| t.micros() / 1000;
    let input = match *input {
        Seen::Contact {
            slot,
            phase,
            at,
            ref time,
            ..
        } => match phase {
            Phase::Down => {
                if composer.wake.finger.is_some_and(|f| f != slot) {
                    return;
                }
                composer.wake.finger = Some(slot);
                composer.wake.at = (at.x as f32, at.y as f32);
                Input::Down {
                    at: composer.wake.at,
                    ms: ms(time),
                }
            }
            Phase::Moved => {
                if composer.wake.finger != Some(slot) {
                    return;
                }
                composer.wake.at = (at.x as f32, at.y as f32);
                Input::Move {
                    at: composer.wake.at,
                    ms: ms(time),
                }
            }
            Phase::Up => {
                if composer.wake.finger != Some(slot) {
                    return;
                }
                composer.wake.finger = None;
                Input::Up {
                    at: composer.wake.at,
                    ms: ms(time),
                }
            }
        },
        Seen::Cancel => {
            composer.wake.finger = None;
            Input::Cancel
        }
        _ => return,
    };
    let tap = composer
        .wake
        .tap
        .get_or_insert_with(|| DoubleTap::new(unit));
    let verdict = match tap.feed(input) {
        // Too late or too far for a second tap: this contact may be the first of a new one.
        Verdict::Reject if matches!(input, Input::Down { .. }) => {
            let mut fresh = DoubleTap::new(unit);
            let v = fresh.feed(input);
            composer.wake.tap = Some(fresh);
            v
        }
        v => v,
    };
    match verdict {
        Verdict::Accept => {
            composer.wake.tap = None;
            info!("display on, double tap");
            super::controls::set_display(composer, true);
        }
        Verdict::Reject => composer.wake.tap = None,
        Verdict::Possible => {}
    }
}

/// A contact's phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Down,
    Moved,
    Up,
}

/// An input as the stages see it (system ADR 0014: stages, not branches): a key with its role, or
/// a contact, a finger's or the pointer's (an absolute pointer's primary button is a finger, in a
/// slot of its own), in one form, so a stage handles both once.
enum Seen {
    Key {
        code: u32,
        pressed: bool,
        role: Option<opendap_device::Role>,
        time: smithay::backend::input::InputTime,
    },
    Contact {
        source: Source,
        slot: TouchSlot,
        phase: Phase,
        at: Point<f64, Logical>,
        time: smithay::backend::input::InputTime,
    },
    /// The pointer moving with no contact: hover, never a touch.
    Hover {
        at: Point<f64, Logical>,
        time: smithay::backend::input::InputTime,
    },
    /// The end of a group of finger events.
    Frame,
    /// The kernel took every contact back.
    Cancel,
}

/// What a stage did with an input.
enum Flow {
    /// Not its business, or only observed: the next stage gets it.
    Next,
    /// It took it: no stage after it sees it.
    Taken,
}

type Stage = fn(&mut Composer, &Seen) -> Flow;

/// The stages, in order, each with one job (ADR 0014): a new condition is a new stage here, not a
/// branch in every path.
const STAGES: &[(&str, Stage)] = &[
    ("display off", display_off),
    ("observe", observe),
    ("layers", layers),
    ("system contacts", system_contacts),
    ("system combinations", system_combinations),
    ("not responding", not_responding),
    ("activity", activity),
];

/// The pointer's slot: never a finger's.
fn pointer_slot() -> TouchSlot {
    TouchSlot::from(Some(u32::MAX - 1))
}

/// libinput's event in the stages' form, keeping the pointer's own state (where it is, whether its
/// button is down) as it goes.
fn normalize(composer: &mut Composer, event: InputEvent<LibinputInputBackend>) -> Option<Seen> {
    Some(match event {
        InputEvent::Keyboard { event } => {
            // xkb keycodes are Linux codes plus 8.
            let code = event.key_code().raw().saturating_sub(8);
            let role = linux_key_name(code).and_then(|name| composer.description.role_of(name));
            Seen::Key {
                code,
                pressed: event.state() == KeyState::Pressed,
                role,
                time: event.time(),
            }
        }
        InputEvent::TouchDown { event } => {
            let at = touch_location(composer, &event)?;
            Seen::Contact {
                source: Source::Finger(event.slot()),
                slot: event.slot(),
                phase: Phase::Down,
                at,
                time: event.time(),
            }
        }
        InputEvent::TouchMotion { event } => {
            let at = touch_location(composer, &event)?;
            Seen::Contact {
                source: Source::Finger(event.slot()),
                slot: event.slot(),
                phase: Phase::Moved,
                at,
                time: event.time(),
            }
        }
        // A lift has no place of its own: where the contact last was.
        InputEvent::TouchUp { event } => Seen::Contact {
            source: Source::Finger(event.slot()),
            slot: event.slot(),
            phase: Phase::Up,
            at: composer.last_touch,
            time: event.time(),
        },
        InputEvent::TouchFrame { .. } => Seen::Frame,
        InputEvent::TouchCancel { .. } => Seen::Cancel,
        InputEvent::PointerMotionAbsolute { event } => {
            let at = touch_location(composer, &event)?;
            composer.pointer_location = at;
            if composer.pointer_contact {
                Seen::Contact {
                    source: Source::Pointer,
                    slot: pointer_slot(),
                    phase: Phase::Moved,
                    at,
                    time: event.time(),
                }
            } else {
                Seen::Hover {
                    at,
                    time: event.time(),
                }
            }
        }
        // The primary button is the finger; others mean nothing on a touch interface.
        InputEvent::PointerButton { event } if event.button_code() == 0x110 => {
            let at = composer.pointer_location;
            let phase = match event.state() {
                ButtonState::Pressed if !composer.pointer_contact => Phase::Down,
                ButtonState::Released if composer.pointer_contact => Phase::Up,
                _ => return None,
            };
            composer.pointer_contact = phase == Phase::Down;
            Seen::Contact {
                source: Source::Pointer,
                slot: pointer_slot(),
                phase,
                at,
                time: event.time(),
            }
        }
        _ => return None,
    })
}

pub fn process(composer: &mut Composer, event: InputEvent<LibinputInputBackend>) {
    let Some(seen) = normalize(composer, event) else {
        return;
    };
    for (_, stage) in STAGES {
        if let Flow::Taken = stage(composer, &seen) {
            return;
        }
    }
}

/// With the display off, the screen takes no touch: only the buttons act (spec §1.3). The one
/// exception is a double tap, which wakes it (composer spec §2); nothing reaches a client.
fn display_off(composer: &mut Composer, seen: &Seen) -> Flow {
    let off = composer.backend.as_ref().is_some_and(|b| !b.display_on());
    if !off || matches!(seen, Seen::Key { .. }) {
        return Flow::Next;
    }
    wake_gesture(composer, seen);
    Flow::Taken
}

/// The touch indicator follows every contact and the pointer; the debug UI mode shows contacts.
fn observe(composer: &mut Composer, seen: &Seen) -> Flow {
    match *seen {
        Seen::Contact {
            source, phase, at, ..
        } => {
            match phase {
                Phase::Down => indicate(composer, |t| t.down(source, at)),
                Phase::Moved => indicate(composer, |t| t.moved(source, at)),
                Phase::Up => indicate(composer, |t| t.up(source)),
            }
            debug(composer, at, layer_contact(phase));
        }
        Seen::Hover { at, .. } => indicate(composer, |t| t.moved(Source::Pointer, at)),
        _ => {}
    }
    Flow::Next
}

fn layer_contact(phase: Phase) -> LayerContact {
    match phase {
        Phase::Down => LayerContact::Down,
        Phase::Moved => LayerContact::Moved,
        Phase::Up => LayerContact::Up,
    }
}

/// The composer's own layers (a system view, the debug UI's panel) take what lands on them.
fn layers(composer: &mut Composer, seen: &Seen) -> Flow {
    let taken = match *seen {
        Seen::Contact { phase, at, .. } => to_layers(composer, at, layer_contact(phase)),
        Seen::Hover { at, .. } => to_layers(composer, at, LayerContact::Moved),
        _ => false,
    };
    if taken { Flow::Taken } else { Flow::Next }
}

/// A contact that starts in the status bar is the system's, down to its end.
fn system_contacts(composer: &mut Composer, seen: &Seen) -> Flow {
    match *seen {
        Seen::Contact {
            slot, phase, at, ..
        } if system_contact(composer, slot, at, layer_contact(phase)) => Flow::Taken,
        Seen::Cancel => {
            composer.system_contacts.clear();
            Flow::Next
        }
        _ => Flow::Next,
    }
}

/// The device's controls: the system's combinations first (controls.rs). A release first lets go
/// of what the press sent the activity, whatever the system does with it next, so no key stays down
/// and no repeat goes on.
fn system_combinations(composer: &mut Composer, seen: &Seen) -> Flow {
    let Seen::Key {
        pressed,
        role: Some(role),
        time,
        ..
    } = *seen
    else {
        return Flow::Next;
    };
    if !pressed {
        release_forwarded(composer, role, time);
    }
    let taken = super::controls::control(composer, role, pressed);
    if taken && pressed {
        // The system takes this press: nothing the activity holds goes on.
        release_all_forwarded(composer);
    }
    if taken || !pressed {
        Flow::Taken
    } else {
        Flow::Next
    }
}

/// An activity that does not answer a ping sent after its last key is busy with it (a hang): the
/// next key is dropped rather than queued, so nothing acts seconds later by surprise.
fn not_responding(composer: &mut Composer, seen: &Seen) -> Flow {
    let Seen::Key {
        pressed: true,
        role,
        ..
    } = *seen
    else {
        return Flow::Next;
    };
    if activity_responds(composer) {
        return Flow::Next;
    }
    if role.is_some() {
        info!("control dropped: the activity does not respond");
    } else {
        info!("key dropped: the activity does not respond");
    }
    Flow::Taken
}

/// The activity: a control as its role's key, any other key as is (a keyboard attached to the
/// device, the simulator's arrows), a contact to the surface under it, the pointer's hover.
fn activity(composer: &mut Composer, seen: &Seen) -> Flow {
    match *seen {
        Seen::Key {
            role: Some(role),
            time,
            ..
        } => {
            info!(?role, "control");
            forward(composer, role, time);
            ping_activity(composer);
        }
        Seen::Key {
            code,
            pressed,
            role: None,
            time,
        } => {
            let state = if pressed {
                KeyState::Pressed
            } else {
                KeyState::Released
            };
            send_key(composer, code, state, time);
            if pressed {
                ping_activity(composer);
            }
            keyboard_repeat(composer, code, pressed);
        }
        Seen::Contact {
            source,
            slot,
            phase,
            at,
            time,
        } => {
            let Some(touch) = composer.seat.get_touch() else {
                return Flow::Taken;
            };
            match phase {
                Phase::Down => {
                    let under = composer.surface_under(at);
                    touch.down(
                        composer,
                        under,
                        &DownEvent {
                            slot,
                            location: at,
                            serial: SERIAL_COUNTER.next_serial(),
                            time,
                        },
                    );
                }
                Phase::Moved => {
                    let under = composer.surface_under(at);
                    touch.motion(
                        composer,
                        under,
                        &MotionEvent {
                            slot,
                            location: at,
                            time,
                        },
                    );
                }
                Phase::Up => touch.up(
                    composer,
                    &UpEvent {
                        slot,
                        serial: SERIAL_COUNTER.next_serial(),
                        time,
                    },
                ),
            }
            // The pointer has no frame events of its own: each of its contacts is one.
            if source == Source::Pointer {
                touch.frame(composer);
            }
        }
        Seen::Hover { at, time } => hover(composer, at, time),
        Seen::Frame => {
            if let Some(touch) = composer.seat.get_touch() {
                touch.frame(composer);
            }
        }
        Seen::Cancel => {
            if let Some(touch) = composer.seat.get_touch() {
                touch.cancel(composer);
            }
        }
    }
    Flow::Taken
}

/// The pointer over a client, without contact: `wl_pointer` motion, for hover states only.
fn hover(
    composer: &mut Composer,
    location: Point<f64, Logical>,
    time: smithay::backend::input::InputTime,
) {
    let under = composer.surface_under(location);
    let Some(pointer) = composer.seat.get_pointer() else {
        return;
    };
    pointer.motion(
        composer,
        under,
        &smithay::input::pointer::MotionEvent {
            location,
            serial: SERIAL_COUNTER.next_serial(),
            time,
        },
    );
    pointer.frame(composer);
}

/// A contact for the system layers while one takes input (spec §7: then no client gets it).
/// Returns whether the layers took it.
fn to_layers(composer: &mut Composer, at: Point<f64, Logical>, contact: LayerContact) -> bool {
    composer.last_touch = at;
    let Some(layers) = composer.layers.as_mut().filter(|l| l.takes_input()) else {
        return false;
    };
    layers.contact(at, contact);
    super::controls::layer_requests(composer);
    if let Some(backend) = composer.backend.as_mut() {
        backend.schedule();
    }
    super::backend::render_if_idle(composer);
    true
}

// Linux key codes (linux/input-event-codes.h) the activity receives.
const KEY_NEXTSONG: u32 = 163;
const KEY_PREVIOUSSONG: u32 = 165;
const KEY_TAB: u32 = 15;
const KEY_ENTER: u32 = 28;
const KEY_LEFTSHIFT: u32 = 42;

/// A held focus step's repeat accelerates exponentially and stops where it is released, with no
/// inertia (SYS-073): after Android's
/// key-repeat delay, the first step comes after REPEAT_FIRST, and each next interval is
/// REPEAT_FACTOR of the last, down to REPEAT_FASTEST. Precise at first, fast through long lists.
/// SYS-066 asks for these to be adjustable.
const REPEAT_DELAY: std::time::Duration =
    std::time::Duration::from_millis(opendap_input::thresholds::REPEAT_DELAY_MS);
const REPEAT_FIRST: f64 = opendap_input::thresholds::REPEAT_FIRST_MS;
const REPEAT_FACTOR: f64 = opendap_input::thresholds::REPEAT_FACTOR;
const REPEAT_FASTEST: f64 = opendap_input::thresholds::REPEAT_FASTEST_MS;

/// One more Tab while a volume button is held in focus navigation: Tab released and pressed
/// again, Shift (for Volume +) still down.
fn step_again(composer: &mut Composer) {
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
fn now_ms() -> u64 {
    static EPOCH: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    EPOCH
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_millis() as u64
}

/// Whether sound is playing: any ALSA playback stream running (the cards' `status` files).
fn sound_playing() -> bool {
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

/// A role the system combinations left to the activity, pressed (T1 controls spec §1.3).
/// Volume ± is the volume while sound plays or the activity asks for it, and otherwise moves the
/// focus: Volume − to the next element, Volume + to the previous (Tab, Shift+Tab). Action takes
/// the focused element (Enter), or the screen's primary action where it declares one (Blackbird
/// UI's components). With the display off nothing reaches the focus: Action is play/pause and
/// Volume the volume (§1.3, T1-060). Main stays the system's. The keys chosen at the press are
/// the ones released ([`release_forwarded`]).
fn forward(
    composer: &mut Composer,
    role: opendap_device::Role,
    time: smithay::backend::input::InputTime,
) {
    use opendap_device::Role;
    use opendap_input::semantic::{self, Semantic};
    let display_off = composer.backend.as_ref().is_some_and(|b| !b.display_on());
    let Some(input_role) = (match role {
        Role::Action => Some(semantic::Role::Action),
        Role::VolumeUp => Some(semantic::Role::VolumeUp),
        Role::VolumeDown => Some(semantic::Role::VolumeDown),
        _ => None,
    }) else {
        return;
    };
    // What the press means here (input events §3.4). The composer does not see the activity's focus
    // or locks: the components refine Activate into the primary action and Next/Previous into a
    // value's steps (Blackbird UI).
    let ctx = semantic::Context {
        display_on: !display_off,
        volume_wanted: !display_off && sound_playing(),
        ..Default::default()
    };
    let meaning = semantic::press(input_role, ctx);
    let keys: &'static [u32] = match meaning {
        // No audio engine yet to take these (system ADR 0002 odap-audiod).
        Semantic::PlayPause => {
            info!("play/pause, display off");
            &[]
        }
        // The volume: a step as the button goes down, repeated while it is held (as the focus
        // is); a second press within the double-press time takes the first one's step back and
        // changes the track (Volume + next, Volume − previous: T1-062, T1-063), as media keys
        // to the activity until the audio engine takes them. No press
        // waits for another.
        Semantic::VolumeUp | Semantic::VolumeDown => {
            let up = meaning == Semantic::VolumeUp;
            match composer.volume_presses[usize::from(!up)].down(now_ms()) {
                opendap_input::double::Down::First => info!(?role, "volume step"),
                opendap_input::double::Down::Second => {
                    info!(?role, "volume step undone");
                    let key = match semantic::double_press(input_role, ctx) {
                        Semantic::NextTrack => KEY_NEXTSONG,
                        _ => KEY_PREVIOUSSONG,
                    };
                    info!(?role, "track, double press");
                    let now = smithay::backend::input::InputTime::now();
                    send_key(composer, key, KeyState::Pressed, now);
                    send_key(composer, key, KeyState::Released, now);
                    // The double's press is spent: it does not repeat.
                    composer.forwarded.insert(role, &[]);
                    return;
                }
            }
            &[]
        }
        Semantic::Next => &[KEY_TAB],
        Semantic::Previous => &[KEY_LEFTSHIFT, KEY_TAB],
        Semantic::Activate | Semantic::Primary => &[KEY_ENTER],
        _ => &[],
    };
    composer.forwarded.insert(role, keys);
    // Held, a volume button keeps its meaning: the focus keeps moving, the volume keeps
    // stepping, after REPEAT_DELAY, accelerating down to REPEAT_FASTEST, until it is released
    // (T1-062). Action never repeats.
    if semantic::repeats(input_role, ctx) {
        composer.repeat_gen += 1;
        let generation = composer.repeat_gen;
        let mut interval = REPEAT_FIRST;
        let _ = composer.handle.insert_source(
            calloop::timer::Timer::from_duration(REPEAT_DELAY),
            move |_, _, composer| {
                if composer.repeat_gen != generation || !composer.forwarded.contains_key(&role) {
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
    for &code in keys {
        send_key(composer, code, KeyState::Pressed, time);
    }
}

/// A role released: the keys its press sent are released, in reverse, and its repeat stops.
/// The focus stays where it is, with no step after (SYS-074).
fn release_forwarded(
    composer: &mut Composer,
    role: opendap_device::Role,
    time: smithay::backend::input::InputTime,
) {
    let Some(keys) = composer.forwarded.remove(&role) else {
        return;
    };
    composer.repeat_gen += 1;
    match role {
        opendap_device::Role::VolumeUp => composer.volume_presses[0].up(now_ms()),
        opendap_device::Role::VolumeDown => composer.volume_presses[1].up(now_ms()),
        _ => {}
    }
    for &code in keys.iter().rev() {
        send_key(composer, code, KeyState::Released, time);
    }
}

/// The system takes over (a combination, the power menu, the display going off): every key the
/// activity holds is released, every repeat stops, and its touches are cancelled (input events
/// §3.1: `pointercancel`; Wayland's `wl_touch.cancel`), so nothing that started acts.
pub fn release_all_forwarded(composer: &mut Composer) {
    let roles: Vec<_> = composer.forwarded.keys().copied().collect();
    for role in roles {
        release_forwarded(composer, role, smithay::backend::input::InputTime::now());
    }
    composer.key_repeat_gen += 1;
}

pub fn system_takeover(composer: &mut Composer) {
    release_all_forwarded(composer);
    // No double press spans the system's turn.
    composer.volume_presses.iter_mut().for_each(|p| p.reset());
    if let Some(touch) = composer.seat.get_touch() {
        touch.cancel(composer);
    }
    if let Some(layers) = composer.layers.as_mut() {
        let _ = layers.debug_contact(composer.last_touch, LayerContact::Cancel);
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

/// One key to the activity (xkb keycodes are Linux codes plus 8).
fn send_key(
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
const KEY_REPEAT_DELAY: std::time::Duration =
    std::time::Duration::from_millis(opendap_input::thresholds::KEY_REPEAT_DELAY_MS);
const KEY_REPEAT_EVERY: std::time::Duration =
    std::time::Duration::from_millis(opendap_input::thresholds::KEY_REPEAT_EVERY_MS);
const MODIFIERS: &[u32] = &[29, 42, 54, 56, 97, 100, 125, 126, 58];
/// Keys whose meaning never repeats (input events §5.1: activate, primary and back): Escape,
/// Enter, Space, the keypad's Enter.
const NO_REPEAT: &[u32] = &[1, 28, 57, 96];

fn keyboard_repeat(composer: &mut Composer, code: u32, pressed: bool) {
    composer.key_repeat_gen += 1;
    if !pressed || MODIFIERS.contains(&code) || NO_REPEAT.contains(&code) {
        return;
    }
    let generation = composer.key_repeat_gen;
    let _ = composer.handle.insert_source(
        calloop::timer::Timer::from_duration(KEY_REPEAT_DELAY),
        move |_, _, composer| {
            if composer.key_repeat_gen != generation {
                return calloop::timer::TimeoutAction::Drop;
            }
            let now = smithay::backend::input::InputTime::now();
            send_key(composer, code, KeyState::Released, now);
            send_key(composer, code, KeyState::Pressed, now);
            calloop::timer::TimeoutAction::ToDuration(KEY_REPEAT_EVERY)
        },
    );
}

/// How long the debug UI mode shows a gesture after its contact ends: feedback for the eye, not a
/// message to read (SYS-075). It stays while the contact lasts, and a newer
/// gesture replaces it at once.
const GESTURE_SHOWN: std::time::Duration = std::time::Duration::from_millis(800);

/// Tells the debug UI mode, when it is on, of a contact: its crosshair and region, then its
/// gesture, shown for a while.
fn debug(composer: &mut Composer, at: Point<f64, Logical>, contact: LayerContact) {
    // A long press is named at LONG_MS while the finger is still down.
    if matches!(contact, LayerContact::Down) && super::layers::debug_ui() {
        let _ = composer.handle.insert_source(
            calloop::timer::Timer::from_duration(std::time::Duration::from_millis(
                opendap_input::gesture::LONG_MS,
            )),
            |_, _, composer| {
                if let Some(layers) = composer.layers.as_mut() {
                    // Shown while the finger stays down; its timeout starts when it lifts.
                    layers.debug_long_press();
                    if let Some(b) = composer.backend.as_mut() {
                        b.schedule();
                    }
                    super::backend::render_if_idle(composer);
                }
                calloop::timer::TimeoutAction::Drop
            },
        );
    }
    let Some(layers) = composer.layers.as_mut() else {
        return;
    };
    let changed = layers.refresh_debug();
    let Some(seq) = layers.debug_contact(at, contact).or(changed.then_some(0)) else {
        if super::layers::debug_ui() {
            if let Some(backend) = composer.backend.as_mut() {
                backend.schedule();
            }
            super::backend::render_if_idle(composer);
        }
        return;
    };
    if seq > 0 {
        clear_later(composer, seq);
    }
    if let Some(backend) = composer.backend.as_mut() {
        backend.schedule();
    }
    super::backend::render_if_idle(composer);
}

/// The gesture's callout leaves after GESTURE_SHOWN.
fn clear_later(composer: &mut Composer, seq: u64) {
    {
        let _ = composer.handle.insert_source(
            calloop::timer::Timer::from_duration(GESTURE_SHOWN),
            move |_, _, composer| {
                if let Some(l) = composer.layers.as_mut() {
                    l.clear_gesture(seq);
                }
                if let Some(b) = composer.backend.as_mut() {
                    b.schedule();
                }
                super::backend::render_if_idle(composer);
                calloop::timer::TimeoutAction::Drop
            },
        );
    }
}

/// Tells the touch indicator, when it is on, and draws its change.
fn indicate(composer: &mut Composer, change: impl FnOnce(&mut super::touches::Touches)) {
    if !composer.touches.enabled() {
        return;
    }
    change(&mut composer.touches);
    if let Some(backend) = composer.backend.as_mut() {
        backend.schedule();
    }
    super::backend::render_if_idle(composer);
}

fn touch_location<E: AbsolutePositionEvent<LibinputInputBackend>>(
    composer: &Composer,
    event: &E,
) -> Option<smithay::utils::Point<f64, smithay::utils::Logical>> {
    let output = composer.output.as_ref()?;
    let geometry = composer.space.output_geometry(output)?;
    Some(event.position_transformed(geometry.size) + geometry.loc.to_f64())
}

/// The Linux name of a key code (`linux/input-event-codes.h`), as device descriptions write it.
pub fn linux_key_name(code: u32) -> Option<&'static str> {
    super::key_names::KEY_NAMES
        .binary_search_by_key(&code, |&(c, _)| c)
        .ok()
        .map(|i| super::key_names::KEY_NAMES[i].1)
}

#[cfg(test)]
mod tests {
    use super::linux_key_name;

    #[test]
    fn names_match_the_kernel_table() {
        assert_eq!(linux_key_name(164), Some("KEY_PLAYPAUSE"));
        assert_eq!(linux_key_name(115), Some("KEY_VOLUMEUP"));
        assert_eq!(linux_key_name(114), Some("KEY_VOLUMEDOWN"));
        assert_eq!(linux_key_name(116), Some("KEY_POWER"));
        assert_eq!(linux_key_name(57), Some("KEY_SPACE"));
        assert_eq!(linux_key_name(25), Some("KEY_P"));
        assert_eq!(linux_key_name(0), None);
    }
}
