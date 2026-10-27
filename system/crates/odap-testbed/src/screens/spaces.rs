//! The Spaces screen: the input primitives in use (opendap-input's space and node). A list
//! scrolled by its own pan: the finger moves in the viewport, while in the content, which follows
//! it, it stays put; the row under it, its address and its own coordinates.

use std::{cell::RefCell, rc::Rc, time::Instant};

use opendap_input::{gesture, node, space};
use opendap_layout::focus as vfocus;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::{
    Testbed,
    keys::{Arrows, Router},
};

const ROW: f32 = 42.0;
const ROWS: usize = 20;

struct ListState {
    track: space::Track,
    offset: f32,
    offset_at_press: f32,
    last_y: f32,
    /// The row the buttons focus, by index (its key).
    focused: Option<usize>,
}

/// A contact on the list: where in the list (x, y), the list's origin and size.
type Touch = (f32, f32, f32, f32, f32, f32);

pub(super) fn wire(ui: &Testbed, keys: &mut Router<Testbed>) {
    let list = Rc::new(RefCell::new(ListState {
        track: space::Track::default(),
        offset: 0.0,
        offset_at_press: 0.0,
        last_y: 0.0,
        focused: None,
    }));
    let list_k = list.clone();
    let weak = ui.as_weak();
    let started_list = Instant::now();
    ui.on_spaces_list(move |kind, x, y, ax, ay, w, h| {
        let Some(ui) = weak.upgrade() else { return };
        let now = started_list.elapsed().as_millis() as u64;
        touch(
            &ui,
            &mut list.borrow_mut(),
            &kind,
            (x, y, ax, ay, w, h),
            now,
        );
    });
    keys.add(&["spaces"], Arrows::Values, move |ui, _, key| match key {
        "next" | "prev" | "enter-down" | "enter-up" | "reveal" => {
            focus_key(ui, &mut list_k.borrow_mut(), key)
        }
        "activate" => true,
        _ => false,
    });
}

/// A contact on the list: the list's own pan, and what the finger is over, in every space.
fn touch(ui: &Testbed, st: &mut ListState, kind: &str, t: Touch, now: u64) {
    let (x, y, ax, ay, w, h) = t;
    let unit = ui.get_unit();
    let viewport = gesture::Frame {
        rtl: ui.get_strip_rtl(),
        ..gesture::Frame::new((w, h), unit)
    }
    .at((ax, ay));
    let content_h = ROWS as f32 * ROW;
    let content = |offset: f32| viewport.content((w, content_h), (0.0, offset));
    match kind {
        "down" => {
            st.track = space::Track::default();
            st.offset_at_press = st.offset;
            st.last_y = y;
        }
        "move" => {
            // The list's own pan: the content follows the finger.
            let dy = y - st.last_y;
            st.offset = (st.offset - dy).clamp(0.0, (content_h - h).max(0.0));
            st.last_y = y;
        }
        _ => {}
    }
    st.track.push((ax + x, ay + y), now);
    ui.set_spaces_offset(st.offset);
    let vp = gesture::FramePath(vec![viewport]);
    let ct_now = gesture::FramePath(vec![viewport, content(st.offset)]);
    let ct_then = gesture::FramePath(vec![viewport, content(st.offset_at_press)]);
    let (Some(p_vp), Some(p_ct), Some(t_vp), Some(t_ct)) = (
        st.track.position(&vp),
        st.track.position(&ct_now),
        st.track.translation(&vp, &vp),
        st.track.translation(&ct_then, &ct_now),
    ) else {
        return;
    };
    let i = ((p_ct.1 / ROW).floor().max(0.0) as usize).min(ROWS - 1);
    let row = row_path(viewport, content(st.offset), i, w, unit);
    let in_row = row.frames().to_inner((ax + x, ay + y));
    let region = st
        .track
        .region(&vp)
        .map(|r| format!("{r:?}"))
        .unwrap_or_default();
    let lines: Vec<SharedString> = vec![
        row.address().into(),
        format!(
            "@viewport {:.0}, {:.0}  moved {:.0}, {:.0}",
            p_vp.0, p_vp.1, t_vp.0, t_vp.1
        )
        .into(),
        format!(
            "@content  {:.0}, {:.0}  moved {:.0}, {:.0}",
            p_ct.0, p_ct.1, t_ct.0, t_ct.1
        )
        .into(),
        format!("in row-{i}  {:.0}, {:.0}", in_row.0, in_row.1).into(),
        format!("region in the viewport: {region}").into(),
    ];
    ui.set_spaces_lines(ModelRc::new(VecModel::from(lines)));
}

/// Row `i`'s path from the activity: the list, its content, the row.
fn row_path(
    viewport: gesture::Frame,
    content: gesture::Frame,
    i: usize,
    w: f32,
    unit: f32,
) -> node::NodePath {
    node::NodePath(vec![
        node::Node::new(
            node::NodeId::new(node::NodeKind::Activity, "ui-primitives"),
            gesture::Frame::new((0.0, 0.0), unit),
        ),
        node::Node::new(
            node::NodeId::new(node::NodeKind::Container, "list"),
            viewport,
        ),
        node::Node::new(
            node::NodeId::new(node::NodeKind::Region, "content"),
            content,
        ),
        node::Node::new(
            node::NodeId::new(node::NodeKind::Element, format!("row-{i}")),
            gesture::Frame::new((w, ROW), unit).at((0.0, i as f32 * ROW)),
        ),
    ])
}

/// The buttons on the list: the focus walks its rows, the list keeps the focused one in view.
fn focus_key(ui: &Testbed, l: &mut ListState, key: &str) -> bool {
    let s = match key {
        "next" => vfocus::step(l.focused, 1, ROWS, vfocus::Ends::Stop),
        "prev" => vfocus::step(l.focused, -1, ROWS, vfocus::Ends::Stop),
        "enter-up" => vfocus::step(None, -1, ROWS, vfocus::Ends::Stop),
        _ => vfocus::step(l.focused.filter(|_| false), 1, ROWS, vfocus::Ends::Stop),
    };
    let (vfocus::Step::Item(i) | vfocus::Step::Wrapped(i) | vfocus::Step::Jumped(i)) = s else {
        return false;
    };
    // Entering or revealing keeps the item it had.
    let i = if matches!(key, "enter-down" | "reveal") {
        l.focused.unwrap_or(i)
    } else {
        i
    };
    l.focused = Some(i);
    ui.set_spaces_focused(i as i32);
    let rows = opendap_layout::extents::Fixed {
        count: ROWS,
        size: 42.0,
    };
    l.offset = vfocus::keep_in_view(&rows, i, l.offset, 168.0);
    ui.set_spaces_offset(l.offset);
    let address = row_path(
        gesture::Frame::new((288.0, 168.0), 1.0),
        gesture::Frame::new((288.0, 840.0), 1.0).at((0.0, -l.offset)),
        i,
        288.0,
        1.0,
    )
    .address();
    ui.set_spaces_lines(ModelRc::new(VecModel::from(vec![
        SharedString::from(address),
        SharedString::from(format!(
            "focused by the buttons: row-{i}, @content y {:.0}",
            i as f32 * 42.0
        )),
    ])));
    true
}
