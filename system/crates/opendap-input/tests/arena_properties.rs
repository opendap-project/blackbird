//! The gesture arena's invariants over generated arenas and contacts (ADR 0014, level L1): any
//! set of recognizers (tap, double tap, long press, drags on each axis and start, a sequenced long
//! press then drag), in any order, each competing or simultaneous, fed any contact with time moving
//! as live time does. One owner per contact, nothing activated by a contact the system took, and
//! no recognizer left waiting once the contact is over. A failure is shrunk to the smallest arena
//! and contact that breaks one.

use std::collections::HashSet;

use opendap_input::arena::{
    Arena, DoubleTap, Drag, Input, LongPress, Outcome, Priority, Sequenced, Tap,
};
use opendap_input::gesture::{Axis, DragOptions, DragStart};
use opendap_input::harness::{Script, play};
use opendap_input::thresholds::{DOUBLE_TAP_MS, LONG_MS};
use proptest::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Tap,
    DoubleTap,
    LongPress,
    DragH,
    DragV,
    DragAfterLong,
    LongThenDrag,
}

const KINDS: [Kind; 7] = [
    Kind::Tap,
    Kind::DoubleTap,
    Kind::LongPress,
    Kind::DragH,
    Kind::DragV,
    Kind::DragAfterLong,
    Kind::LongThenDrag,
];

fn name(k: Kind) -> &'static str {
    match k {
        Kind::Tap => "tap",
        Kind::DoubleTap => "double tap",
        Kind::LongPress => "long press",
        Kind::DragH => "drag horizontal",
        Kind::DragV => "drag vertical",
        Kind::DragAfterLong => "drag after long press",
        Kind::LongThenDrag => "long press then drag",
    }
}

fn arena(members: &[(Kind, bool)]) -> Arena {
    let immediate = |axis| DragOptions {
        axis,
        start: DragStart::Immediate,
    };
    members.iter().fold(Arena::new(), |a, &(k, simultaneous)| {
        let p = if simultaneous {
            Priority::Simultaneous
        } else {
            Priority::Normal
        };
        match k {
            Kind::Tap => a.add(Tap::new(1.0), p),
            Kind::DoubleTap => a.add(DoubleTap::new(1.0), p),
            Kind::LongPress => a.add(LongPress::new(1.0), p),
            Kind::DragH => a.add(Drag::new(name(k), 1.0, immediate(Axis::Horizontal)), p),
            Kind::DragV => a.add(Drag::new(name(k), 1.0, immediate(Axis::Vertical)), p),
            Kind::DragAfterLong => a.add(Drag::new(name(k), 1.0, DragOptions::default()), p),
            Kind::LongThenDrag => a.add(
                Sequenced::new(
                    name(k),
                    LongPress::new(1.0),
                    Drag::new("drag", 1.0, immediate(Axis::Free)),
                ),
                p,
            ),
        }
    })
}

/// A set of distinct recognizers, in some order, each competing or simultaneous.
fn members() -> impl Strategy<Value = Vec<(Kind, bool)>> {
    (
        Just(KINDS.to_vec()).prop_shuffle(),
        1usize..=7,
        prop::collection::vec(prop::bool::weighted(0.2), 7),
    )
        .prop_map(|(ks, n, sim)| ks.into_iter().take(n).zip(sim).collect())
}

/// Distinct recognizers that include `k`, competing, at some place among them.
fn members_with(k: Kind) -> impl Strategy<Value = Vec<(Kind, bool)>> {
    (members(), any::<prop::sample::Index>()).prop_map(move |(mut ms, at)| {
        ms.retain(|m| m.0 != k);
        let i = at.index(ms.len() + 1);
        ms.insert(i, (k, false));
        ms
    })
}

/// One contact: down, moves and pauses (time only goes forward), then a lift or the system taking it.
fn contact(start: u64) -> impl Strategy<Value = (Vec<Input>, bool)> {
    let moves = prop::collection::vec((-30.0f32..30.0, -30.0f32..30.0, 1u64..300), 0..8);
    (moves, any::<bool>(), prop::bool::weighted(0.2), 1u64..900).prop_map(
        move |(ms, still, cancel, last)| {
            let mut t = start;
            let mut at = (100.0f32, 100.0f32);
            let mut inputs = vec![Input::Down { at, ms: t }];
            for (dx, dy, dt) in ms {
                t += dt;
                if !still {
                    at = (at.0 + dx, at.1 + dy);
                }
                inputs.push(Input::Move { at, ms: t });
            }
            if cancel {
                inputs.push(Input::Cancel);
            } else {
                inputs.push(Input::Up { at, ms: t + last });
            }
            (inputs, cancel)
        },
    )
}

