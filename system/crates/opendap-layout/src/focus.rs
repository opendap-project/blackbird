//! The focus in a virtual list (design system §8.4 with ui-model §7.1): the focus belongs to an
//! item, by its key, never to the slot that shows it, since slots are rebound as the window
//! moves. The buttons move it item by item across the whole list (not just what is built); the
//! list scrolls just enough to keep the focused item in view (the window follows); the ring is
//! drawn on whichever slot shows that item now; and at either end the focus leaves the list for
//! the next element.

use crate::extents::Extents;

pub use opendap_input::ends::Ends;

/// Where the focus went after a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// To this item of the list.
    Item(usize),
    /// Round to this item, at the other end of a list that cycles: the view jumps there (no scroll
    /// through everything between) and a screen reader says the list started again.
    Wrapped(usize),
    /// Straight to this item, far from where it was ([`jump`]): the view goes there at once, as
    /// for a wrap, never scrolling through everything between.
    Jumped(usize),
    /// Past an end: the focus leaves the list (the next or previous element takes it).
    Leave,
}

/// Where a jump goes: the list's start, its end, or one item by its index (a keyboard's Home and
/// End, as in ARIA's lists; an index of letters; a scrubber; a test that needs the middle of a
/// hundred thousand rows without walking there).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Jump {
    Start,
    End,
    Index(usize),
}

/// The focus straight to `to` in a list of `count` items: an index past the end lands on the last
/// item; an empty list has nothing to land on.
pub fn jump(to: Jump, count: usize) -> Step {
    if count == 0 {
        return Step::Leave;
    }
    Step::Jumped(match to {
        Jump::Start => 0,
        Jump::End => count - 1,
        Jump::Index(i) => i.min(count - 1),
    })
}

/// One step from `focused` (`None`: the focus enters the list) by `delta` (+1 next, −1
/// previous) in a list of `count` items, past its ends as `ends` says: a list stops (the
/// default, [`Ends`]); one over cyclic content (hours, days of the week) cycles.
pub fn step(focused: Option<usize>, delta: i32, count: usize, ends: Ends) -> Step {
    if count == 0 {
        return Step::Leave;
    }
    match focused {
        // Entering: from above, the first; from below, the last.
        None => Step::Item(if delta >= 0 { 0 } else { count - 1 }),
        Some(i) => match ends.step(i, i64::from(delta), Some(count)) {
            opendap_input::ends::Step::To(j) => Step::Item(j),
            opendap_input::ends::Step::Wrapped(j) => Step::Wrapped(j),
            opendap_input::ends::Step::Past => Step::Leave,
        },
    }
}

/// The scroll offset that shows item `i` whole in a viewport of `viewport` at `offset`, moving
/// as little as possible (FocusReveal's rule); unchanged when it is already in view.
pub fn keep_in_view(extents: &impl Extents, i: usize, offset: f32, viewport: f32) -> f32 {
    let (top, bottom) = (extents.offset(i), extents.offset(i) + extents.size(i));
    let max = (extents.total() - viewport).max(0.0);
    let o = if top < offset {
        top
    } else if bottom > offset + viewport {
        bottom - viewport
    } else {
        offset
    };
    o.clamp(0.0, max)
}

/// The focused item, kept by key across changes to the list (items inserted or removed above):
/// its new index, or the nearest item where it was when it is gone.
pub fn follow<K: PartialEq>(keys: &[K], focused: &K, old_index: usize) -> Option<usize> {
    if keys.is_empty() {
        return None;
    }
    Some(
        keys.iter()
            .position(|k| k == focused)
            .unwrap_or(old_index.min(keys.len() - 1)),
    )
}

/// Action on an item whose data has not arrived (a placeholder): it waits for its data, with the
/// loading shown, and is taken when the data is here; moving the focus away gives it up (input
/// events §3.7).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Activation {
    pending: Option<usize>,
}

impl Activation {
    /// Action on item `i`: taken now (`Some(i)`) when its data is ready, else waiting.
    pub fn press(&mut self, i: usize, ready: bool) -> Option<usize> {
        if ready {
            self.pending = None;
            Some(i)
        } else {
            self.pending = Some(i);
            None
        }
    }

    /// The focus moved to `to`: a wait for another item is given up.
    pub fn moved(&mut self, to: usize) {
        if self.pending != Some(to) {
            self.pending = None;
        }
    }

    /// Data arrived: the waiting item, taken now that it is ready.
    pub fn arrived(&mut self, ready: impl Fn(usize) -> bool) -> Option<usize> {
        let i = self.pending.filter(|i| ready(*i))?;
        self.pending = None;
        Some(i)
    }

