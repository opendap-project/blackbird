//! The Gestures screen's pad: a contact followed while it lasts (opendap-input's Contact), so a
//! long press is named while the finger is still down and nothing follows when it lifts.

use std::{cell::RefCell, rc::Rc, time::Duration, time::Instant};

use opendap_input::gesture;
use slint::{ComponentHandle, Timer};

use crate::Testbed;

pub(super) fn wire(ui: &Testbed) {
    let contact: Rc<RefCell<Option<gesture::Contact>>> = Rc::default();
    let started = Instant::now();
    let long_timer = Rc::new(Timer::default());
    let weak = ui.as_weak();
    ui.on_pad(move |kind, x, y, w, h| {
        let Some(ui) = weak.upgrade() else { return };
        // The pad is a frame of its own: its regions are its edges and corners, not the screen's
        // (input events §3.2.1). Leading and trailing follow the layout's direction (design
        // system §8.5, §9.5).
        let surface = gesture::Frame {
            rtl: ui.get_strip_rtl(),
            ..gesture::Frame::new((w, h), ui.get_unit())
        };
        let now = started.elapsed().as_millis() as u64;
        match kind.as_str() {
            "down" => {
                ui.set_contacts(ui.get_contacts() + 1);
                *contact.borrow_mut() = Some(gesture::Contact::down((x, y), now, surface));
                let contact = contact.clone();
                let weak = ui.as_weak();
                long_timer.start(
                    slint::TimerMode::SingleShot,
                    Duration::from_millis(gesture::LONG_MS),
                    move || {
                        let Some(ui) = weak.upgrade() else { return };
                        let now = started.elapsed().as_millis() as u64;
                        if let Some(g) = contact.borrow_mut().as_mut().and_then(|c| c.long_due(now))
                        {
                            ui.set_gesture(g.describe().into());
                        }
                    },
                );
            }
            "move" => {
                // A pan or a drag, named live phase by phase.
                if let Some(g) = contact
                    .borrow_mut()
                    .as_mut()
                    .and_then(|c| c.moved((x, y), now))
                {
                    ui.set_gesture(g.describe().into());
                }
            }
            "up" => {
                long_timer.stop();
                if let Some(mut c) = contact.borrow_mut().take()
                    && let Some(g) = c.up((x, y), now).pop()
                {
                    ui.set_gesture(g.describe().into());
                }
            }
            _ => {
                long_timer.stop();
                let g = contact.borrow_mut().take().and_then(|mut c| c.cancel());
                ui.set_gesture(g.map_or("cancelled".into(), |g| g.describe()).into());
            }
        }
    });
}
