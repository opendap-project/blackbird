//! Layout's geometry over generated lists (ADR 0014, level L1): item sizes measured in any order,
//! windows at any offset and speed, slots recycled as the window moves anywhere, scroll anchoring
//! across any change above, the focus in a virtual list, sticky headers and grids. A failure is
//! shrunk to the smallest case that breaks one.

use opendap_layout::anchor;
use opendap_layout::extents::{Extents, Fixed, Measured};
use opendap_layout::focus::{Activation, Ends, Step, follow, keep_in_view, step};
use opendap_layout::grid::{auto_fit, row_items, rows};
use opendap_layout::recycle::Pool;
use opendap_layout::sticky::{current, header_y};
use opendap_layout::window::{Overscan, visible, window};
use proptest::prelude::*;

const EPS: f32 = 0.05;

/// Measured sizes against a plain sum, built from an estimate and measurements in any order.
fn measured() -> impl Strategy<Value = (Measured, Vec<f32>)> {
    (0usize..60, 10.0f32..120.0).prop_flat_map(|(n, est)| {
        (
            Just(n),
            Just(est),
            prop::collection::vec((0usize..60, 1.0f32..300.0), 0..80),
        )
            .prop_map(|(n, est, ms)| {
                let mut m = Measured::new(n, est);
                let mut sizes = vec![est; n];
                for (i, s) in ms {
                    if n > 0 {
                        m.measure(i % n, s);
                        sizes[i % n] = s;
                    }
                }
                (m, sizes)
            })
    })
}

