//! A contact's invariants over generated input (ADR 0014, level L1): any frame (size, density,
//! direction), any directives (axis, drag start), and any sequence of moves, long-press checks and
//! pauses, ended by a lift or by the system taking the contact. Whatever the combination, what the
//! contact reports is a well-formed stream (W3C's pointer events: one start, moves, one end), and a
//! failure is shrunk to the smallest contact that breaks it.

use blackbird_ui_input::gesture::{
    Axis, Contact, DragOptions, DragStart, Edge, FLING_PX_S, Frame, FramePath, Gesture, Phase,
    Region,
};
use proptest::prelude::*;

#[derive(Debug, Clone)]
enum Step {
    /// Move by (dx, dy) after `ms`.
    Move(f32, f32, u64),
    /// The long-press timer fires after `ms`.
    Long(u64),
}

#[derive(Debug, Clone)]
enum End {
    Up(f32, f32, u64),
    /// The system takes it, then the finger goes on moving and lifts (the composer may still
    /// forward what the hardware reports).
    Cancel(Vec<(f32, f32, u64)>),
}

fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        // Small moves (inside the slop) as often as large ones (past it, fast or slow).
        4 => (-8.0f32..8.0, -8.0f32..8.0, 1u64..40).prop_map(|(x, y, t)| Step::Move(x, y, t)),
        3 => (-80.0f32..80.0, -80.0f32..80.0, 1u64..60).prop_map(|(x, y, t)| Step::Move(x, y, t)),
        1 => (0u64..700).prop_map(Step::Long),
    ]
}

fn end() -> impl Strategy<Value = End> {
    prop_oneof![
        3 => (-60.0f32..60.0, -60.0f32..60.0, 1u64..200).prop_map(|(x, y, t)| End::Up(x, y, t)),
        1 => prop::collection::vec((-40.0f32..40.0, -40.0f32..40.0, 1u64..40), 0..4).prop_map(End::Cancel),
    ]
}

fn axis() -> impl Strategy<Value = Axis> {
    prop_oneof![
        Just(Axis::Free),
        Just(Axis::Horizontal),
        Just(Axis::Vertical),
        Just(Axis::Dominant)
    ]
}

fn frame() -> impl Strategy<Value = Frame> {
    (
        200.0f32..800.0,
        200.0f32..800.0,
        prop_oneof![Just(1.0f32), Just(1.5), Just(2.0)],
        any::<bool>(),
    )
        .prop_map(|(w, h, unit, rtl)| Frame {
            rtl,
            ..Frame::new((w, h), unit)
        })
}

/// A composite gesture's part: whether it is a drag (else a pan), its phase, where it began, where it is.
type Part = (bool, Phase, (f32, f32), (f32, f32));

