//! Dispatch's and the node tree's invariants over generated trees, listeners and events (ADR 0014,
//! level L1): any path from the activity to a target, listeners at any node by exact name,
//! wildcard or any depth, with every combination of directives, and events at any times. An event
//! reaches only its path, in capture, target and bubble order, and the directives do what they say.
//! A failure is shrunk to the smallest tree and listeners that break one.

use std::collections::{HashMap, HashSet};

use blackbird_ui_input::dispatch::{
    Action, Directives, Intent, Listener, Phase, dispatch, resolve,
};
use blackbird_ui_input::gesture::Frame;
use blackbird_ui_input::node::{Node, NodeId, NodeKind, NodePath, Selector};
use proptest::prelude::*;

const KINDS: [NodeKind; 5] = [
    NodeKind::Layer,
    NodeKind::Group,
    NodeKind::Container,
    NodeKind::Region,
    NodeKind::Element,
];

/// A path: the activity, then nodes of any kind with short names (so selectors collide).
fn path() -> impl Strategy<Value = NodePath> {
    prop::collection::vec((0usize..5, 0u8..3, (-50.0f32..50.0, -50.0f32..50.0)), 1..6).prop_map(
        |v| {
            let mut nodes = vec![Node::new(
                NodeId::new(NodeKind::Activity, "music"),
                Frame::new((320.0, 480.0), 1.0),
            )];
            for (k, n, o) in v {
                nodes.push(Node::new(
                    NodeId::new(KINDS[k], format!("n{n}")),
                    Frame::new((100.0, 100.0), 1.0).at(o),
                ));
            }
            NodePath(nodes)
        },
    )
}

/// A selector aimed at some depth of `p`: its exact address, the same with a wildcard or a name
/// prefix, or `**` in front; sometimes one that names nothing on the path.
fn selector_for(p: &NodePath, depth: usize, style: u8) -> String {
    let segs: Vec<String> = p.0[..=depth].iter().map(|n| n.id.to_string()).collect();
    match style % 5 {
        0 => segs.join("/"),
        1 => {
            let mut s = segs.clone();
            let last = s.len() - 1;
            s[last] = format!("{}:*", p.0[depth].id.kind.as_str());
            s.join("/")
        }
        2 => format!("**/{}", segs.last().unwrap()),
        3 => {
            let mut s = segs.clone();
            let last = s.len() - 1;
            s[last] = "*".into();
            s.join("/")
        }
        _ => "activity:music/**/element:nowhere".into(),
    }
}

#[derive(Debug, Clone)]
struct Spec {
    depth: usize,
    style: u8,
    event: bool,
    d: Directives,
}

fn spec() -> impl Strategy<Value = Spec> {
    (
        0usize..6,
        0u8..5,
        prop::bool::weighted(0.85),
        prop::array::uniform6(prop::bool::weighted(0.25)),
        prop::option::weighted(0.2, 1u64..100),
        prop::option::weighted(0.1, 1u64..100),
    )
        .prop_map(|(depth, style, event, b, throttle_ms, debounce_ms)| Spec {
            depth,
            style,
            event,
            d: Directives {
                once: b[0],
                stop: b[1],
                prevent: b[2],
                capture: b[3],
                self_only: b[4],
                passive: b[5],
                throttle_ms,
                debounce_ms,
            },
        })
}

fn listeners(p: &NodePath, specs: &[Spec]) -> Vec<Listener> {
    specs
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let depth = s.depth % p.0.len();
            let sel = selector_for(p, depth, s.style);
            Listener::new(
                i as u32,
                &sel,
                if s.event { "press" } else { "scroll" },
                s.d,
            )
            .expect("a well-formed selector")
        })
        .collect()
}