fn naive_index(sizes: &[f32], offset: f32) -> usize {
    let mut acc = 0.0;
    let mut idx = 0;
    for (i, s) in sizes.iter().enumerate() {
        if acc <= offset {
            idx = i;
        }
        acc += s;
    }
    idx
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 3000, ..ProptestConfig::default() })]

    /// Offsets are the sums of the sizes before, the total is the sum of all, offsets never go
    /// back, and the item at an offset is the last one starting at or before it.
    #[test]
    fn extents_agree_with_a_plain_sum((m, sizes) in measured(), probe in -100.0f32..20000.0) {
        let mut acc = 0.0;
        for (i, s) in sizes.iter().enumerate() {
            prop_assert!((m.offset(i) - acc).abs() < EPS, "offset {i}: {} vs {acc}", m.offset(i));
            prop_assert_eq!(m.size(i), *s);
            acc += s;
            prop_assert!(m.offset(i + 1) >= m.offset(i));
        }
        prop_assert!((m.total() - acc).abs() < EPS);
        if !sizes.is_empty() {
            let i = m.index_at(probe);
            prop_assert!(i < sizes.len());
            // Away from boundaries (where float sums may round either way), the naive answer.
            let near = (0..=sizes.len()).any(|k| (m.offset(k) - probe).abs() < EPS);
            if !near {
                prop_assert_eq!(i, naive_index(&sizes, probe));
            }
        }
    }

    /// The same for fixed sizes.
    #[test]
    fn fixed_extents_agree(count in 0usize..1000, size in 1.0f32..200.0, probe in -100.0f32..300000.0) {
        let f = Fixed { count, size };
        prop_assert!((f.total() - count as f32 * size).abs() < 1.0);
        if count > 0 {
            let i = f.index_at(probe);
            prop_assert!(i < count);
            prop_assert!(f.offset(i) <= probe.max(0.0) + 1e-2 || i == 0);
            prop_assert!(i == count - 1 || f.offset(i + 1) > probe - 1e-2);
        }
    }

    /// The window: inside the list, holding every item on screen, and what is visible inside it;
    /// moving fast builds at least as much ahead as standing still.
    #[test]
    fn the_window_holds_everything_on_screen((m, sizes) in measured(), at in 0.0f32..1.0, viewport in 50.0f32..900.0, v in -6000.0f32..6000.0) {
        let n = sizes.len();
        let offset = at * (m.total() - viewport).max(0.0);
        let w = window(&m, offset, viewport, v, Overscan::default());
        let vis = visible(&m, offset, viewport);
        prop_assert!(w.end <= n && vis.end <= n);
        if n == 0 { prop_assert!(w.is_empty()); return Ok(()); }
        prop_assert!(w.start <= vis.start && vis.end <= w.end, "visible {vis:?} in window {w:?}");
        for i in 0..n {
            let (top, bottom) = (m.offset(i), m.offset(i) + m.size(i));
            if bottom > offset + EPS && top < offset + viewport - EPS {
                prop_assert!(vis.contains(&i), "item {i} ({top}..{bottom}) on screen at {offset}+{viewport}, visible {vis:?}");
            }
        }
        let still = window(&m, offset, viewport, 0.0, Overscan::default());
        if v > 0.0 { prop_assert!(w.end >= still.end && w.start == still.start); }
        if v < 0.0 { prop_assert!(w.start <= still.start && w.end == still.end); }
    }

    /// Recycling as the window moves anywhere: every item in the window has its own slot, an item
    /// that stays keeps its slot, slots are created only when none is free, and their number never
    /// passes the largest window seen.
    #[test]
    fn recycled_slots_follow_the_window(windows in prop::collection::vec((0u32..500, 0u32..30), 1..30)) {
        let mut pool = Pool::new();
        let mut largest = 0;
        let mut prev: Vec<u32> = Vec::new();
        let mut prev_slots = std::collections::HashMap::new();
        for (start, len) in windows {
            let keys: Vec<u32> = (start..start + len).collect();
            largest = largest.max(keys.len());
            let free_before = pool.slots() - prev.len();
            let c = pool.update(&keys);
            prop_assert_eq!(c.kept + c.bound.len(), keys.len());
            let mut used = std::collections::HashSet::new();
            for k in &keys {
                let s = pool.slot_of(k);
                prop_assert!(s.is_some(), "{k} shown");
                prop_assert!(used.insert(s.unwrap()), "one slot, one item");
                if let Some(old) = prev_slots.get(k) {
                    prop_assert_eq!(s, Some(*old), "{} stayed: same slot", k);
                }
            }
            for k in &prev {
                if !keys.contains(k) { prop_assert!(pool.slot_of(k).is_none(), "{k} left: its slot freed"); }
            }
            let leaving = prev.iter().filter(|k| !keys.contains(k)).count();
            let needed = c.bound.len().saturating_sub(free_before + leaving);
            prop_assert_eq!(c.created, needed, "created only when none was free");
            prop_assert!(pool.slots() <= largest, "bounded by the largest window");
            prev_slots = keys.iter().map(|k| (*k, pool.slot_of(k).unwrap())).collect();
            prev = keys;
        }
    }

    /// Scroll anchoring: items above change size, appear or go, and the item looked at stays where
    /// it was on screen (unless that would scroll above the top).
    #[test]
    fn the_anchored_item_does_not_move((m, sizes) in measured(), at in 0.0f32..1.0, inserted in prop::collection::vec(1.0f32..200.0, 0..5), resized in prop::collection::vec((0usize..60, 1.0f32..300.0), 0..5)) {
        if sizes.is_empty() { return Ok(()); }
        let keys: Vec<usize> = (0..sizes.len()).map(|i| i + 1000).collect();
        let offset = at * m.total();
        let a = anchor::take(&m, &keys, offset).unwrap();
        let pos = keys.iter().position(|k| *k == a.key).unwrap();
        // New items before the start; items above the anchor resized.
        let mut new_keys: Vec<usize> = (0..inserted.len()).collect();
        new_keys.extend(&keys);
        let mut new_sizes = inserted.clone();
        new_sizes.extend(&sizes);
        for (i, s) in resized {
            if i < pos { new_sizes[inserted.len() + i] = s; }
        }
        let mut n2 = Measured::new(new_sizes.len(), 50.0);
        for (i, s) in new_sizes.iter().enumerate() { n2.measure(i, *s); }
        let o2 = anchor::restore(&n2, &new_keys, &a).unwrap();
        let j = new_keys.iter().position(|k| *k == a.key).unwrap();
        let inset = n2.offset(j) - o2;
        if n2.offset(j) - a.inset >= 0.0 {
            prop_assert!((inset - a.inset).abs() < EPS, "on screen at {} then {inset}", a.inset);
        } else {
            prop_assert_eq!(o2, 0.0);
        }
        prop_assert!(anchor::restore(&n2, &[1usize, 2], &a).is_none(), "gone: no offset");
    }

    /// The focus in a virtual list: a step lands on an item of the list or leaves it, one item at a
    /// time; the item it lands on is shown whole after keeping it in view (when it fits), with no
    /// move when it was already shown; kept by key, the focus follows its item.
    #[test]
    fn the_focus_walks_the_whole_list((m, sizes) in measured(), from in prop::option::of(0usize..60), delta in prop_oneof![Just(1i32), Just(-1i32)], at in 0.0f32..1.0, viewport in 100.0f32..800.0, cycle in any::<bool>()) {
        let n = sizes.len();
        let from = from.map(|f| f % n.max(1));
        let ends = if cycle { Ends::Cycle } else { Ends::Stop };
        let at_end = from.is_some_and(|f| (f == 0 && delta < 0) || (f + 1 == n && delta > 0));
        match step(from, delta, n, ends) {
            Step::Item(i) => {
                prop_assert!(i < n);
                if let Some(f) = from { prop_assert_eq!(i as i64, f as i64 + i64::from(delta)); }
                let offset = at * (m.total() - viewport).max(0.0);
                let o = keep_in_view(&m, i, offset, viewport);
                prop_assert!(o >= 0.0 && o <= (m.total() - viewport).max(0.0) + EPS);
                let (top, bottom) = (m.offset(i), m.offset(i) + m.size(i));
                if top >= offset && bottom <= offset + viewport { prop_assert_eq!(o, offset, "already shown"); }
                if m.size(i) <= viewport { prop_assert!(top >= o - EPS && bottom <= o + viewport + EPS, "shown whole"); }
            }
            // Only a cyclic list goes round, only from an end, to the other end.
            Step::Wrapped(i) => {
                prop_assert!(cycle && at_end, "wrapped: cycle {} at an end {}", cycle, at_end);
                prop_assert_eq!(i, if delta > 0 { 0 } else { n - 1 });
            }
            Step::Jumped(_) => prop_assert!(false, "a step never jumps"),
            // A list that stops is left at its ends; a cyclic one only when empty.
            Step::Leave => prop_assert!(n == 0 || (!cycle && at_end)),
        }
    }

    #[test]
    fn the_focus_follows_its_item(keys in prop::collection::vec(0u16..40, 0..20), focused in 0u16..40, old in 0usize..30) {
        let mut seen = std::collections::HashSet::new();
        let keys: Vec<u16> = keys.into_iter().filter(|k| seen.insert(*k)).collect();
        match follow(&keys, &focused, old) {
            None => prop_assert!(keys.is_empty()),
            Some(i) => {
                prop_assert!(i < keys.len());
                if keys.contains(&focused) { prop_assert_eq!(keys[i], focused); } else { prop_assert_eq!(i, old.min(keys.len() - 1)); }
            }
        }
    }

    /// Action on a placeholder: at most one wait at a time; it is taken only when its data is
    /// there, only once, and given up when the focus moves elsewhere.
    #[test]
    fn a_waiting_action_is_taken_once_or_given_up(ops in prop::collection::vec((0u8..3, 0usize..4, any::<bool>()), 1..30)) {
        let mut a = Activation::default();
        let mut model: Option<usize> = None;
        for (kind, i, ready) in ops {
            match kind {
                0 => {
                    let r = a.press(i, ready);
                    prop_assert_eq!(r, ready.then_some(i));
                    model = (!ready).then_some(i);
                }
                1 => { a.moved(i); if model != Some(i) { model = None; } }
                _ => {
                    let r = a.arrived(|x| ready && x == i);
                    let expect = model.filter(|m| ready && *m == i);
                    prop_assert_eq!(r, expect);
                    if expect.is_some() { model = None; }
                }
            }
            prop_assert_eq!(a.waiting(), model);
        }
    }

    /// A sticky header never leaves its section: drawn somewhere from the section's start to its
    /// end minus its height, at its natural place until it reaches the top, pinned there while its
    /// section passes.
    #[test]
    fn a_sticky_header_stays_in_its_section(start in 0.0f32..5000.0, len in 0.0f32..2000.0, height in 1.0f32..100.0, offset in 0.0f32..8000.0) {
        let end = start + len.max(height);
        let y = header_y(start, end, height, offset);
        let at = y + offset;
        prop_assert!(at >= start - EPS && at <= end - height + EPS, "{at} in {start}..{}", end - height);
        if start >= offset { prop_assert!((y - (start - offset)).abs() < EPS, "below the top: at its natural place"); }
        if offset >= start && offset <= end - height { prop_assert!(y.abs() < EPS, "pinned at the top"); }
    }

    #[test]
    fn the_current_section_is_the_one_at_the_top(mut starts in prop::collection::vec(0.0f32..5000.0, 0..10), offset in -100.0f32..6000.0) {
        starts.sort_by(f32::total_cmp);
        match current(&starts, offset) {
            None => prop_assert!(starts.first().is_none_or(|s| *s > offset)),
            Some(i) => prop_assert!(starts[i] <= offset && starts.get(i + 1).is_none_or(|s| *s > offset)),
        }
    }

    /// A grid: the columns fill the width exactly, each at least the minimum (unless one column is
    /// all that fits), one more would not fit; rows hold every item once, in order.
    #[test]
    fn a_grid_tiles_its_items(width in 50.0f32..2000.0, min in 20.0f32..300.0, gap in 0.0f32..24.0, count in 0usize..500) {
        let (cols, cell) = auto_fit(width, min, gap);
        prop_assert!(cols >= 1);
        prop_assert!((cols as f32 * cell + (cols - 1) as f32 * gap - width).abs() < EPS);
        if cols > 1 { prop_assert!(cell >= min - EPS); }
        prop_assert!((cols + 1) as f32 * min + cols as f32 * gap > width - EPS, "one more column would not fit");
        let r = rows(count, cols, 100.0, gap);
        let mut next = 0;
        for row in 0..r.count {
            let items = row_items(row, cols, count);
            prop_assert_eq!(items.start, next);
            prop_assert!(!items.is_empty() && items.len() <= cols);
            next = items.end;
        }
        prop_assert_eq!(next, count);
    }
}
