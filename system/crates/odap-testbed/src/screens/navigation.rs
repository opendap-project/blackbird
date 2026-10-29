//! The Navigation screen's stops (input events §3.7), each navigated by blackbird-ui-input's nav: a
//! shelf of albums, the days of the week, and a player whose seek is adjustable inside it (two
//! levels).

use std::{cell::RefCell, rc::Rc};

use blackbird_ui_input::{
    nav::{self, Effect, NavNode, Navigator},
    nested,
};
use blackbird_ui_layout::{
    focus as vfocus,
    window::{Overscan, window},
};
use slint::{ModelRc, VecModel};

use crate::{
    NavTile, NavView, Testbed,
    keys::{Arrows, Router},
};

/// The shelf: ten thousand albums, only a window of them built.
const SHELF: usize = 10_000;

/// The days stop's children, in reading order.
const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/// The stops and their state.
struct Stops {
    shelf: Navigator,
    /// The album the focus is on inside the shelf, by index (its key).
    shelf_index: usize,
    player: Navigator,
    days: Navigator,
    shelf_scroll: nested::Scroller,
    seek: f32,
}

impl Stops {
    fn new() -> Self {
        Stops {
            // The shelf is a stop over a virtual list (input events §3.7): nav keeps its levels
            // with one child standing for the list, and blackbird-ui-layout walks the list's items by
            // index.
            shelf: Navigator::new(NavNode::group(
                "screen",
                vec![NavNode::stop("shelf", vec![NavNode::leaf("album")])],
            )),
            shelf_index: 0,
            player: Navigator::new(NavNode::group(
                "screen",
                vec![NavNode::stop(
                    "player",
                    vec![NavNode::leaf("play"), NavNode::adjustable("seek")],
                )],
            )),
            // The days of the week: cyclic content, so the stop goes round (blackbird-ui-input's
            // ends); the shelf above keeps the default and stops.
            days: Navigator::new(NavNode::group(
                "screen",
                vec![
                    NavNode::stop("days", DAYS.iter().map(|d| NavNode::leaf(d)).collect()).cycle(),
                ],
            )),
            // 72 px albums 80 px apart, in the 288 px of content.
            shelf_scroll: nested::Scroller::new(SHELF as f32 * 80.0 - 8.0, 288.0),
            seek: 30.0,
        }
    }

    /// The navigator of the stop called `name`.
    fn nav(&mut self, name: &str) -> &mut Navigator {
        match name {
            "shelf" => &mut self.shelf,
            "days" => &mut self.days,
            _ => &mut self.player,
        }
    }
}

pub(super) fn wire(ui: &Testbed, keys: &mut Router<Testbed>) {
    let stops = Rc::new(RefCell::new(Stops::new()));
    shelf_tiles(ui, 0.0);
    // The player shows its seek's value from the start, not from its first key.
    ui.set_nav_player(NavView {
        focus: "".into(),
        depth: 0,
        offset: 0.0,
        value: stops.borrow().seek,
    });
    keys.add(
        &["shelf", "player", "days"],
        Arrows::Raw,
        move |ui, name, key| stop_key(ui, &mut stops.borrow_mut(), name, key),
    );
}

/// A key on one of the stops, through its navigator; false lets the focus move on (past the stop
/// at its own level) or the screen go back.
fn stop_key(ui: &Testbed, st: &mut Stops, name: &str, key: &str) -> bool {
    let k = nav_key(key, ui.get_strip_rtl());
    if name == "shelf"
        && st.shelf.depth() == 1
        && let Some(said) = shelf_walk(st, k)
    {
        stop_view(ui, st, name, &said);
        return true;
    }
    let (handled, said) = match k {
        Some(k) => nav_effect(st, name, k),
        None => stop_own(st, name, key),
    };
    stop_view(ui, st, name, &said);
    handled
}

/// The navigator's key for a control's key: a keyboard's arrows in reading order, so mirrored
/// right to left.
fn nav_key(key: &str, rtl: bool) -> Option<nav::Key> {
    match (key, rtl) {
        ("next", _) => Some(nav::Key::Next),
        ("prev", _) => Some(nav::Key::Previous),
        ("activate", _) => Some(nav::Key::Action),
        ("escape", _) => Some(nav::Key::Escape),
        ("right", false) | ("left", true) => Some(nav::Key::Forward),
        ("left", false) | ("right", true) => Some(nav::Key::Backward),
        _ => None,
    }
}

/// Inside the engaged shelf, Volume and the arrows walk its list by index (blackbird-ui-layout); past
/// either end the shelf lets go, as nav does for any stop. `None` for a key that does not walk.
fn shelf_walk(st: &mut Stops, k: Option<nav::Key>) -> Option<String> {
    let d = match k {
        Some(nav::Key::Next | nav::Key::Forward) => 1,
        Some(nav::Key::Previous | nav::Key::Backward) => -1,
        _ => return None,
    };
    Some(
        match vfocus::step(Some(st.shelf_index), d, SHELF, vfocus::Ends::Stop) {
            vfocus::Step::Item(i) | vfocus::Step::Wrapped(i) | vfocus::Step::Jumped(i) => {
                st.shelf_index = i;
                format!("moved to album {}", i + 1)
            }
            vfocus::Step::Leave => {
                st.shelf.key(nav::Key::Escape);
                "released shelf".into()
            }
        },
    )
}

