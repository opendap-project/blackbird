//! The Transitions screen's state: the pager (opendap-layout::pager), the morph between a tile
//! and its detail (opendap-motion::transition, opendap-input::matched), and the lines that enter
//! and leave (opendap-motion::presence).

use opendap_input::{
    anchor::Rect as ARect,
    matched::{Matched, Side as MSide},
};
use opendap_layout::pager::{resist, settle};
use opendap_motion::{
    animator::{Animator, Kind as AnimKind},
    easing::Bezier,
    presence::Presence,
    spring::Spring,
    transition::{End as TEnd, Transition},
};

/// The screen's state.
pub(super) struct Tr {
    /// The pager: its offset (pixels into the strip, in reading order), the page it rests on, its
    /// settling animation, and the drag in progress (where it began, the last move, its velocity).
    pub(super) page: usize,
    pub(super) pager: Animator,
    pub(super) drag: Option<(f32, f32, f32, f32)>,
    pub(super) velocity: f32,
    /// The morph: the tile it opened from and its transition; a downward drag on the detail.
    pub(super) morph: Transition,
    pub(super) tile: usize,
    pub(super) detail_drag: Option<(f32, f32)>,
    pub(super) matched: Matched,
    /// The lines and whether line 2 is removed.
    pub(super) lines: Presence<usize>,
    pub(super) removed: bool,
}

/// The stage's sizes, in design pixels: the content's width, the morph's stage, its tiles.
pub(super) const TR_W: f32 = 288.0;
pub(super) const TR_PAGES: usize = 3;
const TR_DETAIL: (f32, f32, f32, f32) = (0.0, 0.0, 288.0, 190.0);

impl Tr {
    pub(super) fn new() -> Self {
        let mut matched = Matched::default();
        for i in 0..3 {
            matched.declare(
                "tiles",
                &i.to_string(),
                MSide::Source,
                ARect::new(16.0 + i as f32 * 96.0, 120.0, 64.0, 64.0),
            );
            matched.declare(
                "tiles",
                &i.to_string(),
                MSide::Destination,
                ARect::new(TR_DETAIL.0, TR_DETAIL.1, TR_DETAIL.2, TR_DETAIL.3),
            );
        }
        let mut lines = Presence::default();
        lines.update(&[0, 1, 2, 3], -10.0);
        Tr {
            page: 0,
            pager: Animator::new(0.0, AnimKind::Spring(Spring::SETTLE)),
            drag: None,
            velocity: 0.0,
            morph: Transition::new(TEnd::From, TR_DETAIL.3),
            tile: 0,
            detail_drag: None,
            matched,
            lines,
            removed: false,
        }
    }

    pub(super) fn offset(&self, now: f32) -> f32 {
        match self.drag {
            Some((x0, _, _, off0)) => {
                let raw = off0 + (x0 - self.drag.unwrap().1);
                let max = (TR_PAGES - 1) as f32 * TR_W;
                // Past the ends the strip resists the finger (rubber band).
                if raw < 0.0 {
                    resist(raw, TR_W)
                } else if raw > max {
                    max + resist(raw - max, TR_W)
                } else {
                    raw
                }
            }
            None => self.pager.sample(now).0,
        }
    }

    pub(super) fn pager_touch(&mut self, kind: &str, x: f32, now: f32) {
        match kind {
            "down" => {
                let off = self.offset(now);
                self.drag = Some((x, x, now, off));
                self.velocity = 0.0;
            }
            "move" => {
                if let Some((x0, last, t, off0)) = self.drag {
                    let dt = (now - t).max(0.001);
                    self.velocity = (last - x) / dt;
                    self.drag = Some((x0, x, now, off0));
                }
            }
            _ => {
                let off = self.offset(now);
                // Only recent motion counts: a finger that rested before lifting has no velocity
                // (every velocity tracker's window, input events §7).
                if self.drag.is_some_and(|(_, _, t, _)| now - t > 0.05) {
                    self.velocity = 0.0;
                }
                self.drag = None;
                let from = self.page;
                self.page = settle(
                    from,
                    off,
                    self.velocity,
                    TR_W,
                    TR_PAGES,
                    opendap_layout::pager::Ends::Stop,
                );
                self.pager = Animator::new(off, AnimKind::Spring(Spring::SETTLE));
                self.pager
                    .retarget(self.page as f32 * TR_W, now, Some(self.velocity));
            }
        }
    }

    pub(super) fn turn(&mut self, d: i32, now: f32, reduce: bool) {
        // Past the first or the last page the pager stops (opendap-input's ends, the default).
        let to =
            match opendap_layout::pager::Ends::Stop.step(self.page, i64::from(d), Some(TR_PAGES)) {
                opendap_input::ends::Step::To(p) | opendap_input::ends::Step::Wrapped(p) => p,
                opendap_input::ends::Step::Past => self.page,
            };
        self.page = to;
        let off = self.offset(now);
        self.pager = Animator::new(
            off,
            AnimKind::Timed {
                duration: 0.2,
                curve: Bezier::STANDARD,
            },
        );
        self.pager.reduce_motion = reduce;
        self.pager.retarget(to as f32 * TR_W, now, None);
    }

    pub(super) fn open(&mut self, tile: usize, now: f32, reduce: bool) {
        self.morph.reduce_motion = reduce;
        let open = self.morph.settled(now) == Some(TEnd::To)
            || self.morph.running(now) && self.morph.progress(now) > 0.0 && self.tile == tile;
        if open {
            self.morph.go(TEnd::From, now);
        } else {
            self.tile = tile;
            self.morph.go(TEnd::To, now);
        }
    }

    pub(super) fn detail_touch(&mut self, kind: &str, y: f32, now: f32) {
        match kind {
            "down" => {
                self.morph.grab(now);
                self.detail_drag = Some((y, now));
                self.velocity = 0.0;
            }
            "move" => {
                if let Some((last, t)) = self.detail_drag {
                    let dt = (now - t).max(0.001);
                    // Down is toward closing: the transition's way back.
                    self.morph.drag(-(y - last));
                    self.velocity = -(y - last) / dt;
                    self.detail_drag = Some((y, now));
                }
            }
            _ => {
                if self.detail_drag.is_some_and(|(_, t)| now - t > 0.05) {
                    self.velocity = 0.0;
                }
                let moved = self
                    .detail_drag
                    .take()
                    .is_some_and(|_| self.velocity.abs() > 1.0);
                if moved || self.morph.progress(now) < 0.98 {
                    self.morph.release(self.velocity, now);
                } else {
                    // A tap on the open detail: back.
                    self.morph.release(0.0, now);
                    self.morph.go(TEnd::From, now);
                }
            }
        }
    }

    pub(super) fn toggle_line(&mut self, now: f32, reduce: bool) {
        self.removed = !self.removed;
        self.lines.reduce_motion = reduce;
        let keys: Vec<usize> = (0..4).filter(|k| !(self.removed && *k == 1)).collect();
        self.lines.update(&keys, now);
    }

    pub(super) fn key(
        &mut self,
        name: &str,
        key: &str,
        now: f32,
        reduce: bool,
        _rtl: bool,
    ) -> bool {
        match (name, key) {
            ("pager", "inc") => self.turn(1, now, reduce),
            ("pager", "dec") => self.turn(-1, now, reduce),
            ("morph", "activate") => self.open(self.tile, now, reduce),
            _ => return false,
        }
        true
    }
}
