//! Places the Transitions screen at a moment.

use opendap_input::anchor::Rect as ARect;
use opendap_motion::{
    interpolate::{Frame, frame},
    transition::End as TEnd,
};
use slint::{ModelRc, VecModel};

use super::state::{TR_PAGES, TR_W, Tr};
use crate::{Testbed, TrBox};

/// Places the screen at `now`; whether anything still moves.
pub(super) fn tr_render(ui: &Testbed, t: &mut Tr, now: f32) -> bool {
    let off = t.offset(now);
    ui.set_tr_pager_offset(off);
    ui.set_tr_page(t.page as i32);
    let p = t.morph.progress(now);
    morph(ui, t, p);
    ui.set_tr_rows(ModelRc::new(VecModel::from(rows(t, now))));
    ui.set_tr_text(
        format!(
            "page {} of {TR_PAGES}, offset {off:.0}\nmorph {p:.2}{}\nlines {}",
            t.page + 1,
            match t.morph.settled(now) {
                Some(TEnd::To) => ", open",
                Some(TEnd::From) => ", closed",
                None => ", moving",
            },
            if t.removed {
                "1, 3, 4 (2 removed)"
            } else {
                "1 to 4"
            }
        )
        .into(),
    );
    t.pager.running(now)
        || t.morph.running(now)
        || t.lines.running(now)
        || t.drag.is_some()
        || t.detail_drag.is_some()
}

fn tile_frame(r: ARect, radius: f32) -> Frame {
    Frame::new(r.origin.0, r.origin.1, r.size.0, r.size.1, radius)
}

fn tr_box(f: Frame, opacity: f32, label: String) -> TrBox {
    TrBox {
        x: f.x,
        y: f.y,
        w: f.w,
        h: f.h,
        radius: f.radius,
        opacity,
        label: label.into(),
    }
}

/// The tiles and the detail the opened one becomes, at the morph's progress `p`.
fn morph(ui: &Testbed, t: &Tr, p: f32) {
    let pairs = t.matched.pairs("tiles");
    let tiles: Vec<TrBox> = pairs
        .iter()
        .enumerate()
        .map(|(i, (_, from, _))| {
            // The opened tile is the detail now; the others recede.
            let opacity = if i == t.tile && p > 0.0 {
                0.0
            } else {
                1.0 - 0.6 * p
            };
            tr_box(tile_frame(*from, 8.0), opacity, format!("Album {}", i + 1))
        })
        .collect();
    ui.set_tr_tiles(ModelRc::new(VecModel::from(tiles)));
    let (_, from, to) = pairs[t.tile];
    let f = frame(tile_frame(from, 8.0), tile_frame(to, 16.0), p);
    ui.set_tr_detail(tr_box(
        f,
        if p > 0.001 { 1.0 } else { 0.0 },
        format!("Album {}", t.tile + 1),
    ));
}

/// The lines, each as tall as its presence.
fn rows(t: &mut Tr, now: f32) -> Vec<TrBox> {
    let mut y = 0.0;
    t.lines
        .frame(now)
        .into_iter()
        .map(|(k, _, prog)| {
            let h = 40.0 * prog;
            let r = TrBox {
                x: 0.0,
                y,
                w: TR_W,
                h,
                radius: 0.0,
                opacity: prog,
                label: format!("Line {}", k + 1).into(),
            };
            y += h;
            r
        })
        .collect()
}