/// A navigator's key on the stop `name`: whether it was taken, and what it did in words.
fn nav_effect(st: &mut Stops, name: &str, k: nav::Key) -> (bool, String) {
    let e = st.nav(name).key(k);
    if name == "shelf" {
        // Entering: at the first album, or the last from the far side (← in reading order).
        match &e {
            Effect::Engaged(_) => st.shelf_index = 0,
            Effect::Moved(n) if n == "album" => st.shelf_index = SHELF - 1,
            _ => {}
        }
    }
    let said = match &e {
        Effect::Moved(n) => format!("moved to {n}"),
        Effect::Wrapped(n) => format!("round to {n}"),
        Effect::Engaged(n) => format!("engaged {n}"),
        Effect::Released(n) => format!("released {n}"),
        Effect::Activated(n) if n == "album" => {
            format!("activated album {}", st.shelf_index + 1)
        }
        Effect::Activated(n) => format!("activated {n}"),
        Effect::Adjusted(n, d) => {
            st.seek = (st.seek + 2.0 * *d as f32).clamp(0.0, 100.0);
            format!("{n} {d:+}")
        }
        Effect::Refused(n) => format!("{n} refused"),
        Effect::AtEnd => "past the end: on".into(),
        Effect::Back => "back".into(),
        Effect::Closed { layer, to } => format!("closed {layer}, to {to}"),
    };
    (!matches!(e, Effect::AtEnd | Effect::Back), said)
}

/// The focus's own keys on the stop `name`: arriving on it, leaving it, a touch on a part.
fn stop_own(st: &mut Stops, name: &str, key: &str) -> (bool, String) {
    match key {
        // Keys arrived on the stop: it, not inside it.
        "enter" => {
            st.nav(name).touched(name);
            (true, format!("on {name}"))
        }
        // The focus hid or left: every level releases.
        "release" => {
            st.nav(name).release_all();
            st.nav(name).touched(name);
            (true, format!("{name} released"))
        }
        t if t.starts_with("touch:") => {
            let what = &t["touch:".len()..];
            match what.strip_prefix('a').and_then(|n| n.parse::<usize>().ok()) {
                Some(i) if name == "shelf" => {
                    st.shelf_index = i;
                    st.shelf.touched("album");
                    (true, format!("touched album {}", i + 1))
                }
                _ => {
                    st.nav(name).touched(what);
                    (true, format!("touched {what}"))
                }
            }
        }
        _ => (false, String::new()),
    }
}

/// What a stop shows after a key: the focus inside it, its level, and for the shelf, the built
/// window of albums scrolled to keep the focused one in view.
fn stop_view(ui: &Testbed, st: &mut Stops, name: &str, said: &str) {
    let shelf = name == "shelf";
    let nav = st.nav(name);
    let inside = nav.focus().filter(|f| *f != name).map(str::to_string);
    let depth = nav.depth() as i32;
    let focus = match inside.as_deref() {
        Some("album") => format!("a{}", st.shelf_index),
        Some(f) => f.to_string(),
        None => String::new(),
    };
    let mut offset = 0.0;
    if shelf {
        if inside.is_some() {
            let x = st.shelf_index as f32 * 80.0;
            let mut area = [nested::RevealArea {
                scroller: st.shelf_scroll,
                viewport: 288.0,
                at: 0.0,
                inset_start: 0.0,
                inset_end: 0.0,
            }];
            nested::reveal(&mut area, x, x + 72.0);
            st.shelf_scroll = area[0].scroller;
        }
        offset = st.shelf_scroll.offset;
        shelf_tiles(ui, offset);
    }
    let view = NavView {
        focus: focus.clone().into(),
        depth,
        offset,
        value: st.seek,
    };
    match name {
        "shelf" => ui.set_nav_shelf(view),
        "days" => ui.set_nav_days(view),
        _ => ui.set_nav_player(view),
    }
    if !said.is_empty() {
        let at = if focus.is_empty() {
            format!(", on {name}")
        } else {
            format!(", in {name}: {focus}")
        };
        ui.set_nav_text(format!("{said}\nlevel {depth}{at}").into());
    }
}

/// The albums to build for the shelf at `offset`: only the window around what shows.
fn shelf_tiles(ui: &Testbed, offset: f32) {
    let extents = blackbird_ui_layout::extents::Fixed {
        count: SHELF,
        size: 80.0,
    };
    let o = Overscan {
        around: 80.0,
        lead_s: 0.0,
        max_ahead: 0.0,
    };
    let tiles: Vec<NavTile> = window(&extents, offset, 288.0, 0.0, o)
        .map(|i| NavTile { index: i as i32 })
        .collect();
    ui.set_nav_tiles(ModelRc::new(VecModel::from(tiles)));
}
