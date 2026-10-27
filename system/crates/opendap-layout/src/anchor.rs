//! Scroll anchoring (CSS `overflow-anchor`, Android's and iOS's list behaviour): when items
//! above what the user is looking at change size, appear or go (a cover loads, a track is added
//! at the top), the scroll offset moves by the same amount, so nothing on screen jumps.

use crate::extents::Extents;

/// What the user is looking at: an item by key, and how far into the viewport its start is.
#[derive(Debug, Clone, PartialEq)]
pub struct Anchor<K> {
    pub key: K,
    /// The item's start minus the scroll offset (negative when it starts above the viewport).
    pub inset: f32,
}

/// The anchor at a scroll offset: the first item visible, by its key.
pub fn take<K: Clone>(extents: &impl Extents, keys: &[K], offset: f32) -> Option<Anchor<K>> {
    if extents.is_empty() {
        return None;
    }
    let i = extents.index_at(offset);
    Some(Anchor {
        key: keys.get(i)?.clone(),
        inset: extents.offset(i) - offset,
    })
}

/// The scroll offset that keeps `anchor` where it was after the list changed (new extents and
/// keys); `None` when the anchored item is gone (keep the offset, clamped by the caller).
pub fn restore<K: PartialEq>(
    extents: &impl Extents,
    keys: &[K],
    anchor: &Anchor<K>,
) -> Option<f32> {
    let i = keys.iter().position(|k| *k == anchor.key)?;
    Some((extents.offset(i) - anchor.inset).max(0.0))
}

/// The anchor of a list keyed by index, whose items are not inserted or removed but are measured
/// as they are built (a long list estimated until seen): the focused item when it is on screen,
/// what the person is looking at, else the first item visible. O(log n), no list of keys.
pub fn take_index(
    extents: &impl Extents,
    offset: f32,
    viewport: f32,
    focused: Option<usize>,
) -> Option<Anchor<usize>> {
    if extents.is_empty() {
        return None;
    }
    let on_screen = |i: usize| {
        extents.offset(i) < offset + viewport && extents.offset(i) + extents.size(i) > offset
    };
    let i = focused
        .filter(|f| *f < extents.len() && on_screen(*f))
        .unwrap_or_else(|| extents.index_at(offset));
    Some(Anchor {
        key: i,
        inset: extents.offset(i) - offset,
    })
}

/// The scroll offset that keeps an index anchor where it was after its list was measured, within
/// the list's range (0 to its length past one viewport).
pub fn restore_index(extents: &impl Extents, anchor: &Anchor<usize>, viewport: f32) -> f32 {
    let max = (extents.total() - viewport).max(0.0);
    (extents.offset(anchor.key.min(extents.len().saturating_sub(1))) - anchor.inset).clamp(0.0, max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extents::{Fixed, Measured};

    #[test]
    fn items_added_above_do_not_move_what_is_on_screen() {
        let keys: Vec<u32> = (100..200).collect();
        let e = Fixed {
            count: keys.len(),
            size: 50.0,
        };
        // Looking at item 120, which starts 10 px into the viewport.
        let a = take(&e, &keys, 50.0 * 20.0 - 10.0).unwrap();
        assert_eq!((a.key, a.inset), (119, -40.0));
        // Three tracks inserted at the top.
        let keys2: Vec<u32> = [1, 2, 3].into_iter().chain(100..200).collect();
        let e2 = Fixed {
            count: keys2.len(),
            size: 50.0,
        };
        let o = restore(&e2, &keys2, &a).unwrap();
        assert_eq!(o, 50.0 * 22.0 + 40.0, "moved by the 150 px inserted");
        assert_eq!(take(&e2, &keys2, o).unwrap().key, 119);
    }

    #[test]
    fn a_cover_that_grows_above_moves_the_offset_by_its_growth() {
        let keys: Vec<u32> = (0..50).collect();
        let mut e = Measured::new(50, 60.0);
        let a = take(&e, &keys, 600.0).unwrap();
        e.measure(2, 160.0);
        assert_eq!(restore(&e, &keys, &a), Some(700.0));
        assert_eq!(restore(&e, &[7u32, 8], &a), None, "gone");
    }

    #[test]
    fn a_jump_to_the_end_of_an_estimated_list_stays_on_screen_once_measured() {
        // 100 000 rows estimated at 48 px; every seventh is 72 px once seen.
        let mut e = Measured::new(100_000, 48.0);
        let viewport = 260.0;
        let last = 99_999;
        // The jump: the last row at the viewport's bottom, by the estimates.
        let offset = e.offset(last) + e.size(last) - viewport;
        let a = take_index(&e, offset, viewport, Some(last)).unwrap();
        assert_eq!(a.key, last, "the focused row is the anchor");
        // The rows on screen are built and measured: some are taller.
        for i in 99_990..100_000 {
            e.measure(i, if i % 7 == 0 { 72.0 } else { 48.0 });
        }
        let o = restore_index(&e, &a, viewport);
        let (top, bottom) = (e.offset(last) - o, e.offset(last) + e.size(last) - o);
        assert!(
            top >= 0.0 && bottom <= viewport + 0.01,
            "still on screen: {top}..{bottom}"
        );
    }

    #[test]
    fn without_a_focus_on_screen_the_first_visible_row_anchors() {
        let e = Fixed {
            count: 1000,
            size: 50.0,
        };
        let a = take_index(&e, 510.0, 200.0, Some(900)).unwrap();
        assert_eq!((a.key, a.inset), (10, -10.0));
    }
}
