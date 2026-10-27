//! The buttons in the list: the focus walks items (not slots), the list keeps it in view.

use std::{rc::Rc, time::Duration, time::Instant};

use opendap_input::nested;
use opendap_layout::extents::Extents;
use opendap_layout::{data::pages::ItemState, focus as vfocus};
use opendap_motion::animator::Animator;
use slint::{ComponentHandle, Timer};

use super::{HEAD, REVEAL, SECTION, Shared, VCOUNT, Virtual, render::vrender};
use crate::Testbed;

pub(super) fn wire(ui: &Testbed, st: Shared, timer: Rc<Timer>) {
    let weak = ui.as_weak();
    let clock = st.borrow().reveal.clock;
    ui.on_vkey(move |kind| {
        let Some(ui) = weak.upgrade() else {
            return false;
        };
        let handled = {
            let mut v = st.borrow_mut();
            match step(&mut v, kind.as_str()) {
                Some(
                    s
                    @ (vfocus::Step::Item(i) | vfocus::Step::Wrapped(i) | vfocus::Step::Jumped(i)),
                ) => {
                    // A wrap or a jump lands far away: the view goes there at once, never
                    // scrolling through everything between.
                    let far = !matches!(s, vfocus::Step::Item(_));
                    // Entering keeps a focus the list already had (a tap, a return).
                    let i = if kind.as_str() == "enter-down" {
                        v.focused.unwrap_or(i)
                    } else {
                        i
                    };
                    let reduce = ui.get_reduce_motion() || far;
                    if reveal(&mut v, i, clock, reduce) {
                        animate(&st, &ui, &timer, clock);
                    }
                    true
                }
                Some(vfocus::Step::Leave) => false,
                None => true,
            }
        };
        vrender(&st, &ui);
        handled
    });
}

/// Where `kind` moves the focus; `None` for what does not move it (Action, an unknown key).
fn step(v: &mut Virtual, kind: &str) -> Option<vfocus::Step> {
    match kind {
        "next" => Some(vfocus::step(v.focused, 1, VCOUNT, vfocus::Ends::Stop)),
        "prev" => Some(vfocus::step(v.focused, -1, VCOUNT, vfocus::Ends::Stop)),
        "enter-down" => Some(vfocus::step(None, 1, VCOUNT, vfocus::Ends::Stop)),
        "enter-up" => Some(vfocus::step(None, -1, VCOUNT, vfocus::Ends::Stop)),
        // A keyboard's Home and End, and the debug jump to an index (/run/opendap/debug-jump).
        "home" => Some(vfocus::jump(vfocus::Jump::Start, VCOUNT)),
        "end" => Some(vfocus::jump(vfocus::Jump::End, VCOUNT)),
        k if k.starts_with("jump:") => k["jump:".len()..]
            .parse()
            .ok()
            .map(|i| vfocus::jump(vfocus::Jump::Index(i), VCOUNT)),
        "activate" => {
            // A placeholder waits for its data, with the loading shown; then it is taken.
            if let Some(i) = v.focused {
                let ready = v.data.pages.item(i) == ItemState::Ready;
                if let Some(i) = v.activation.press(i, ready) {
                    v.activated = Some(i);
                }
            }
            None
        }
        _ => None,
    }
}

/// Focuses row `i` and starts the reveal's scroll to it; whether that scroll still runs.
fn reveal(v: &mut Virtual, i: usize, clock: Instant, reduce: bool) -> bool {
    // Moving away gives up a wait for data (input events §3.7).
    v.activation.moved(i);
    v.focused = Some(i);
    v.velocity = 0.0;
    // Revealed below the pinned header, from where the scroll is going (a held button retargets
    // it), animated unless motion is reduced.
    let now = clock.elapsed().as_secs_f32();
    let going = v.reveal.animator.sample(now).0;
    let from = if v.reveal.animator.running(now) {
        v.reveal.animator.target()
    } else {
        going
    };
    let max = (v.extents.total() - v.viewport).max(0.0);
    let mut area = [nested::RevealArea {
        scroller: nested::Scroller {
            offset: from,
            max,
            takes_first: false,
        },
        viewport: v.viewport,
        at: 0.0,
        inset_start: HEAD,
        inset_end: 0.0,
    }];
    let top = v.extents.offset(i) + if i.is_multiple_of(SECTION) { HEAD } else { 0.0 };
    let to = nested::reveal(&mut area, top, v.extents.offset(i) + v.extents.size(i))[0];
    if !v.reveal.animator.running(now) {
        v.reveal.animator = Animator::new(v.offset, REVEAL);
    }
    // After a new animator, not before: reduce motion (and a far move) must reach the one that
    // runs.
    v.reveal.animator.reduce_motion = reduce;
    v.reveal.animator.retarget(to, now, None);
    v.offset = v.reveal.animator.sample(now).0;
    v.reveal.animator.running(now)
}

/// Frames only while the reveal's scroll moves.
fn animate(st: &Shared, ui: &Testbed, timer: &Rc<Timer>, clock: Instant) {
    let (st, weak, t) = (st.clone(), ui.as_weak(), Rc::downgrade(timer));
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(16),
        move || {
            let Some(ui) = weak.upgrade() else { return };
            let now = clock.elapsed().as_secs_f32();
            let running = {
                let mut v = st.borrow_mut();
                v.offset = v.reveal.animator.sample(now).0;
                v.reveal.animator.running(now)
            };
            vrender(&st, &ui);
            if !running && let Some(t) = t.upgrade() {
                t.stop();
            }
        },
    );
}
