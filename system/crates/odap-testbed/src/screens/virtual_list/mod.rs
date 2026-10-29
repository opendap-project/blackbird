//! The Virtual screen: a hundred thousand rows scrolled by the list's own pan. blackbird-ui-layout
//! says which rows to build (the window, with prefetch ahead by velocity), measures them as they
//! are built (every seventh is taller), reuses a bounded set of slots by key, loads the rows' data
//! a page at a time (each page takes 250 ms, as a slow card would), and decodes covers only while
//! the scroll is slow, within a cache's budget.
//!
//! `render` lays the window out, `focus` walks it with the buttons, `pan` scrolls it by touch.

mod focus;
mod pan;
mod render;

use std::{cell::RefCell, rc::Rc, time::Instant};

use blackbird_ui_layout::{
    data::{
        cache::Cache,
        gate::Gate,
        pages::{Pages, Paging},
    },
    extents::Measured,
    focus as vfocus,
    recycle::Pool,
};
use blackbird_ui_motion::{
    animator::{Animator, Kind as AnimKind},
    easing::Bezier,
};
use slint::{Timer, VecModel};

use crate::{Testbed, VRow};

const VCOUNT: usize = 100_000;
// Sections of a thousand rows, each headed by a 28 px header above its first row, pinned at the
// list's top while it passes (blackbird-ui-layout's sticky).
const SECTION: usize = 1000;
const HEAD: f32 = 28.0;
// The reveal's scroll: a curve the system starts, retargeted by every press (ui-model §8).
const REVEAL: AnimKind = AnimKind::Timed {
    duration: 0.18,
    curve: Bezier::STANDARD,
};

/// The list's state.
struct Virtual {
    extents: Measured,
    pool: Pool<usize>,
    model: Rc<VecModel<VRow>>,
    offset: f32,
    viewport: f32,
    velocity: f32,
    /// The focused item, by index (its key here), never by slot.
    focused: Option<usize>,
    activated: Option<usize>,
    /// Action on a placeholder: it waits for its data (input events §3.7).
    activation: vfocus::Activation,
    data: Data,
    reveal: Reveal,
    touch: Touch,
}

/// The rows' data: its pages, the decoded covers, and what the window's moves cost.
struct Data {
    pages: Pages,
    covers: Cache<usize, ()>,
    gate: Gate,
    rebound: usize,
    cancelled: usize,
    evicted: usize,
}

/// The scroll the list makes by itself: the reveal toward the focused item, or a fling.
struct Reveal {
    /// The reveal's clock (its animator's time).
    clock: Instant,
    /// The reveal's scroll toward the focused item.
    animator: Animator,
    /// A free fling after the finger left (blackbird-ui-motion's decay): where it began, its velocity
    /// and when, by the reveal's clock; and where it is projected to stop.
    fling: Option<(f32, f32, f32)>,
    fling_end: Option<f32>,
}

/// The contact on the list.
struct Touch {
    last_y: f32,
    last_ms: u64,
    /// How far the contact moved: a tap when it did not.
    travel: f32,
}

type Shared = Rc<RefCell<Virtual>>;

impl Virtual {
    fn new() -> Self {
        Virtual {
            extents: Measured::new(VCOUNT, 48.0),
            pool: Pool::new(),
            model: Rc::new(VecModel::default()),
            offset: 0.0,
            viewport: 260.0,
            velocity: 0.0,
            focused: None,
            activated: None,
            activation: vfocus::Activation::default(),
            data: Data {
                pages: Pages::new(Paging::default(), Some(VCOUNT)),
                // About sixty decoded covers at most (proposed: to measure).
                covers: Cache::new(60),
                gate: Gate::default(),
                rebound: 0,
                cancelled: 0,
                evicted: 0,
            },
            reveal: Reveal {
                clock: Instant::now(),
                animator: Animator::new(0.0, REVEAL),
                fling: None,
                fling_end: None,
            },
            touch: Touch {
                last_y: 0.0,
                last_ms: 0,
                travel: 0.0,
            },
        }
    }
}

pub(super) fn wire(ui: &Testbed, opened: &mut super::Openers<Testbed>) {
    let st: Shared = Rc::new(RefCell::new(Virtual::new()));
    let timer = Rc::new(Timer::default());
    focus::wire(ui, st.clone(), timer.clone());
    pan::wire(ui, st, timer);
    // Opened from the list: the scroll is still, the window laid out.
    opened.add("Virtual", |ui| ui.invoke_vpan("open".into(), 0.0, 0.0));
}
