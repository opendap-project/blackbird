//! The Arena screen: tap, double tap (when opted in) and long press compete for each contact,
//! and nested scrolling with its own primitive.

use std::{cell::RefCell, rc::Rc, time::Duration, time::Instant};

use opendap_input::{arena, nested};
use slint::{ComponentHandle, Timer};

use crate::{
    Testbed,
    keys::{Arrows, Router},
};

type Shared = Rc<RefCell<Option<arena::Arena>>>;

/// The two scrollers (inner, outer), whether the contact started in the inner one, its last y.
type Chain = Rc<RefCell<([nested::Scroller; 2], bool, f32)>>;

pub(super) fn wire(ui: &Testbed, keys: &mut Router<Testbed>) {
    // A timer ticks the arena at its deadlines, as live time would.
    let arena: Shared = Rc::default();
    let arena_timer = Rc::new(Timer::default());
    let started_arena = Instant::now();
    let weak = ui.as_weak();
    ui.on_arena_pad(move |kind, x, y| {
        let Some(ui) = weak.upgrade() else { return };
        let ms = started_arena.elapsed().as_millis() as u64;
        let input = match kind.as_str() {
            "down" => arena::Input::Down { at: (x, y), ms },
            "move" => arena::Input::Move { at: (x, y), ms },
            "up" => arena::Input::Up { at: (x, y), ms },
            _ => arena::Input::Cancel,
        };
        {
            let mut a = arena.borrow_mut();
            // A new arena when the members change (the switch) or the last contact settled.
            if kind.as_str() == "down"
                && (a.is_none() || a.as_ref().is_some_and(|a| a.deadline().is_none()))
            {
                *a = Some(fresh(ui.get_arena_double(), ui.get_unit()));
                ui.set_arena_text("…".into());
            }
            if let Some(w) = a.as_mut().and_then(|a| a.feed(input).won.first().cloned()) {
                ui.set_arena_text(format!("{w} won").into());
            }
        }
        arm(&arena_timer, &arena, ui.as_weak(), started_arena);
    });

    let chain = wire_nested(ui);
    keys.add(
        &["arena", "nested"],
        Arrows::Values,
        move |ui, name, key| {
            match (name, key) {
                ("arena", "activate") => {
                    ui.set_arena_text("Action: activate (a semantic action, no arena)".into());
                    true
                }
                ("nested", "inc" | "dec") => {
                    // Volume − (dec) scrolls on, as the finger moving up does: the inner list first.
                    let step = if key == "inc" { 1.0 } else { -1.0 };
                    let mut c = chain.borrow_mut();
                    let left = nested::scroll(&mut c.0, -24.0 * step);
                    show_nested(ui, &c.0, left);
                    true
                }
                _ => false,
            }
        },
    );
}

fn fresh(double: bool, unit: f32) -> arena::Arena {
    let a = arena::Arena::new();
    let a = if double {
        a.add(arena::DoubleTap::new(unit), arena::Priority::Normal)
    } else {
        a
    };
    a.add(arena::Tap::new(unit), arena::Priority::Normal)
        .add(arena::LongPress::new(unit), arena::Priority::Normal)
}

fn arm(timer: &Rc<Timer>, arena: &Shared, weak: slint::Weak<Testbed>, started: Instant) {
    let Some(deadline) = arena.borrow().as_ref().and_then(|a| a.deadline()) else {
        timer.stop();
        return;
    };
    let now = started.elapsed().as_millis() as u64;
    let (arena, t) = (arena.clone(), timer.clone());
    timer.start(
        slint::TimerMode::SingleShot,
        Duration::from_millis(deadline.saturating_sub(now)),
        move || {
            let Some(ui) = weak.upgrade() else { return };
            let ms = started.elapsed().as_millis() as u64;
            let won = arena
                .borrow_mut()
                .as_mut()
                .map(|a| a.feed(arena::Input::Tick { ms }).won);
            if let Some(w) = won.and_then(|w| w.first().cloned()) {
                ui.set_arena_text(format!("{w} won").into());
            }
            arm(&t, &arena, ui.as_weak(), started);
        },
    );
}

/// Nested scrolling: the finger's move goes to the inner list first when it started there, and
/// what the inner one cannot take goes to the outer one.
fn wire_nested(ui: &Testbed) -> Chain {
    let chain: Chain = Rc::new(RefCell::new((
        [
            nested::Scroller::new(8.0 * 24.0, 72.0),
            nested::Scroller::new(400.0, 150.0),
        ],
        false,
        0.0f32,
    )));
    let chain_k = chain.clone();
    let weak = ui.as_weak();
    ui.on_nested_pan(move |kind, _x, y, in_inner| {
        let Some(ui) = weak.upgrade() else { return };
        let mut st = chain.borrow_mut();
        if kind.as_str() == "down" {
            st.1 = in_inner;
            st.2 = y;
            return;
        }
        let delta = st.2 - y;
        st.2 = y;
        let start = if st.1 { 0 } else { 1 };
        let left = nested::scroll(&mut st.0[start..], delta);
        show_nested(&ui, &st.0, left);
    });
    chain_k
}

fn show_nested(ui: &Testbed, s: &[nested::Scroller; 2], left: f32) {
    ui.set_nested_inner(s[0].offset);
    ui.set_nested_outer(s[1].offset);
    ui.set_nested_text(
        format!(
            "inner {:.0}/{:.0}  outer {:.0}/{:.0}  left {:.0}",
            s[0].offset, s[0].max, s[1].offset, s[1].max, left
        )
        .into(),
    );
}
