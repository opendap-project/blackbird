//! The pipeline's stages, each with one job (ADR 0014), and where a contact goes.

use super::*;

/// A contact that starts in the status bar is the system's (its swipe down, input events §3.2.1)
/// and never reaches the activity, down to its end: the content area begins at the status bar's
/// bottom line (input events §6.2), even while the activity's surface still covers the screen.
pub(super) fn system_contact(
    composer: &mut Composer,
    slot: TouchSlot,
    at: Point<f64, Logical>,
    contact: LayerContact,
) -> bool {
    match contact {
        LayerContact::Down => {
            let unit = composer.system.layers.as_ref().map_or(1.0, |l| l.unit());
            let in_bar = composer
                .system
                .layers
                .as_ref()
                .is_some_and(|l| l.status_shown())
                && (at.y as f32) < opendap_input::gesture::STATUS_BAR_PX * unit;
            if in_bar {
                composer.contacts.system_contacts.push(slot);
            }
            in_bar
        }
        LayerContact::Moved => composer.contacts.system_contacts.contains(&slot),
        LayerContact::Up | LayerContact::Cancel => {
            let len = composer.contacts.system_contacts.len();
            composer.contacts.system_contacts.retain(|s| *s != slot);
            composer.contacts.system_contacts.len() != len
        }
    }
}

/// With the display off, the screen takes no touch: only the buttons act (spec §1.3). The one
/// exception is a double tap, which wakes it (composer spec §2); nothing reaches a client.
pub(super) fn display_off(composer: &mut Composer, seen: &Seen) -> Flow {
    let off = composer.backend.as_ref().is_some_and(|b| !b.display_on());
    if !off || matches!(seen, Seen::Key { .. }) {
        return Flow::Next;
    }
    wake_gesture(composer, seen);
    Flow::Taken
}

/// The touch indicator follows every contact and the pointer; the debug UI mode shows contacts.
pub(super) fn observe(composer: &mut Composer, seen: &Seen) -> Flow {
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

pub(super) fn layer_contact(phase: Phase) -> LayerContact {
    match phase {
        Phase::Down => LayerContact::Down,
        Phase::Moved => LayerContact::Moved,
        Phase::Up => LayerContact::Up,
    }
}

/// The composer's own layers (a system view, the debug UI's panel) take what lands on them.
pub(super) fn layers(composer: &mut Composer, seen: &Seen) -> Flow {
    let taken = match *seen {
        Seen::Contact { phase, at, .. } => to_layers(composer, at, layer_contact(phase)),
        Seen::Hover { at, .. } => to_layers(composer, at, LayerContact::Moved),
        _ => false,
    };
    if taken { Flow::Taken } else { Flow::Next }
}

/// A contact that starts in the status bar is the system's, down to its end.
pub(super) fn system_contacts(composer: &mut Composer, seen: &Seen) -> Flow {
    match *seen {
        Seen::Contact {
            slot, phase, at, ..
        } if system_contact(composer, slot, at, layer_contact(phase)) => Flow::Taken,
        Seen::Cancel => {
            composer.contacts.system_contacts.clear();
            Flow::Next
        }
        _ => Flow::Next,
    }
}

/// The device's controls: the system's combinations first (controls.rs). A release first lets go
/// of what the press sent the activity, whatever the system does with it next, so no key stays down
/// and no repeat goes on.
pub(super) fn system_combinations(composer: &mut Composer, seen: &Seen) -> Flow {
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
    let taken = crate::linux::controls::control(composer, role, pressed);
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
pub(super) fn not_responding(composer: &mut Composer, seen: &Seen) -> Flow {
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
pub(super) fn activity(composer: &mut Composer, seen: &Seen) -> Flow {
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
        } => contact_to_client(composer, source, slot, phase, at, time),
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
pub(super) fn hover(
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
pub(super) fn to_layers(
    composer: &mut Composer,
    at: Point<f64, Logical>,
    contact: LayerContact,
) -> bool {
    composer.contacts.last_touch = at;
    let Some(layers) = composer.system.layers.as_mut().filter(|l| l.takes_input()) else {
        return false;
    };
    layers.contact(at, contact);
    crate::linux::controls::layer_requests(composer);
    if let Some(backend) = composer.backend.as_mut() {
        backend.schedule();
    }
    crate::linux::backend::render_if_idle(composer);
    true
}

/// A contact to the client surface under it (`wl_touch`).
fn contact_to_client(
    composer: &mut Composer,
    source: Source,
    slot: TouchSlot,
    phase: Phase,
    at: Point<f64, Logical>,
    time: smithay::backend::input::InputTime,
) {
    let Some(touch) = composer.seat.get_touch() else {
        return;
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
