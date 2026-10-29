//! The input domain's geometry over generated input (ADR 0014, level L1): an overlay placed at
//! any anchor in any bounds, either direction; nested scroll areas under any motion; the focus
//! revealed through any chain of areas; matched elements declared and withdrawn in any order;
//! frames, angles and a contact's track. A failure is shrunk to the smallest case that breaks one.

use std::collections::HashMap;

use blackbird_ui_input::anchor::{Rect, Side, place};
use blackbird_ui_input::gesture::{Frame, FramePath};
use blackbird_ui_input::matched::{self, Matched};
use blackbird_ui_input::nested::{RevealArea, Scroller, fling, reveal, scroll};
use blackbird_ui_input::space::{Track, angle_delta, normalised, polar};
use proptest::prelude::*;

const EPS: f32 = 1e-2;

fn side() -> impl Strategy<Value = Side> {
    prop_oneof![
        Just(Side::Below),
        Just(Side::Above),
        Just(Side::Start),
        Just(Side::End)
    ]
}

/// Bounds, an anchor somewhere in them, and an overlay that fits in them.
fn scene() -> impl Strategy<Value = (Rect, Rect, (f32, f32))> {
    (
        100.0f32..600.0,
        100.0f32..900.0,
        -50.0f32..50.0,
        -50.0f32..50.0,
    )
        .prop_flat_map(|(bw, bh, bx, by)| {
            let bounds = Rect::new(bx, by, bw, bh);
            (
                Just(bounds),
                (0.0f32..1.0, 0.0f32..1.0, 1.0f32..0.6 * bw, 1.0f32..0.6 * bh),
                (1.0f32..bw, 1.0f32..bh),
            )
                .prop_map(move |(b, (fx, fy, w, h), size)| {
                    (
                        b,
                        Rect::new(
                            b.origin.0 + fx * (b.size.0 - w),
                            b.origin.1 + fy * (b.size.1 - h),
                            w,
                            h,
                        ),
                        size,
                    )
                })
        })
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.origin.0 + EPS < b.origin.0 + b.size.0
        && b.origin.0 + EPS < a.origin.0 + a.size.0
        && a.origin.1 + EPS < b.origin.1 + b.size.1
        && b.origin.1 + EPS < a.origin.1 + a.size.1
}

/// The physical side a logical one names in a direction.
fn physical(s: Side, rtl: bool) -> Side {
    match (s, rtl) {
        (Side::Start, true) => Side::End,
        (Side::End, true) => Side::Start,
        (s, _) => s,
    }
}

/// The room on a physical side of the anchor, inside the bounds.
fn room(anchor: Rect, bounds: Rect, s: Side, gap: f32) -> f32 {
    match s {
        Side::Below => bounds.origin.1 + bounds.size.1 - (anchor.origin.1 + anchor.size.1) - gap,
        Side::Above => anchor.origin.1 - bounds.origin.1 - gap,
        Side::Start => anchor.origin.0 - bounds.origin.0 - gap,
        Side::End => bounds.origin.0 + bounds.size.0 - (anchor.origin.0 + anchor.size.0) - gap,
    }
}

