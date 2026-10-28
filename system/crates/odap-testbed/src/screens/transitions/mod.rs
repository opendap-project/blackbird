//! The Transitions screen: a pager (opendap-layout::pager), a morph between a tile and its detail
//! (opendap-motion::transition and interpolate, opendap-input::matched), enter and exit
//! (opendap-motion::presence). Frames only while something moves.

mod render;
mod state;

use std::{cell::RefCell, rc::Rc, time::Duration, time::Instant};

use slint::{ComponentHandle, Timer};

use crate::{
    Testbed,
    keys::{Arrows, Router},
};
use render::tr_render;
use state::Tr;

type Shared = Rc<RefCell<Tr>>;

pub(super) fn wire(ui: &Testbed, keys: &mut Router<Testbed>) {
    let tr: Shared = Rc::new(RefCell::new(Tr::new()));
    let clock = Instant::now();
    tr_render(ui, &mut tr.borrow_mut(), 0.0);
    let kick = kicker(ui, &tr, clock);
    {
        let (weak, tr, kick) = (ui.as_weak(), tr.clone(), kick.clone());
        ui.on_tr_pager(move |kind, x| {
            let Some(ui) = weak.upgrade() else { return };
            let now = clock.elapsed().as_secs_f32();
            let rtl = ui.get_strip_rtl();
            tr.borrow_mut()
                .pager_touch(kind.as_str(), if rtl { -x } else { x }, now);
            tr_render(&ui, &mut tr.borrow_mut(), now);
            kick();
        });
    }
    {
        let (weak, tr, kick) = (ui.as_weak(), tr.clone(), kick.clone());
        ui.on_tr_tile(move |i| {
            let Some(ui) = weak.upgrade() else { return };
            let now = clock.elapsed().as_secs_f32();
            tr.borrow_mut()
                .open(i as usize, now, ui.get_reduce_motion());
            kick();
        });
    }
    {
        let (weak, tr, kick) = (ui.as_weak(), tr.clone(), kick.clone());
        ui.on_tr_detail_touch(move |kind, y| {
            let Some(ui) = weak.upgrade() else { return };
            let now = clock.elapsed().as_secs_f32();
            tr.borrow_mut().detail_touch(kind.as_str(), y, now);
            tr_render(&ui, &mut tr.borrow_mut(), now);
            kick();
        });
    }
    {
        let (weak, tr, kick) = (ui.as_weak(), tr.clone(), kick.clone());
        ui.on_tr_list(move || {
            let Some(ui) = weak.upgrade() else { return };
            let now = clock.elapsed().as_secs_f32();
            tr.borrow_mut().toggle_line(now, ui.get_reduce_motion());
            kick();
        });
    }
    // The pager and the morph under the buttons (KeyControl): the same primitives the touch uses.
    keys.add(&["pager", "morph"], Arrows::Raw, move |ui, name, key| {
        let now = clock.elapsed().as_secs_f32();
        let handled =
            tr.borrow_mut()
                .key(name, key, now, ui.get_reduce_motion(), ui.get_strip_rtl());
        kick();
        handled
    });
}

/// Starts the frames, if they are not running, until nothing moves.
fn kicker(ui: &Testbed, tr: &Shared, clock: Instant) -> Rc<dyn Fn()> {
    // The kick owns the timer: `wire` returns once the screens are wired, and a timer only a
    // weak reference holds would be gone with it (the pager then never moved). Its callback holds
    // it weakly, so the two do not keep each other.
    let (weak, tr, timer) = (ui.as_weak(), tr.clone(), Rc::new(Timer::default()));
    Rc::new(move || {
        let t = &timer;
        if t.running() {
            return;
        }
        let (weak, tr, timer2) = (weak.clone(), tr.clone(), Rc::downgrade(&timer));
        t.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(16),
            move || {
                let Some(ui) = weak.upgrade() else { return };
                let now = clock.elapsed().as_secs_f32();
                let running = tr_render(&ui, &mut tr.borrow_mut(), now);
                if !running && let Some(t) = timer2.upgrade() {
                    t.stop();
                }
            },
        );
    })
}
