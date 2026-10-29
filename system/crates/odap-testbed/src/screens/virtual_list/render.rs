//! Lays out the window and asks for what it lacks; page arrivals call it again.

use std::{collections::HashSet, ops::Range, time::Duration};

use blackbird_ui_layout::extents::Extents;
use blackbird_ui_layout::{
    anchor,
    data::{gate::Cost, pages::ItemState},
    window::{Overscan, visible, window},
};
use blackbird_ui_motion::animator::Animator;
use slint::{ComponentHandle, Model, ModelRc, Timer};

use super::{HEAD, REVEAL, SECTION, Shared, VCOUNT, Virtual};
use crate::{Testbed, VRow};

/// A slot no row uses: drawn nowhere.
const HIDDEN: VRow = VRow {
    index: 0,
    y: -1000.0,
    h: 0.0,
    tall: false,
    ready: false,
    cover: false,
    focused: false,
    head: 0.0,
};

pub(super) fn vrender(st: &Shared, ui: &Testbed) {
    let mut v = st.borrow_mut();
    let range = layout(&mut v);
    let keys: Vec<usize> = range.clone().collect();
    let changes = v.pool.update(&keys);
    v.data.rebound = changes.bound.len();
    request(st, ui, &mut v, range.clone());
    let shown = visible(&v.extents, v.offset, v.viewport);
    decode(&mut v, shown.clone());
    rows(&mut v, &keys);
    ui.set_vrows(ModelRc::from(v.model.clone()));
    band(&v, ui);
    ui.set_vtext(status(&v, shown, range.len()).into());
}

/// The window to build. Rows are measured as they are built, so sizes above what is on screen
/// change (a jump far into rows only estimated so far): the focused row, or the first visible,
/// keeps its place (blackbird-ui-layout's anchor), and the window is laid out again where that leaves
/// it.
fn layout(v: &mut Virtual) -> Range<usize> {
    let mut range = window(
        &v.extents,
        v.offset,
        v.viewport,
        v.velocity,
        Overscan::default(),
    );
    for _ in 0..3 {
        let a = anchor::take_index(&v.extents, v.offset, v.viewport, v.focused);
        for i in range.clone() {
            let head = if i % SECTION == 0 { HEAD } else { 0.0 };
            v.extents
                .measure(i, head + if i % 7 == 0 { 72.0 } else { 48.0 });
        }
        let Some(a) = a else { break };
        let to = anchor::restore_index(&v.extents, &a, v.viewport);
        let moved = to - v.offset;
        if moved.abs() < 0.5 {
            break;
        }
        v.offset = to;
        // A reveal on its way keeps going to the same row, from where that row now is.
        let now = v.reveal.clock.elapsed().as_secs_f32();
        if v.reveal.animator.running(now) {
            let target = v.reveal.animator.target() + moved;
            v.reveal.animator = Animator::new(to, REVEAL);
            v.reveal.animator.retarget(target, now, None);
        }
        range = window(
            &v.extents,
            v.offset,
            v.viewport,
            v.velocity,
            Overscan::default(),
        );
    }
    range
}

/// The data: the pages this window needs, most needed first; and what Action waited for, once
/// its data is here.
fn request(st: &Shared, ui: &Testbed, v: &mut Virtual, range: Range<usize>) {
    let forward = v.velocity >= 0.0;
    let plan = v.data.pages.plan(range, forward);
    v.data.cancelled += plan.cancel.len();
    v.data.evicted += plan.evict.len();
    for page in plan.load {
        let (st, weak) = (st.clone(), ui.as_weak());
        Timer::single_shot(Duration::from_millis(250), move || {
            st.borrow_mut().data.pages.arrived(page, 50, false);
            if let Some(ui) = weak.upgrade() {
                vrender(&st, &ui);
            }
        });
    }
    let ready = v
        .activation
        .waiting()
        .is_some_and(|i| v.data.pages.item(i) == ItemState::Ready);
    if let Some(i) = v.activation.arrived(|_| ready) {
        v.activated = Some(i);
    }
}

/// Covers: decoded only while the scroll is slow, kept within the cache's budget.
fn decode(v: &mut Virtual, shown: Range<usize>) {
    if v.data.gate.admits(Cost::Costly, v.velocity) {
        for i in shown {
            if v.data.pages.item(i) == ItemState::Ready && v.data.covers.get(&i).is_none() {
                v.data.covers.put(i, (), 1);
            }
        }
    }
}

/// The slots' rows.
fn rows(v: &mut Virtual, keys: &[usize]) {
    while v.model.row_count() < v.pool.slots() {
        v.model.push(HIDDEN);
    }
    // A slot no row of the window uses keeps whatever it last drew, where it drew it: hidden, or
    // after a jump it would show a row from far away among the ones on screen.
    let used: HashSet<usize> = keys.iter().filter_map(|k| v.pool.slot_of(k)).collect();
    for slot in 0..v.model.row_count() {
        if !used.contains(&slot) && v.model.row_data(slot).is_some_and(|r| r.y > -1000.0) {
            v.model.set_row_data(slot, HIDDEN);
        }
    }
    for &k in keys {
        if let Some(slot) = v.pool.slot_of(&k) {
            let row = VRow {
                index: k as i32,
                y: v.extents.offset(k) - v.offset,
                h: v.extents.size(k),
                tall: k % 7 == 0,
                ready: v.data.pages.item(k) == ItemState::Ready,
                cover: v.data.covers.get(&k).is_some(),
                focused: v.focused == Some(k),
                head: if k % SECTION == 0 { HEAD } else { 0.0 },
            };
            v.model.set_row_data(slot, row);
        }
    }
}

/// The pinned header: the section at the list's top.
fn band(v: &Virtual, ui: &Testbed) {
    let k = v.extents.index_at(v.offset) / SECTION;
    let start = v.extents.offset(k * SECTION);
    let end = if (k + 1) * SECTION < VCOUNT {
        v.extents.offset((k + 1) * SECTION)
    } else {
        v.extents.total()
    };
    ui.set_vband_y(blackbird_ui_layout::sticky::header_y(
        start, end, HEAD, v.offset,
    ));
    ui.set_vband_text(
        format!(
            "Rows {}–{}",
            k * SECTION,
            ((k + 1) * SECTION).min(VCOUNT) - 1
        )
        .into(),
    );
}

/// What the list says about itself.
fn status(v: &Virtual, shown: Range<usize>, built: usize) -> String {
    let d = &v.data;
    let slot = |i: usize| v.pool.slot_of(&i).map_or("-".into(), |s| s.to_string());
    format!(
        "{VCOUNT} rows, {}-{} on screen, built {}\nslots {}, rebound {}, {:.0} px/s{}\npages {} loaded, {} loading, {} cancelled, {} evicted\ncovers {} decoded{}\nfocus: {}{}",
        shown.start,
        shown.end.saturating_sub(1),
        built,
        v.pool.slots(),
        d.rebound,
        v.velocity,
        v.reveal
            .fling_end
            .map_or(String::new(), |e| format!(", fling to {e:.0}")),
        d.pages.loaded(),
        d.pages.in_flight(),
        d.cancelled,
        d.evicted,
        d.covers.len(),
        if d.gate.admits(Cost::Costly, v.velocity) {
            ""
        } else {
            ", waiting for the scroll to slow"
        },
        v.focused
            .map_or("none".to_string(), |i| format!("row {i}, slot {}", slot(i))),
        match (v.activation.waiting(), v.activated) {
            (Some(i), _) => format!(", row {i} waits for its data"),
            (None, Some(i)) => format!(", activated row {i}"),
            _ => String::new(),
        }
    )
}