fn opposite(s: Side) -> Side {
    match s {
        Side::Below => Side::Above,
        Side::Above => Side::Below,
        Side::Start => Side::End,
        Side::End => Side::Start,
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 5000, ..ProptestConfig::default() })]

    /// An overlay that fits the bounds always lands inside them; when the side asked (or, failing
    /// it, the other) has room, it never covers its anchor and uses the side with room, keeping
    /// the one asked whenever that has room.
    #[test]
    fn an_overlay_stays_inside_and_off_its_anchor((bounds, anchor, size) in scene(), asked in side(), gap in 0.0f32..16.0, rtl in any::<bool>()) {
        let (used, (x, y)) = place(anchor, size, asked, gap, bounds, rtl);
        prop_assert!(x >= bounds.origin.0 - EPS && y >= bounds.origin.1 - EPS, "inside: ({x}, {y})");
        prop_assert!(x + size.0 <= bounds.origin.0 + bounds.size.0 + EPS && y + size.1 <= bounds.origin.1 + bounds.size.1 + EPS, "inside: ({x}, {y}) {size:?}");
        let main = |s: Side| match physical(s, rtl) { Side::Below | Side::Above => size.1, _ => size.0 };
        let fits = |s: Side| room(anchor, bounds, physical(s, rtl), gap) >= main(s);
        if fits(asked) {
            prop_assert_eq!(used, asked, "the side asked has room: kept");
        } else if fits(opposite(asked)) {
            prop_assert_eq!(used, opposite(asked), "flipped to the side with room");
        }
        if fits(asked) || fits(opposite(asked)) {
            prop_assert!(!overlaps(Rect::new(x, y, size.0, size.1), anchor), "off the anchor: {used:?} at ({x}, {y})");
        }
    }

    /// Right to left is a mirror: the same logical request in the mirrored scene lands at the
    /// mirrored place, on the same logical side.
    #[test]
    fn right_to_left_mirrors_the_placement((bounds, anchor, size) in scene(), asked in side(), gap in 0.0f32..16.0) {
        let mirror_x = |x: f32, w: f32| 2.0 * bounds.origin.0 + bounds.size.0 - x - w;
        let (u1, (x1, y1)) = place(anchor, size, asked, gap, bounds, false);
        let m = Rect::new(mirror_x(anchor.origin.0, anchor.size.0), anchor.origin.1, anchor.size.0, anchor.size.1);
        let (u2, (x2, y2)) = place(m, size, asked, gap, bounds, true);
        prop_assert_eq!(u1, u2);
        prop_assert!((mirror_x(x1, size.0) - x2).abs() < EPS && (y1 - y2).abs() < EPS, "({x1}, {y1}) mirrors to ({x2}, {y2})");
    }
}

fn chain() -> impl Strategy<Value = Vec<Scroller>> {
    prop::collection::vec(
        (0.0f32..2000.0, 0.0f32..1.0, prop::bool::weighted(0.25)),
        1..5,
    )
    .prop_map(|v| {
        v.into_iter()
            .map(|(max, at, first)| Scroller {
                offset: max * at,
                max,
                takes_first: first,
            })
            .collect()
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 5000, ..ProptestConfig::default() })]

    /// A finger's motion through nested areas: nothing is lost or invented (what the areas took
    /// plus what is left is the motion), every area stays in its range, what is left goes the
    /// finger's way, and something is left only when no area can move that way any more.
    #[test]
    fn nested_scroll_conserves_the_motion(mut areas in chain(), deltas in prop::collection::vec(-1500.0f32..1500.0, 1..10)) {
        for d in deltas {
            let before: f32 = areas.iter().map(|s| s.offset).sum();
            let left = scroll(&mut areas, d);
            let after: f32 = areas.iter().map(|s| s.offset).sum();
            prop_assert!(((after - before) + left - d).abs() < 0.05, "taken {} + left {left} = {d}", after - before);
            prop_assert!(areas.iter().all(|s| (0.0..=s.max).contains(&s.offset)));
            prop_assert!(left * d >= 0.0 && left.abs() <= d.abs() + 1e-3);
            if left.abs() > 1e-3 {
                prop_assert!(areas.iter().all(|s| if d > 0.0 { s.offset >= s.max } else { s.offset <= 0.0 }), "left over only at every edge");
            }
            // A fling goes to the innermost area that can still move its way.
            for v in [d, -d] {
                if v == 0.0 { continue; }
                let can = |s: &Scroller| if v > 0.0 { s.offset < s.max } else { s.offset > 0.0 };
                prop_assert_eq!(fling(&areas, v), areas.iter().position(can));
            }
        }
    }
}

