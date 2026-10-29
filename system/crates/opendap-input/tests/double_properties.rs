//! L1 properties of the double press (input events §3.3, T1-062/063), over any presses: it never
//! delays a press (every down is answered at once), a double is exactly a short press released
//! within the window before it, and no double follows a double.

use blackbird_ui_input::double::{DoublePress, Down};
use blackbird_ui_input::thresholds::{DOUBLE_PRESS_MS, REPEAT_DELAY_MS};
use proptest::prelude::*;

proptest! {
    #[test]
    fn a_double_is_a_short_press_just_before(presses in prop::collection::vec((0u64..700, 1u64..900), 1..40)) {
        let mut p = DoublePress::default();
        let (mut now, mut last): (u64, Option<(u64, u64, Down)>) = (0, None);
        for (gap, held) in presses {
            now += gap;
            let down = p.down(now);
            let expected = match last {
                Some((d, u, Down::First)) => u - d < REPEAT_DELAY_MS && now - u <= DOUBLE_PRESS_MS,
                _ => false,
            };
            prop_assert_eq!(down == Down::Second, expected, "down at {} after {:?}", now, last);
            let up = now + held;
            p.up(up);
            last = Some((now, up, down));
            now = up;
        }
    }
}
