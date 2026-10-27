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

mod debug;
mod forward;
mod keyboard;
mod liveness;
mod stages;
mod wake;

use debug::*;
use forward::*;
use keyboard::*;
use liveness::*;
use stages::*;

pub use forward::{activity_keyboard, release_all_forwarded, system_takeover};
pub use wake::Wake;
use wake::wake_gesture;

/// A contact's phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    Down,
    Moved,
    Up,
}

/// An input as the stages see it (system ADR 0014: stages, not branches): a key with its role, or
/// a contact, a finger's or the pointer's (an absolute pointer's primary button is a finger, in a
/// slot of its own), in one form, so a stage handles both once.
pub(super) enum Seen {
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
pub(super) enum Flow {
    /// Not its business, or only observed: the next stage gets it.
    Next,
    /// It took it: no stage after it sees it.
    Taken,
}

pub(super) type Stage = fn(&mut Composer, &Seen) -> Flow;

/// The stages, in order, each with one job (ADR 0014): a new condition is a new stage here, not a
/// branch in every path.
pub(super) const STAGES: &[(&str, Stage)] = &[
    ("display off", display_off),
    ("observe", observe),
    ("layers", layers),
    ("system contacts", system_contacts),
    ("system combinations", system_combinations),
    ("not responding", not_responding),
    ("activity", activity),
];

/// The pointer's slot: never a finger's.
pub(super) fn pointer_slot() -> TouchSlot {
    TouchSlot::from(Some(u32::MAX - 1))
}

/// libinput's event in the stages' form, keeping the pointer's own state (where it is, whether its
/// button is down) as it goes.
pub(super) fn normalize(
    composer: &mut Composer,
    event: InputEvent<LibinputInputBackend>,
) -> Option<Seen> {
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
        InputEvent::PointerMotionAbsolute { event } => pointer_motion(composer, &event)?,
        // The primary button is the finger; others mean nothing on a touch interface.
        InputEvent::PointerButton { event } if event.button_code() == 0x110 => {
            pointer_button(composer, &event)?
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

pub(super) fn touch_location<E: AbsolutePositionEvent<LibinputInputBackend>>(
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

/// The absolute pointer moved: a contact's motion while its button is down, a hover otherwise.
fn pointer_motion(
    composer: &mut Composer,
    event: &impl AbsolutePositionEvent<LibinputInputBackend>,
) -> Option<Seen> {
    let at = touch_location(composer, event)?;
    composer.pointer_location = at;
    Some(if composer.pointer_contact {
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
    })
}

/// The absolute pointer's primary button: the finger down or up, where the pointer is.
fn pointer_button(
    composer: &mut Composer,
    event: &impl PointerButtonEvent<LibinputInputBackend>,
) -> Option<Seen> {
    let at = composer.pointer_location;
    let phase = match event.state() {
        ButtonState::Pressed if !composer.pointer_contact => Phase::Down,
        ButtonState::Released if composer.pointer_contact => Phase::Up,
        _ => return None,
    };
    composer.pointer_contact = phase == Phase::Down;
    Some(Seen::Contact {
        source: Source::Pointer,
        slot: pointer_slot(),
        phase,
        at,
        time: event.time(),
    })
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
