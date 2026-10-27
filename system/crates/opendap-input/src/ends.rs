//! What happens past an end of a sequence (input events §3.7): the next item after the last one,
//! the one before the first. Every sequence the buttons, the arrows or a hand walk takes it from
//! here (a navigation level, a list, a shelf, a pager, a value stepped through its options), so a
//! list and a pager and a picker answer the same question the same way.
//!
//! **The default is to stop.** A sequence stops at its ends unless what it holds has no first and
//! no last. The end is information: it says the library, the queue or the settings are all seen;
//! wrapping hides it, and in a long list it throws the focus thousands of items away with one press
//! (iOS's and Android's lists, tvOS's and Android TV's focus engines: none wraps). Past the end the
//! focus goes on to whatever follows the sequence, or stays (a screen's end).
//!
//! **Cycle** where the content is cyclic: hours, minutes, days of the week, months, an angle, a hue
//! (iOS's and Android's time pickers wrap; their lists do not). And where the focus cannot leave: a
//! modal traps it, so going on from its last element can only mean its first. A sequence whose
//! length is not known yet (pages still arriving) cannot wrap to an end it does not know: it stops.
//!
//! A wrap is said apart from a move ([`Step::Wrapped`]), so the view jumps to the far end rather
//! than scrolling through everything between, and a screen reader says it is back at the start.

/// What a sequence does past its ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Ends {
    /// Stays at the end; past it, the focus goes on to what follows the sequence (the default).
    #[default]
    Stop,
    /// Past the last comes the first, before the first the last (cyclic content, a trap).
    Cycle,
}

/// Where one step through a sequence lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// This item.
    To(usize),
    /// This item, at the other end: the sequence wrapped.
    Wrapped(usize),
    /// Past an end of a sequence that stops (or an empty one).
    Past,
}

impl Ends {
    /// One step by `delta` (+1 next, −1 previous, or more) from item `at` in a sequence of `len`
    /// items; `len` is `None` while it is not known (a cyclic sequence then stops).
    pub fn step(self, at: usize, delta: i64, len: Option<usize>) -> Step {
        let j = at as i64 + delta;
        match len {
            Some(0) => Step::Past,
            Some(n) if (0..n as i64).contains(&j) => Step::To(j as usize),
            Some(n) if self == Ends::Cycle => Step::Wrapped(j.rem_euclid(n as i64) as usize),
            None if j >= 0 => Step::To(j as usize),
            _ => Step::Past,
        }
    }

    /// A continuous value moved by `delta` within `min..=max`: clamped at the ends, or, cyclic,
    /// wrapped around the period `max - min` (an angle, a hue, a time of day).
    pub fn value(self, v: f64, delta: f64, min: f64, max: f64) -> f64 {
        let x = v + delta;
        match self {
            Ends::Stop => x.clamp(min, max),
            Ends::Cycle => {
                let period = max - min;
                if period <= 0.0 {
                    min
                } else {
                    min + (x - min).rem_euclid(period)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_stops_and_says_so() {
        let e = Ends::default();
        assert_eq!(e, Ends::Stop, "the default");
        assert_eq!(e.step(3, 1, Some(5)), Step::To(4));
        assert_eq!(e.step(4, 1, Some(5)), Step::Past);
        assert_eq!(e.step(0, -1, Some(5)), Step::Past);
        assert_eq!(e.step(0, 1, Some(0)), Step::Past, "nothing to walk");
    }

    #[test]
    fn cyclic_content_wraps_and_says_it_wrapped() {
        let hours = Ends::Cycle;
        assert_eq!(hours.step(23, 1, Some(24)), Step::Wrapped(0));
        assert_eq!(hours.step(0, -1, Some(24)), Step::Wrapped(23));
        assert_eq!(
            hours.step(10, 1, Some(24)),
            Step::To(11),
            "a move inside is a move"
        );
        assert_eq!(
            hours.step(22, 5, Some(24)),
            Step::Wrapped(3),
            "a page of steps wraps by its remainder"
        );
    }

    #[test]
    fn an_end_not_known_yet_is_not_wrapped_to() {
        assert_eq!(Ends::Cycle.step(0, -1, None), Step::Past);
        assert_eq!(
            Ends::Cycle.step(41, 1, None),
            Step::To(42),
            "forward goes on while pages arrive"
        );
    }

    #[test]
    fn values_clamp_or_go_round() {
        assert_eq!(Ends::Stop.value(350.0, 20.0, 0.0, 360.0), 360.0);
        assert_eq!(
            Ends::Cycle.value(350.0, 20.0, 0.0, 360.0),
            10.0,
            "an angle goes round"
        );
        assert_eq!(Ends::Cycle.value(5.0, -10.0, 0.0, 360.0), 355.0);
        assert_eq!(
            Ends::Cycle.value(1.0, 0.0, 1.0, 1.0),
            1.0,
            "no period, no motion"
        );
    }
}
