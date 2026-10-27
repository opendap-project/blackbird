//! The list's own pan: a tap focuses the row under it, a drag scrolls, a throw flings.

use std::{rc::Rc, time::Duration, time::Instant};

use opendap_input::thresholds::{FLING_PX_S, SLOP_PX};
use opendap_layout::extents::Extents;
use opendap_motion::{animator::Animator, decay::Decay};
use slint::{ComponentHandle, Timer};

use super::{HEAD, REVEAL, Shared, Virtual, render::vrender};
use crate::Testbed;

pub(super) fn wire(ui: &Testbed, st: Shared, timer: Rc<Timer>) {
    let started = Instant::now();
    let weak = ui.as_weak();
    ui.on_vpan(move |kind, y, h| {
        let Some(ui) = weak.upgrade() else { return };
        {
            let mut v = st.borrow_mut();
            let ms = started.elapsed().as_millis() as u64;
            if h > 0.0 {
                v.viewport = h;
            }
            match kind.as_str() {
                "down" => {
                    // A touch stops the reveal's scroll, or a fling, where it is.
                    timer.stop();
                    down(&mut v, y, ms);
                    return;
                }
                // A move, or the release with the last stretch it carries.
                "move" | "up" => {
                    if moved(&mut v, &ui, kind.as_str() == "up", y, ms) {
                        fling(&st, &ui, &timer);
                    }
                }
                // Opened, or cancelled (no tap): the scroll is still; costly loads may run.
                _ => v.velocity = 0.0,
            }
        }
        vrender(&st, &ui);
    });
}

fn down(v: &mut Virtual, y: f32, ms: u64) {
    v.reveal.fling = None;
    v.reveal.fling_end = None;
    v.reveal.animator = Animator::new(v.offset, REVEAL);
    v.touch.last_y = y;
    v.touch.last_ms = ms;
    v.velocity = 0.0;
    v.touch.travel = 0.0;
}

/// A move, or the release (`up`); whether the release threw the list into a fling.
fn moved(v: &mut Virtual, ui: &Testbed, up: bool, y: f32, ms: u64) -> bool {
    let unit = ui.get_unit();
    let dy = y - v.touch.last_y;
    v.touch.travel += dy.abs();
    // A tap (it did not move past the slop) focuses the row under it. The pinned header covers
    // the row under it: a tap there focuses nothing.
    let on_band = (ui.get_vband_y()..ui.get_vband_y() + HEAD).contains(&y);
    if up && !on_band && v.touch.travel < SLOP_PX * unit {
        let i = v.extents.index_at(v.offset + y);
        v.activation.moved(i);
        v.focused = Some(i);
    }
    let dt = ms.saturating_sub(v.touch.last_ms).max(1) as f32 / 1000.0;
    // Released: the velocity of the last moves, if they were recent (a finger that rested before
    // lifting throws nothing, input events §7).
    let thrown = if up && dt <= 0.1 {
        if dy.abs() > 0.0 { -dy / dt } else { v.velocity }
    } else {
        0.0
    };
    v.velocity = if up { 0.0 } else { -dy / dt };
    v.touch.last_y = y;
    v.touch.last_ms = ms;
    let max = (v.extents.total() - v.viewport).max(0.0);
    v.offset = (v.offset - dy).clamp(0.0, max);
    let fast = thrown.abs() >= FLING_PX_S * unit;
    if !(up && fast && v.touch.travel >= SLOP_PX * unit) {
        return false;
    }
    // A fling: the list goes on by itself, slowing as opendap-motion's decay says, and stops
    // where it projects (or at an end).
    let now = v.reveal.clock.elapsed().as_secs_f32();
    v.reveal.fling = Some((v.offset, thrown, now));
    v.reveal.fling_end = Some(Decay::NORMAL.end(v.offset, thrown).clamp(0.0, max));
    v.velocity = thrown;
    true
}

/// Frames while the fling moves.
fn fling(st: &Shared, ui: &Testbed, timer: &Rc<Timer>) {
    let (st, weak, t) = (st.clone(), ui.as_weak(), Rc::downgrade(timer));
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(16),
        move || {
            let (Some(ui), Some(timer)) = (weak.upgrade(), t.upgrade()) else {
                return;
            };
            {
                let mut v = st.borrow_mut();
                let Some((x0, v0, t0)) = v.reveal.fling else {
                    timer.stop();
                    return;
                };
                let t = v.reveal.clock.elapsed().as_secs_f32() - t0;
                let (x, vel) = Decay::NORMAL.at(x0, v0, t);
                let max = (v.extents.total() - v.viewport).max(0.0);
                v.offset = x.clamp(0.0, max);
                v.velocity = vel;
                if vel.abs() < 5.0 || x <= 0.0 || x >= max {
                    v.reveal.fling = None;
                    v.velocity = 0.0;
                    timer.stop();
                }
            }
            vrender(&st, &ui);
        },
    );
}
