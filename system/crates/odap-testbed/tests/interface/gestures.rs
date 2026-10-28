//! The Gestures screen's pad.

use super::*;

pub fn pad(window: &Testbed, run: &mut Run) {
    let ui = window.clone_strong();
    // The gesture pad (opendap-input's gesture, as the Gestures screen shows it): sequences of
    // contacts at their real pace (the pad reads the wall clock; the long press is a timer).
    open(&ui, "Gestures");
    let pad = |kind: &str, x: f32, y: f32| ui.invoke_pad(kind.into(), x, y, 288.0, 160.0);
    let wait = |ms: u64| {
        std::thread::sleep(Duration::from_millis(ms));
        tick(ms);
    };
    let said = |ui: &Testbed| ui.get_gesture().to_string();
    pad("down", 144.0, 80.0);
    wait(60);
    pad("up", 144.0, 80.0);
    run.check("pad: a tap", said(&ui) == "tap", || said(&ui));
    pad("down", 144.0, 80.0);
    wait(650);
    run.check(
        "pad: a still hold is a long press, while down",
        said(&ui).starts_with("long press"),
        || said(&ui),
    );
    pad("up", 144.0, 80.0);
    for i in 1..=8 {
        if i == 1 {
            pad("down", 220.0, 80.0);
        }
        wait(12);
        pad("move", 220.0 - i as f32 * 18.0, 80.0);
    }
    pad("up", 76.0, 80.0);
    run.check(
        "pad: a fast move left is a swipe left",
        said(&ui).starts_with("swipe left"),
        || said(&ui),
    );
    pad("down", 60.0, 80.0);
    for i in 1..=10 {
        wait(60);
        pad("move", 60.0 + i as f32 * 12.0, 80.0);
    }
    wait(250);
    pad("up", 180.0, 80.0);
    run.check(
        "pad: a slow move that stops is a pan, no swipe",
        said(&ui).starts_with("pan end"),
        || said(&ui),
    );
    pad("down", 144.0, 40.0);
    wait(20);
    pad("up", 144.0, 140.0);
    run.check(
        "pad: a lift far from the press with no move between is a swipe",
        said(&ui).starts_with("swipe down"),
        || said(&ui),
    );
    pad("down", 2.0, 80.0);
    for i in 1..=6 {
        wait(12);
        pad("move", 2.0 + i as f32 * 20.0, 80.0);
    }
    pad("up", 122.0, 80.0);
    run.check(
        "pad: a swipe from the left edge says its edge",
        said(&ui).contains("edge") || said(&ui).contains("left"),
        || said(&ui),
    );
}