fn prefix(p: &NodePath, d: usize) -> NodePath {
    NodePath(p.0[..=d].to_vec())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 4000, ..ProptestConfig::default() })]

    #[test]
    fn an_event_travels_its_path_and_the_directives_hold(p in path(), specs in prop::collection::vec(spec(), 0..12), gaps in prop::collection::vec(0u64..80, 1..6)) {
        let mut ls = listeners(&p, &specs);
        let target = p.0.len() - 1;
        let mut ms = 0u64;
        let mut ran_once: HashSet<u32> = HashSet::new();
        let mut last_run: HashMap<u32, u64> = HashMap::new();
        for g in gaps {
            ms += g;
            let route = dispatch(&mut ls, "press", &p, ms);
            let spec_of = |id: u32| &specs[id as usize];
            // Order: capture top-down, the target, bubble bottom-up.
            let rank = |ph: Phase| match ph { Phase::Capture => 0, Phase::Target => 1, Phase::Bubble => 2 };
            for w in route.runs.windows(2) {
                let ((_, d0, p0), (_, d1, p1)) = (w[0], w[1]);
                prop_assert!(rank(p0) <= rank(p1), "phases in order: {:?}", route.runs);
                if p0 == p1 && p0 == Phase::Capture { prop_assert!(d0 <= d1, "capture goes down"); }
                if p0 == p1 && p0 == Phase::Bubble { prop_assert!(d0 >= d1, "bubble goes up"); }
            }
            let mut seen_this: HashSet<u32> = HashSet::new();
            for &(id, d, ph) in &route.runs {
                let s = spec_of(id);
                let l = &ls[id as usize];
                prop_assert!(s.event, "only listeners of this event run");
                prop_assert!(d <= target);
                prop_assert!(l.at.matches(&prefix(&p, d)), "only on the event's path: {} at depth {}", p.address(), d);
                prop_assert_eq!(ph == Phase::Target, d == target, "the target phase is the target's");
                if ph == Phase::Capture { prop_assert!(s.d.capture); }
                if ph == Phase::Bubble { prop_assert!(!s.d.capture); }
                if s.d.self_only { prop_assert_eq!(d, target, "self runs only at the target"); }
                prop_assert!(s.d.debounce_ms.is_none(), "a debounced handler runs later, not now");
                if s.d.once {
                    prop_assert!(ran_once.insert(id), "once runs once, ever");
                }
                if let Some(every) = s.d.throttle_ms
                    && seen_this.insert(id)
                    && let Some(t) = last_run.get(&id)
                {
                    prop_assert!(ms - t >= every, "throttled to one call every {} ms", every);
                }
                last_run.insert(id, ms);
            }
            // Stop: nothing runs at a level after the one where a stopping handler ran.
            if let Some(i) = route.runs.iter().position(|r| spec_of(r.0).d.stop) {
                let (_, d, ph) = route.runs[i];
                prop_assert!(route.runs[i..].iter().all(|r| r.1 == d && (r.2 == ph || (ph == Phase::Target))), "stopped at {:?}: {:?}", (d, ph), route.runs);
            }
            // The default is prevented exactly when a handler that may prevent did.
            let prevented = route.runs.iter().any(|r| spec_of(r.0).d.prevent && !spec_of(r.0).d.passive);
            prop_assert_eq!(route.default_prevented, prevented);
            for &(id, due) in &route.debounced {
                prop_assert_eq!(due, ms + spec_of(id).d.debounce_ms.unwrap());
            }
        }
    }

    /// An intent goes to the nearest enabled action from the focus up: what it finds is enabled,
    /// handles the intent and sits on the focus's path, and nothing nearer would have.
    #[test]
    fn an_intent_reaches_the_nearest_enabled_action(p in path(), acts in prop::collection::vec((0usize..6, 0u8..5, any::<bool>(), any::<bool>()), 0..8)) {
        let actions: Vec<Action> = acts
            .iter()
            .enumerate()
            .map(|(i, &(d, style, enabled, back))| Action {
                at: Selector::parse(&selector_for(&p, d % p.0.len(), style)).unwrap(),
                intent: Intent(if back { "back".into() } else { "activate".into() }),
                enabled,
                name: format!("a{i}"),
            })
            .collect();
        let intent = Intent("activate".into());
        let found = resolve(&actions, &intent, &p);
        let handles = |a: &Action, d: usize| a.enabled && a.intent == intent && a.at.matches(&prefix(&p, d));
        let nearest = (0..p.0.len()).rev().find(|&d| actions.iter().any(|a| handles(a, d)));
        match (found, nearest) {
            (Some(a), Some(d)) => prop_assert!(handles(a, d), "{} is not the nearest", a.name),
            (None, None) => {}
            (f, n) => prop_assert!(false, "found {:?}, nearest at {:?}", f.map(|a| &a.name), n),
        }
    }

    /// Addresses and selectors: a path's own address selects it, `**` selects everything, the
    /// common ancestor is inside both, and a point converts to another node and back.
    #[test]
    fn the_tree_names_and_converts_consistently(a in path(), b in path(), pt in (-200.0f32..200.0, -200.0f32..200.0)) {
        prop_assert!(Selector::parse(&a.address()).unwrap().matches(&a));
        prop_assert!(Selector::parse("**").unwrap().matches(&a));
        prop_assert!(Selector::parse("activity:music/**").unwrap().matches(&a));
        let c = a.common(&b);
        prop_assert!(a.within(&c) && b.within(&c), "the common ancestor contains both");
        prop_assert!(!c.0.is_empty(), "the same activity");
        for k in KINDS {
            if let Some(up) = a.up_to(k, None) {
                prop_assert!(a.within(&up));
                prop_assert_eq!(up.target().unwrap().id.kind, k);
                prop_assert!(!a.0[up.0.len()..].iter().any(|n| n.id.kind == k), "the nearest of its kind");
            }
        }
        let there = a.convert(pt, &b);
        let back = b.convert(there, &a);
        prop_assert!((back.0 - pt.0).abs() < 1e-2 && (back.1 - pt.1).abs() < 1e-2);
    }
}
