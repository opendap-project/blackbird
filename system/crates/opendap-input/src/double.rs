//! A control's double press (input events §3.3), where a context gives one: the volume buttons
//! while they are the volume (T1-062). It never delays the single press (T1-063): the first press
//! acts when it goes down, and a second down within [`DOUBLE_PRESS_MS`] of the first's release
//! is the double, so the caller takes the first press's effect back and does the double's. A
//! first press held into its repeat is no candidate, and a double's own press starts no other.

use crate::thresholds::{DOUBLE_PRESS_MS, REPEAT_DELAY_MS};

/// What a control's down is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Down {
    /// A press of its own: it acts now.
    First,
    /// The second of a double press: the first one's effect is taken back.
    Second,
}

/// One control's presses, fed its edges in milliseconds. A pure state machine.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DoublePress {
    down_at: Option<u64>,
    second: bool,
    /// When a short first press was released: the next down within the window is the double.
    armed_at: Option<u64>,
}

impl DoublePress {
    pub fn down(&mut self, now_ms: u64) -> Down {
        let double = self
            .armed_at
            .is_some_and(|up| now_ms.saturating_sub(up) <= DOUBLE_PRESS_MS);
        self.armed_at = None;
        self.down_at = Some(now_ms);
        self.second = double;
        if double { Down::Second } else { Down::First }
    }

    pub fn up(&mut self, now_ms: u64) {
        let short = self
            .down_at
            .take()
            .is_some_and(|d| now_ms.saturating_sub(d) < REPEAT_DELAY_MS);
        self.armed_at = (short && !self.second).then_some(now_ms);
    }

    /// The system took the control (a combination, the display going off): no double spans it.
    pub fn reset(&mut self) {
        *self = DoublePress::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_quick_presses_are_a_double() {
        let mut p = DoublePress::default();
        assert_eq!(p.down(0), Down::First);
        p.up(80);
        assert_eq!(p.down(80 + DOUBLE_PRESS_MS), Down::Second);
    }

    #[test]
    fn a_slow_second_press_is_its_own() {
        let mut p = DoublePress::default();
        p.down(0);
        p.up(80);
        assert_eq!(p.down(81 + DOUBLE_PRESS_MS), Down::First);
    }

    #[test]
    fn a_held_first_press_is_no_candidate() {
        let mut p = DoublePress::default();
        p.down(0);
        p.up(REPEAT_DELAY_MS + 100); // it repeated
        assert_eq!(p.down(REPEAT_DELAY_MS + 150), Down::First);
    }

    #[test]
    fn a_third_quick_press_starts_again() {
        let mut p = DoublePress::default();
        p.down(0);
        p.up(60);
        assert_eq!(p.down(120), Down::Second);
        p.up(180);
        assert_eq!(p.down(240), Down::First);
        p.up(300);
        assert_eq!(p.down(360), Down::Second);
    }

    #[test]
    fn the_system_taking_over_ends_a_candidate() {
        let mut p = DoublePress::default();
        p.down(0);
        p.up(60);
        p.reset();
        assert_eq!(p.down(120), Down::First);
    }
}