/// A chain of areas, innermost first, each placed inside its parent's content, and a span in the
/// innermost content.
fn reveal_scene() -> impl Strategy<Value = (Vec<RevealArea>, f32, f32)> {
    prop::collection::vec(
        (
            100.0f32..900.0,
            0.0f32..3000.0,
            0.0f32..1.0,
            0.0f32..60.0,
            0.0f32..60.0,
            0.0f32..1.0,
        ),
        1..4,
    )
    .prop_flat_map(|raw| {
        let mut areas: Vec<RevealArea> = Vec::new();
        // Outermost last; each area's viewport sits somewhere in its parent's content.
        for (i, (viewport, extra, at_frac, s, e, off)) in raw.iter().enumerate() {
            let content = viewport + extra;
            let max = (content - viewport).max(0.0);
            let parent_content = raw.get(i + 1).map(|p| p.0 + p.1).unwrap_or(f32::INFINITY);
            let at = if parent_content.is_finite() {
                at_frac * (parent_content - viewport).max(0.0)
            } else {
                0.0
            };
            let scroller = Scroller {
                offset: max * off,
                max,
                takes_first: false,
            };
            areas.push(RevealArea {
                scroller,
                viewport: *viewport,
                at,
                inset_start: *s,
                inset_end: *e,
            });
        }
        let content = areas[0].viewport + areas[0].scroller.max;
        (Just(areas), 0.0f32..1.0, 1.0f32..400.0).prop_map(move |(a, f, len)| {
            let start = f * (content - 1.0);
            (a, start, (start + len).min(content))
        })
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 5000, ..ProptestConfig::default() })]

    /// The focus revealed through nested areas, innermost first: every offset stays in range; an
    /// area whose room already shows the span does not move; otherwise, when the span fits the
    /// room and the area can scroll that far, it ends whole in the room (below what covers the
    /// edges); taller than the room, its start shows at the room's start.
    #[test]
    fn reveal_shows_the_focus_in_every_area((mut areas, start, end) in reveal_scene()) {
        let before: Vec<RevealArea> = areas.clone();
        let offsets = reveal(&mut areas, start, end);
        prop_assert_eq!(offsets.len(), areas.len());
        let (mut s, mut e) = (start, end);
        for (a, b) in areas.iter().zip(&before) {
            let o = a.scroller.offset;
            prop_assert!((0.0..=a.scroller.max).contains(&o), "in range: {o}");
            let room = |o: f32| (o + a.inset_start, o + a.viewport - a.inset_end);
            let (r0, r1) = room(b.scroller.offset);
            if s >= r0 && e <= r1 {
                prop_assert_eq!(o, b.scroller.offset, "already shown: no move");
            } else if e - s <= r1 - r0 {
                let (lo, hi) = (e - a.viewport + a.inset_end, s - a.inset_start);
                // Some offset in range shows it: then the one taken does.
                if hi >= 0.0 && lo <= a.scroller.max && lo <= hi {
                    let (n0, n1) = room(o);
                    prop_assert!(s >= n0 - EPS && e <= n1 + EPS, "{s}..{e} in the room {n0}..{n1}");
                }
            } else if (0.0..=a.scroller.max).contains(&(s - a.inset_start)) {
                prop_assert!((o + a.inset_start - s).abs() < EPS, "taller: its start at the room's start");
            }
            // The part this area shows, in its parent's content.
            let (vs, ve) = ((s - o).max(0.0), (e - o).min(a.viewport));
            s = a.at + vs;
            e = a.at + ve;
        }
    }
}

fn rect() -> impl Strategy<Value = Rect> {
    (0.0f32..300.0, 0.0f32..300.0, 1.0f32..100.0, 1.0f32..100.0)
        .prop_map(|(x, y, w, h)| Rect::new(x, y, w, h))
}

#[derive(Debug, Clone)]
enum MOp {
    Declare(u8, u8, bool, Rect),
    Withdraw(u8, u8, bool),
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 3000, ..ProptestConfig::default() })]

    /// Matched elements against a plain model: whatever is declared, moved and withdrawn, in any
    /// order, a namespace pairs exactly the ids with both sides there now, from the frames they
    /// declared last, and never an id of another namespace.
    #[test]
    fn matched_pairs_exactly_what_is_declared_twice(ops in prop::collection::vec(prop_oneof![
        3 => (0u8..2, 0u8..4, any::<bool>(), rect()).prop_map(|(n, i, s, r)| MOp::Declare(n, i, s, r)),
        1 => (0u8..2, 0u8..4, any::<bool>()).prop_map(|(n, i, s)| MOp::Withdraw(n, i, s)),
    ], 1..30)) {
        let side = |s: bool| if s { matched::Side::Source } else { matched::Side::Destination };
        let mut m = Matched::default();
        let mut model: HashMap<(u8, u8, bool), Rect> = HashMap::new();
        for op in ops {
            match op {
                MOp::Declare(n, i, s, r) => { m.declare(&format!("ns{n}"), &format!("id{i}"), side(s), r); model.insert((n, i, s), r); }
                MOp::Withdraw(n, i, s) => { m.withdraw(&format!("ns{n}"), &format!("id{i}"), side(s)); model.remove(&(n, i, s)); }
            }
            for n in 0..2u8 {
                let mut got = m.pairs(&format!("ns{n}"));
                got.sort_by(|a, b| a.0.cmp(&b.0));
                let mut want: Vec<(String, Rect, Rect)> = (0..4u8)
                    .filter_map(|i| Some((format!("id{i}"), *model.get(&(n, i, true))?, *model.get(&(n, i, false))?)))
                    .collect();
                want.sort_by(|a, b| a.0.cmp(&b.0));
                prop_assert_eq!(got, want);
            }
        }
    }
}

