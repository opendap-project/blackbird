//! The Virtual screen: a hundred thousand rows, jumps, slots, placeholders, fling, sticky.

use super::*;

pub fn jump(window: &Testbed, run: &mut Run) {
    let ui = window.clone_strong();
    // Jump (opendap-layout's focus::jump): Home and End take the long list to its ends at once,
    // and past its end it stops as ever.
    open(&ui, "Virtual");
    key(&ui, Key::Tab);
    for _ in 0..12 {
        key(&ui, Key::Tab);
        if ui.get_vtext().contains("focus: row") {
            break;
        }
    }
    key(&ui, Key::End);
    let vt = ui.get_vtext().to_string();
    run.check(
        "End: the last of 100 000 rows",
        vt.contains("focus: row 99999"),
        || vt.clone(),
    );
    // And on screen: the rows near the end are only estimated until built, so measuring them must
    // not leave the focused one below the view.
    let shown = vt
        .split(" on screen")
        .next()
        .and_then(|h| h.rsplit(", ").next())
        .and_then(|r| r.split_once('-'))
        .and_then(|(a, b)| Some((a.parse::<usize>().ok()?, b.parse::<usize>().ok()?)));
    run.check(
        "End: the last row on screen",
        shown.is_some_and(|(a, b)| a <= 99_999 && 99_999 <= b),
        || format!("{shown:?}"),
    );
    let reveal = focus(&ui);
    key(&ui, Key::Tab);
    let vt2 = ui.get_vtext().to_string();
    run.check(
        "past the end of the list: no wrap, the focus goes on past it",
        vt2.contains("focus: row 99999") && focus(&ui) != reveal,
        || format!("{vt2} at {:?}", focus(&ui)),
    );
    back_key(&ui);
    key(&ui, Key::Home);
    let vt = ui.get_vtext().to_string();
    run.check("Home: the first row", vt.contains("focus: row 0,"), || {
        vt.clone()
    });
    ui.invoke_vkey("jump:50000".into());
    tick(16);
    let vt = ui.get_vtext().to_string();
    run.check(
        "a jump to the middle",
        vt.contains("focus: row 50000"),
        || vt.clone(),
    );
}

pub fn recycled_slots(window: &Testbed, run: &mut Run) {
    let ui = window.clone_strong();
    // Recycled slots: after jumps far apart, every row drawn in the viewport is one of the rows on
    // screen (a freed slot keeps nothing visible from where it was).
    key(&ui, Key::End);
    let vt = ui.get_vtext().to_string();
    let shown = vt
        .split(" on screen")
        .next()
        .and_then(|h| h.rsplit(", ").next())
        .and_then(|r| r.split_once('-'))
        .and_then(|(a, b)| Some((a.parse::<i32>().ok()?, b.parse::<i32>().ok()?)));
    let rows = ui.get_vrows();
    let stray: Vec<i32> = (0..rows.row_count())
        .filter_map(|i| rows.row_data(i))
        .filter(|r| r.y + r.h > 0.0 && r.y < 260.0)
        .map(|r| r.index)
        .filter(|i| shown.is_none_or(|(a, b)| *i < a - 2 || *i > b + 2))
        .collect();
    run.check(
        "after jumps, only the rows on screen are drawn there",
        stray.is_empty(),
        || format!("{stray:?} drawn among {shown:?}"),
    );
    // Stronger: a slot the window does not use is hidden, whatever the pool's size.
    let built: Vec<i32> = (0..rows.row_count())
        .filter_map(|i| rows.row_data(i))
        .filter(|r| r.y > -1000.0)
        .map(|r| r.index)
        .collect();
    let far = built
        .iter()
        .filter(|i| shown.is_none_or(|(a, b)| **i < a - 30 || **i > b + 30))
        .count();
    run.check("a slot the window does not use is hidden", far == 0, || {
        format!("{far} visible slots far from {shown:?}: {built:?}")
    });
}

