//! The display-off double tap that wakes the display.

use super::*;

/// The display-off double tap: one finger (the first down), recognized by opendap-input's
/// `DoubleTap`, with its timing and slop (input events §7).
#[derive(Default)]
pub struct Wake {
    tap: Option<DoubleTap>,
    finger: Option<TouchSlot>,
    at: (f32, f32),
}

pub(super) fn wake_gesture(composer: &mut Composer, input: &Seen) {
    let unit = composer.system.layers.as_ref().map_or(1.0, |l| l.unit());
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
                if composer.contacts.wake.finger.is_some_and(|f| f != slot) {
                    return;
                }
                composer.contacts.wake.finger = Some(slot);
                composer.contacts.wake.at = (at.x as f32, at.y as f32);
                Input::Down {
                    at: composer.contacts.wake.at,
                    ms: ms(time),
                }
            }
            Phase::Moved => {
                if composer.contacts.wake.finger != Some(slot) {
                    return;
                }
                composer.contacts.wake.at = (at.x as f32, at.y as f32);
                Input::Move {
                    at: composer.contacts.wake.at,
                    ms: ms(time),
                }
            }
            Phase::Up => {
                if composer.contacts.wake.finger != Some(slot) {
                    return;
                }
                composer.contacts.wake.finger = None;
                Input::Up {
                    at: composer.contacts.wake.at,
                    ms: ms(time),
                }
            }
        },
        Seen::Cancel => {
            composer.contacts.wake.finger = None;
            Input::Cancel
        }
        _ => return,
    };
    let tap = composer
        .contacts
        .wake
        .tap
        .get_or_insert_with(|| DoubleTap::new(unit));
    let verdict = match tap.feed(input) {
        // Too late or too far for a second tap: this contact may be the first of a new one.
        Verdict::Reject if matches!(input, Input::Down { .. }) => {
            let mut fresh = DoubleTap::new(unit);
            let v = fresh.feed(input);
            composer.contacts.wake.tap = Some(fresh);
            v
        }
        v => v,
    };
    match verdict {
        Verdict::Accept => {
            composer.contacts.wake.tap = None;
            info!("display on, double tap");
            crate::linux::controls::set_display(composer, true);
        }
        Verdict::Reject => composer.contacts.wake.tap = None,
        Verdict::Possible => {}
    }
}