/// Feeds the inputs as live time would (the deadlines ticked before each input that comes after
/// them), then the remaining deadlines: every outcome, with the index of the input it came at
/// (`inputs.len()` for the deadlines after the last).
fn drive(arena: &mut Arena, inputs: &[Input]) -> Vec<(usize, Outcome)> {
    let mut out = Vec::new();
    for (i, input) in inputs.iter().enumerate() {
        let at = match *input {
            Input::Down { ms, .. }
            | Input::Move { ms, .. }
            | Input::Up { ms, .. }
            | Input::Tick { ms } => Some(ms),
            Input::Cancel => None,
        };
        if let Some(at) = at {
            let mut guard = 0;
            while let Some(d) = arena.deadline().filter(|d| *d <= at) {
                out.push((i, arena.feed(Input::Tick { ms: d })));
                guard += 1;
                if guard > 16 {
                    break;
                }
            }
        }
        out.push((i, arena.feed(*input)));
    }
    let mut guard = 0;
    while let Some(d) = arena.deadline() {
        out.push((inputs.len(), arena.feed(Input::Tick { ms: d })));
        guard += 1;
        if guard > 16 {
            break;
        }
    }
    out
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 4000, ..ProptestConfig::default() })]

    /// One contact: at most one competing member owns it, each member wins or loses once, never
    /// both; a contact the system took activates nothing from then on; once it is over, no member
    /// waits and no deadline is left.
    #[test]
    fn one_owner_per_contact_and_nothing_left_waiting(ms in members(), (inputs, cancelled) in contact(0)) {
        let mut a = arena(&ms);
        let log = drive(&mut a, &inputs);
        let competing: HashSet<&str> = ms.iter().filter(|m| !m.1).map(|m| name(m.0)).collect();
        let won: Vec<&str> = log.iter().flat_map(|(_, o)| o.won.iter().map(String::as_str)).collect();
        let lost: Vec<&str> = log.iter().flat_map(|(_, o)| o.lost.iter().map(String::as_str)).collect();
        prop_assert!(won.iter().filter(|w| competing.contains(*w)).count() <= 1, "one owner: {:?}", won);
        let mut once = HashSet::new();
        prop_assert!(won.iter().all(|w| once.insert(*w)), "each wins once: {:?}", won);
        let mut once = HashSet::new();
        prop_assert!(lost.iter().all(|l| once.insert(*l)), "each loses once: {:?}", lost);
        prop_assert!(won.iter().all(|w| !lost.contains(w)), "won and lost: {:?} / {:?}", won, lost);
        if cancelled {
            let at = inputs.len() - 1;
            let late: Vec<&String> = log.iter().filter(|(i, _)| *i >= at).flat_map(|(_, o)| &o.won).collect();
            prop_assert!(late.is_empty(), "the system took the contact, yet {:?} won", late);
        }
        prop_assert_eq!(a.deadline(), None, "no deadline left");
        prop_assert!(a.pending().is_empty(), "still waiting after the contact: {:?}", a.pending());
    }

    /// A tap is a tap: whatever else listens, a contact that does not move and lifts before the long
    /// press is won by the tap (an opt-in double tap that did not happen only makes it wait).
    #[test]
    fn a_tap_always_ends_as_a_tap(ms in members_with(Kind::Tap), hold in 0u64..LONG_MS) {
        let mut a = arena(&ms);
        let won = play(&mut a, &Script::tap((100.0, 100.0), 0, hold));
        prop_assert!(won.iter().any(|w| w == "tap"), "{:?} for {:?}", won, ms);
    }

    /// Two taps in time, with the double tap listed before the tap (the exclusive composition): the
    /// double tap wins, and the tap never fires for either of them.
    #[test]
    fn a_double_tap_listed_first_takes_both_taps(ms in members_with(Kind::Tap), at in any::<prop::sample::Index>(), gap in 1u64..DOUBLE_TAP_MS, hold in 0u64..200) {
        let mut ms = ms;
        ms.retain(|m| m.0 != Kind::DoubleTap);
        let tap = ms.iter().position(|m| m.0 == Kind::Tap).unwrap();
        ms.insert(at.index(tap + 1), (Kind::DoubleTap, false));
        let mut a = arena(&ms);
        let first = Script::tap((100.0, 100.0), 0, hold);
        let second = Script::tap((102.0, 101.0), hold + gap, hold);
        let won = play(&mut a, &first.then(second));
        prop_assert!(won.iter().any(|w| w == "double tap"), "{:?}", won);
        prop_assert!(!won.iter().any(|w| w == "tap"), "{:?}", won);
    }

    /// Held still past the long-press time, then lifted: the long press wins, whatever else listens.
    #[test]
    fn a_still_hold_is_a_long_press(ms in members_with(Kind::LongPress), extra in 0u64..2000) {
        let mut a = arena(&ms);
        let won = play(&mut a, &Script::tap((100.0, 100.0), 0, LONG_MS + extra));
        prop_assert!(won.iter().any(|w| w == "long press"), "{:?} for {:?}", won, ms);
    }

    /// A recording replays to the same outcome, and its text form reads back to the same script: a
    /// bug report from a device becomes a test.
    #[test]
    fn a_recording_replays_the_same(ms in members(), (inputs, _) in contact(0), (more, _) in contact(5000)) {
        let script = Script { inputs: inputs.into_iter().chain(more).collect() };
        let text = script.to_text();
        let read = Script::from_text(&text).expect("its own text reads back");
        prop_assert_eq!(&read, &script);
        let once = play(&mut arena(&ms), &script);
        let again = play(&mut arena(&ms), &read);
        prop_assert_eq!(once, again);
    }
}