fn frame() -> impl Strategy<Value = Frame> {
    (
        -200.0f32..200.0,
        -200.0f32..200.0,
        10.0f32..500.0,
        10.0f32..500.0,
        any::<bool>(),
    )
        .prop_map(|(x, y, w, h, rtl)| Frame {
            rtl,
            ..Frame::new((w, h), 1.0).at((x, y))
        })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 5000, ..ProptestConfig::default() })]

    /// A point taken into a nested frame and back is where it was; through two paths from the same
    /// parent, converting is the same as going out and in.
    #[test]
    fn frames_convert_both_ways(path in prop::collection::vec(frame(), 0..5), other in prop::collection::vec(frame(), 0..5), p in (-1000.0f32..1000.0, -1000.0f32..1000.0)) {
        let a = FramePath(path);
        let b = FramePath(other);
        let back = a.to_outer(a.to_inner(p));
        prop_assert!((back.0 - p.0).abs() < EPS && (back.1 - p.1).abs() < EPS);
        let c = a.convert(p, &b);
        let d = b.to_inner(a.to_outer(p));
        prop_assert!((c.0 - d.0).abs() < EPS && (c.1 - d.1).abs() < EPS);
    }

    /// A slider's value: always between 0 and 1, and right to left reads it from the other end.
    #[test]
    fn normalised_stays_in_range_and_mirrors(f in frame(), local in (-600.0f32..600.0, -600.0f32..600.0)) {
        let ltr = Frame { rtl: false, ..f };
        let rtl = Frame { rtl: true, ..f };
        let (x, y) = normalised(&ltr, local);
        prop_assert!((0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y));
        let (xr, yr) = normalised(&rtl, local);
        prop_assert!((xr - (1.0 - x)).abs() < 1e-5 && yr == y);
    }

    /// A dial: the angle is in 0–360 and, with the radius, gives the point back; the turn between
    /// two angles is in −180–180 and lands on the second.
    #[test]
    fn polar_and_turns_are_consistent(c in (-100.0f32..100.0, -100.0f32..100.0), p in (-300.0f32..300.0, -300.0f32..300.0), from in 0.0f32..360.0, to in 0.0f32..360.0) {
        let (a, r) = polar(c, p);
        prop_assert!((0.0..360.0).contains(&a) && r >= 0.0);
        let rad = a.to_radians();
        let q = (c.0 + r * rad.sin(), c.1 - r * rad.cos());
        prop_assert!((q.0 - p.0).abs() < 0.05 && (q.1 - p.1).abs() < 0.05, "{p:?} → ({a}, {r}) → {q:?}");
        let d = angle_delta(from, to);
        prop_assert!((-180.0..180.0 + 1e-3).contains(&d));
        prop_assert!(((from + d - to).rem_euclid(360.0) + 1e-3) % 360.0 < 2e-3, "{from} + {d} lands on {to}");
    }

    /// A contact's track: the whole move is the sum of its steps (derived from positions, never
    /// summed, so it cannot drift), and a steady move reads its exact speed.
    #[test]
    fn a_track_does_not_drift(steps in prop::collection::vec((-30.0f32..30.0, -30.0f32..30.0), 2..40), path in prop::collection::vec(frame(), 0..3), speed in (-2000.0f32..2000.0, -2000.0f32..2000.0)) {
        let path = FramePath(path);
        let mut t = Track::default();
        let mut p = (100.0f32, 100.0f32);
        t.push(p, 0);
        let mut sum = (0.0f32, 0.0f32);
        for (i, (dx, dy)) in steps.iter().enumerate() {
            p = (p.0 + dx, p.1 + dy);
            t.push(p, (i as u64 + 1) * 8);
            let d = t.delta(&path, &path).unwrap();
            sum = (sum.0 + d.0, sum.1 + d.1);
        }
        let total = t.translation(&path, &path).unwrap();
        prop_assert!((total.0 - sum.0).abs() < 0.05 && (total.1 - sum.1).abs() < 0.05);
        let mut s = Track::default();
        for i in 0..20u64 {
            s.push((speed.0 * i as f32 * 0.008, speed.1 * i as f32 * 0.008), i * 8);
        }
        let v = s.velocity(&path);
        prop_assert!((v.0 - speed.0).abs() < 1.0 && (v.1 - speed.1).abs() < 1.0, "{v:?} for {speed:?}");
    }
}