pub fn placeholder(window: &Testbed, run: &mut Run) {
    let ui = window.clone_strong();
    // virt-placeholder: Action on a row whose page has not arrived waits, with the wait said, and
    // takes the row once its data is there; moving away first cancels it. The jump is the debug
    // jump's own path (the long list's `jump:N`).
    open(&ui, "Virtual");
    key(&ui, Key::Tab);
    ui.invoke_vkey("jump:50000".into());
    tick(16);
    key(&ui, Key::Return);
    let waiting = ui.get_vtext().to_string();
    tick(300);
    let arrived = ui.get_vtext().to_string();
    run.check(
        "virt-placeholder: Action waits for the row's data",
        waiting.contains("row 50000 waits for its data"),
        || waiting.clone(),
    );
    run.check(
        "virt-placeholder: then takes it",
        arrived.contains("activated row 50000"),
        || arrived.clone(),
    );
    ui.invoke_vkey("jump:70000".into());
    tick(16);
    key(&ui, Key::Return);
    key(&ui, Key::Tab); // away before the data
    tick(300);
    let moved = ui.get_vtext().to_string();
    run.check(
        "virt-placeholder: moving away cancels it",
        !moved.contains("activated row 70000") && !moved.contains("waits for its data"),
        || moved.clone(),
    );
}

pub fn fling(window: &Testbed, run: &mut Run) {
    let ui = window.clone_strong();
    // virt-fling: a fast throw on the long list goes on after the finger leaves, slowing as
    // opendap-motion's decay says, and stops (its projected end shown). The list measures time on
    // the real clock, so the throw runs in real time here.
    open(&ui, "Virtual");
    let first_row = |ui: &Testbed| -> usize {
        let t = ui.get_vtext().to_string();
        t.split(", ")
            .nth(1)
            .and_then(|r| r.split('-').next())
            .and_then(|n| n.trim().parse().ok())
            .unwrap_or(0)
    };
    let real = |ms: u64| {
        std::thread::sleep(Duration::from_millis(ms));
        tick(ms);
    };
    ui.invoke_vpan("down".into(), 230.0, 260.0);
    for i in 1..=6 {
        real(16);
        ui.invoke_vpan("move".into(), 230.0 - i as f32 * 30.0, 260.0);
    }
    real(16);
    ui.invoke_vpan("up".into(), 230.0 - 7.0 * 30.0, 260.0);
    let at_release = first_row(&ui);
    let projected = ui.get_vtext().to_string().contains("fling to ");
    real(300);
    let moving = first_row(&ui);
    for _ in 0..200 {
        real(16);
        if ui.get_vtext().to_string().contains(", 0 px/s") {
            break;
        }
    }
    let stopped = ui.get_vtext().to_string();
    run.check("virt-fling: a throw is projected", projected, || {
        ui.get_vtext().to_string()
    });
    run.check(
        "virt-fling: the list goes on after the finger",
        moving > at_release,
        || format!("row {at_release} at release, {moving} 300 ms later"),
    );
    run.check(
        "virt-fling: and stops",
        stopped.contains(", 0 px/s") && first_row(&ui) >= moving,
        || stopped.clone(),
    );
}

pub fn sticky_push(window: &Testbed, run: &mut Run) {
    let ui = window.clone_strong();
    // virt-sticky-push: the pinned header of a section is pushed out by the next one's, then the
    // next one's is pinned.
    ui.invoke_vkey("jump:995".into());
    tick(16);
    let (mut pushed, mut pinned_next) = (false, false);
    // Sampled frame by frame: the push lasts while the scroll crosses the header's height.
    'walk: for _ in 0..40 {
        key(&ui, Key::Tab);
        for _ in 0..15 {
            tick(16);
            let (y, text) = (ui.get_vband_y(), ui.get_vband_text().to_string());
            pushed |= text == "Rows 0–999" && y < -0.5;
            pinned_next |= text == "Rows 1000–1999" && y.abs() < 0.5;
            if pinned_next {
                break 'walk;
            }
        }
    }
    run.check(
        "virt-sticky-push: the next section pushes the header out",
        pushed,
        || format!("y {} {:?}", ui.get_vband_y(), ui.get_vband_text()),
    );
    run.check(
        "virt-sticky-push: then its own header is pinned",
        pinned_next,
        || format!("y {} {:?}", ui.get_vband_y(), ui.get_vband_text()),
    );
}