/// The composite part of a gesture: its kind (pan or drag), phase and position.
fn composite(g: &Gesture) -> Option<Part> {
    match *g {
        Gesture::Pan {
            phase, from, at, ..
        } => Some((false, phase, from, at)),
        Gesture::Drag {
            phase, from, at, ..
        } => Some((true, phase, from, at)),
        _ => None,
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 3000, ..ProptestConfig::default() })]

    #[test]
    fn a_contact_reports_a_well_formed_stream(
        f in frame(),
        down in (0.0f32..1.0, 0.0f32..1.0),
        ax in axis(),
        immediate in any::<bool>(),
        steps in prop::collection::vec(step(), 0..20),
        e in end(),
    ) {
        let start = if immediate { DragStart::Immediate } else { DragStart::LongPress };
        let at0 = (down.0 * f.size.0, down.1 * f.size.1);
        let mut c = Contact::with(at0, 0, f, DragOptions { axis: ax, start });
        prop_assert_eq!(c.region(), f.region(at0), "the region is where it went down");
        let mut now = 0u64;
        let mut pos = at0;
        let mut log: Vec<Gesture> = Vec::new();
        for s in &steps {
            match *s {
                Step::Move(dx, dy, t) => {
                    now += t;
                    pos = (pos.0 + dx, pos.1 + dy);
                    log.extend(c.moved(pos, now));
                }
                Step::Long(t) => {
                    now += t;
                    log.extend(c.long_due(now));
                }
            }
        }
        let mut cancelled = false;
        match &e {
            End::Up(dx, dy, t) => {
                now += t;
                pos = (pos.0 + dx, pos.1 + dy);
                log.extend(c.up(pos, now));
            }
            End::Cancel(after) => {
                cancelled = true;
                log.extend(c.cancel());
                let n = log.len();
                // Nothing that started acts, and nothing new starts: the system has the contact.
                for (dx, dy, t) in after {
                    now += t;
                    pos = (pos.0 + dx, pos.1 + dy);
                    log.extend(c.moved(pos, now));
                    log.extend(c.long_due(now));
                }
                log.extend(c.up(pos, now + 1));
                prop_assert_eq!(log.len(), n, "after the system took it, nothing follows: {:?}", &log[n..]);
            }
        }

        // The composite stream: one start first, moves, and one end or cancel last; never a pan
        // that turns into a drag; never an end with no start (a drop nothing picked up).
        let comp: Vec<_> = log.iter().filter_map(composite).collect();
        if let Some(first) = comp.first() {
            prop_assert_eq!(first.1, Phase::Start, "a composite gesture begins with its start: {:?}", log);
            prop_assert!(comp.iter().all(|x| x.0 == first.0), "one kind throughout");
            for (i, x) in comp.iter().enumerate().skip(1) {
                let last = i == comp.len() - 1;
                match x.1 {
                    Phase::Start => prop_assert!(false, "a second start"),
                    Phase::Move => {}
                    Phase::End | Phase::Cancel => prop_assert!(last, "nothing after its end"),
                }
            }
            if cancelled {
                prop_assert_eq!(comp.last().unwrap().1, Phase::Cancel);
            } else {
                prop_assert!(matches!(comp.last().unwrap().1, Phase::End), "a lift ends it: {:?}", log);
            }
            // The axis holds every position to it.
            for x in &comp {
                match ax {
                    Axis::Horizontal => prop_assert_eq!(x.3 .1, x.2 .1),
                    Axis::Vertical => prop_assert_eq!(x.3 .0, x.2 .0),
                    Axis::Dominant => prop_assert!(x.3 .0 == x.2 .0 || x.3 .1 == x.2 .1, "locked to one axis"),
                    Axis::Free => {}
                }
            }
        }

        let longs = log.iter().filter(|g| matches!(g, Gesture::LongPress { .. })).count();
        let taps = log.iter().filter(|g| matches!(g, Gesture::Tap { .. })).count();
        prop_assert!(longs <= 1, "a long press once");
        // A long press only while still within the slop: never after a composite gesture started.
        if let Some(lp) = log.iter().position(|g| matches!(g, Gesture::LongPress { .. })) {
            prop_assert!(!log[..lp].iter().any(|g| composite(g).is_some()));
        }
        // A tap is a contact that did nothing else, and only one that lifted.
        prop_assert!(taps <= 1);
        if taps == 1 {
            prop_assert!(!cancelled && longs == 0 && comp.is_empty(), "a tap and something else: {:?}", log);
        }
        if !cancelled && longs == 0 && comp.is_empty() {
            prop_assert_eq!(taps, 1, "a lift within the slop and before the long press is a tap");
        }
        // A drag starts at once only where it was asked to; otherwise only after the long press.
        if comp.first().is_some_and(|x| x.0) && !immediate {
            prop_assert_eq!(longs, 1, "a default drag starts after its long press");
        }
        if comp.first().is_some_and(|x| !x.0) {
            prop_assert!(!immediate, "an immediate drag never pans");
        }
        // A swipe ends a pan, released fast, from the edge it names.
        for (i, g) in log.iter().enumerate() {
            if let Gesture::Swipe { velocity, edge, region, .. } = *g {
                prop_assert!(i > 0 && matches!(log[i - 1], Gesture::Pan { phase: Phase::End, .. }), "a swipe right after its pan's end");
                prop_assert!(velocity >= FLING_PX_S * f.unit);
                if let Some(e) = edge {
                    prop_assert_eq!(region, Region::Edge(e));
                }
            }
        }
    }

    /// Right to left mirrors the regions: what is leading on one side is leading on the other.
    #[test]
    fn right_to_left_mirrors_every_region(w in 100.0f32..800.0, h in 100.0f32..800.0, x in 0.0f32..1.0, y in 0.0f32..1.0, unit in 1.0f32..3.0) {
        let ltr = Frame::new((w, h), unit);
        let rtl = Frame { rtl: true, ..ltr };
        let p = (x * w, y * h);
        prop_assert_eq!(ltr.region(p), rtl.region((w - p.0, p.1)));
        // A corner is never also an edge: each point has one region.
        let leading = matches!(ltr.region(p), Region::Edge(Edge::Leading));
        prop_assert!(!(leading && p.1 < ltr.top_zone * unit));
    }

    /// A point goes down a path of frames and comes back where it was; between two paths through
    /// their common parent too.
    #[test]
    fn points_convert_down_and_back(origins in prop::collection::vec((-400.0f32..400.0, -400.0f32..400.0), 1..6), other in prop::collection::vec((-400.0f32..400.0, -400.0f32..400.0), 1..6), p in (-500.0f32..500.0, -500.0f32..500.0)) {
        let path = |o: &[(f32, f32)]| FramePath(o.iter().map(|&o| Frame::new((100.0, 100.0), 1.0).at(o)).collect());
        let (a, b) = (path(&origins), path(&other));
        let back = a.to_outer(a.to_inner(p));
        prop_assert!((back.0 - p.0).abs() < 1e-2 && (back.1 - p.1).abs() < 1e-2);
        let there = a.convert(p, &b);
        let again = b.convert(there, &a);
        prop_assert!((again.0 - p.0).abs() < 1e-2 && (again.1 - p.1).abs() < 1e-2);
    }
}
