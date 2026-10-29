//! Every threshold keeps the value it had before thresholds.toml became their source: a change to
//! the data file that moves one of these is a behaviour change, decided on its own, and updates
//! this list with it.

use blackbird_ui_input::thresholds::*;

#[test]
fn every_threshold_keeps_its_value() {
    assert_eq!(SLOP_PX, 10.0);
    assert_eq!(LONG_MS, 500);
    assert_eq!(EDGE_PX, 20.0);
    assert_eq!(STATUS_BAR_PX, 28.0);
    assert_eq!(FLING_PX_S, 50.0);
    assert_eq!(VELOCITY_MS, 100);
    assert_eq!(HOLD_MS, 2000);
    assert_eq!(REPEAT_DELAY_MS, 400);
    assert_eq!(REPEAT_FIRST_MS, 250.0);
    assert_eq!(REPEAT_FACTOR, 0.8);
    assert_eq!(REPEAT_FASTEST_MS, 50.0);
    assert_eq!(DOUBLE_TAP_MS, 300);
    assert_eq!(DOUBLE_PRESS_MS, DOUBLE_TAP_MS);
    assert_eq!(KEY_REPEAT_DELAY_MS, 600);
    assert_eq!(KEY_REPEAT_EVERY_MS, 40);
}

#[test]
fn the_gesture_module_still_names_its_thresholds() {
    use blackbird_ui_input::gesture;
    assert_eq!(gesture::SLOP_PX, SLOP_PX);
    assert_eq!(gesture::LONG_MS, LONG_MS);
    assert_eq!(gesture::EDGE_PX, EDGE_PX);
    assert_eq!(gesture::STATUS_BAR_PX, STATUS_BAR_PX);
    assert_eq!(gesture::FLING_PX_S, FLING_PX_S);
}