    pub fn waiting(&self) -> Option<usize> {
        self.pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extents::Fixed;
    use crate::recycle::Pool;
    use crate::window::{Overscan, window};

    #[test]
    fn the_buttons_walk_the_whole_list_and_leave_at_its_ends() {
        assert_eq!(step(None, 1, 100_000, Ends::Stop), Step::Item(0));
        assert_eq!(
            step(None, -1, 100_000, Ends::Stop),
            Step::Item(99_999),
            "entered from below"
        );
        assert_eq!(step(Some(41), 1, 100_000, Ends::Stop), Step::Item(42));
        assert_eq!(step(Some(0), -1, 100_000, Ends::Stop), Step::Leave);
        assert_eq!(step(Some(99_999), 1, 100_000, Ends::Stop), Step::Leave);
        assert_eq!(step(None, 1, 0, Ends::Stop), Step::Leave);
    }

    #[test]
    fn a_jump_lands_on_the_start_the_end_or_an_item() {
        assert_eq!(jump(Jump::Start, 100_000), Step::Jumped(0));
        assert_eq!(jump(Jump::End, 100_000), Step::Jumped(99_999));
        assert_eq!(jump(Jump::Index(50_000), 100_000), Step::Jumped(50_000));
        assert_eq!(
            jump(Jump::Index(200_000), 100_000),
            Step::Jumped(99_999),
            "past the end: the last"
        );
        assert_eq!(jump(Jump::End, 0), Step::Leave, "nothing to land on");
        // From the end, the list stops as ever.
        assert_eq!(step(Some(99_999), 1, 100_000, Ends::Stop), Step::Leave);
    }

    #[test]
    fn a_list_of_cyclic_content_goes_round_and_says_so() {
        // The days of the week: after Sunday, Monday.
        assert_eq!(step(Some(6), 1, 7, Ends::Cycle), Step::Wrapped(0));
        assert_eq!(step(Some(0), -1, 7, Ends::Cycle), Step::Wrapped(6));
        assert_eq!(step(Some(2), 1, 7, Ends::Cycle), Step::Item(3));
        assert_eq!(
            step(None, -1, 7, Ends::Cycle),
            Step::Item(6),
            "entering is entering"
        );
        assert_eq!(
            step(Some(0), 1, 0, Ends::Cycle),
            Step::Leave,
            "nothing to go round"
        );
    }

    #[test]
    fn the_list_scrolls_just_enough_and_the_ring_follows_the_item_not_the_slot() {
        let e = Fixed {
            count: 100_000,
            size: 48.0,
        };
        let vp = 260.0;
        // Moving down past the bottom: scrolled so the item's bottom meets the viewport's.
        assert_eq!(keep_in_view(&e, 5, 0.0, vp), 48.0 * 6.0 - vp);
        assert_eq!(keep_in_view(&e, 2, 0.0, vp), 0.0, "in view: no move");
        assert_eq!(keep_in_view(&e, 3, 200.0, vp), 144.0, "above: its top");
        // Walk 300 items down: the window follows, slots recycle, the focused item always has one.
        let mut pool: Pool<usize> = Pool::new();
        let mut offset = 0.0;
        let o = Overscan {
            around: 96.0,
            lead_s: 0.0,
            max_ahead: 0.0,
        };
        for i in 0..300 {
            offset = keep_in_view(&e, i, offset, vp);
            let keys: Vec<usize> = window(&e, offset, vp, 0.0, o).collect();
            pool.update(&keys);
            assert!(
                pool.slot_of(&i).is_some(),
                "item {i} is built and has a slot"
            );
        }
        assert!(pool.slots() < 16, "and the pool stays small");
    }

    #[test]
    fn the_focus_follows_its_item_when_the_list_changes() {
        let keys = vec![10, 11, 12, 13];
        assert_eq!(
            follow(&[1, 2, 10, 11, 12, 13], &12, 2),
            Some(4),
            "items inserted above"
        );
        assert_eq!(
            follow(&[10, 11, 13], &12, 2),
            Some(2),
            "gone: the nearest where it was"
        );
        assert_eq!(follow(&keys, &99, 9), Some(3));
        assert_eq!(follow::<u32>(&[], &1, 0), None);
    }

    #[test]
    fn action_on_a_placeholder_waits_for_its_data() {
        let mut a = Activation::default();
        assert_eq!(a.press(3, true), Some(3), "ready: taken now");
        assert_eq!(a.press(70_000, false), None);
        assert_eq!(a.waiting(), Some(70_000));
        assert_eq!(a.arrived(|_| false), None, "still loading");
        assert_eq!(
            a.arrived(|i| i == 70_000),
            Some(70_000),
            "taken when it arrives"
        );
        assert_eq!(a.waiting(), None);
        a.press(5, false);
        a.moved(6);
        assert_eq!(a.arrived(|_| true), None, "moving away gives it up");
        a.press(5, false);
        a.moved(5);
        assert_eq!(a.waiting(), Some(5), "staying keeps it");
    }
}
