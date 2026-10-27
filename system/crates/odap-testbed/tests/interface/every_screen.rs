//! Every screen: the focus on entry, its ends, its geometry left to right and right to left.

use super::*;

pub fn entry(window: &Testbed, run: &mut Run) {
    let ui = window.clone_strong();
    // nav-entry-20, nav-hidden-action: every screen opens with the focus somewhere and its ring
    // hidden; the first key only shows where it is.
    for s in SCREENS {
        open(&ui, s);
        let at = focus(&ui);
        key(&ui, Key::Tab);
        run.check(&format!("entry {s}"), ring(&ui), || {
            "the first key shows the ring".into()
        });
        let serial = ui.global::<FocusReveal>().get_serial();
        run.check(&format!("entry {s}"), serial > 0, || {
            "a control reported the focus".into()
        });
        let _ = at;
    }
}

pub fn ends(window: &Testbed, run: &mut Run) {
    let ui = window.clone_strong();
    let height = 480.0;
    // nav-ends: Volume past either end of a screen stays there, however fast the keys come.
    for s in SCREENS {
        open(&ui, s);
        key(&ui, Key::Tab);
        let forward = walk(&ui, |ui| key(ui, Key::Tab));
        let end = focus(&ui);
        for _ in 0..4 {
            key(&ui, Key::Tab);
        }
        run.check(&format!("end {s}"), focus(&ui) == end, || {
            format!("past the end, moved from {end:?} to {:?}", focus(&ui))
        });
        let back = walk(&ui, back_key);
        let start = focus(&ui);
        for _ in 0..4 {
            back_key(&ui);
        }
        run.check(&format!("start {s}"), focus(&ui) == start, || {
            format!("past the start, moved from {start:?} to {:?}", focus(&ui))
        });
        // Revealed: every place the focus went is on screen (the scroll area brings it in).
        let off: Vec<_> = forward
            .iter()
            .chain(&back)
            .filter(|(y, h)| *y < 0.0 || y + h > height + 0.5)
            .collect();
        run.check(&format!("reveal {s}"), off.is_empty(), || {
            format!("focus left off screen at {off:?}")
        });
    }
}

pub fn left_to_right_as_kept(window: &Testbed, run: &mut Run) {
    let ui = window.clone_strong();
    // layout-ltr-reg: left to right, each screen's geometry (what a screen reader sees, and where)
    // is what it was: a snapshot in tests/snapshots, reviewed and updated on purpose
    // (ODAP_UPDATE_SNAPSHOTS=1) when a change means to move things.
    {
        let mut text = String::new();
        for s in SCREENS {
            open(&ui, s);
            text.push_str(&format!("# {s}\n"));
            for (l, x, y, w, h) in named(&ui) {
                text.push_str(&format!("{l:?} {x:.0} {y:.0} {w:.0} {h:.0}\n"));
            }
        }
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/snapshots/ltr-geometry.txt");
        if std::env::var_os("ODAP_UPDATE_SNAPSHOTS").is_some() || !path.exists() {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, &text).unwrap();
        }
        let kept = std::fs::read_to_string(&path).unwrap_or_default();
        let moved: Vec<String> = kept
            .lines()
            .zip(text.lines())
            .filter(|(a, b)| a != b)
            .map(|(a, b)| format!("{a}  →  {b}"))
            .take(8)
            .collect();
        run.check("left to right as kept", kept == text, || {
            if moved.is_empty() {
                format!(
                    "{} lines kept, {} now",
                    kept.lines().count(),
                    text.lines().count()
                )
            } else {
                moved.join("; ")
            }
        });
    }
}

pub fn right_to_left_mirrors(window: &Testbed, run: &mut Run) {
    let ui = window.clone_strong();
    // layout-dirrow, layout-wrap-rtl: right to left mirrors every screen. Each
    // element a screen reader sees lands where the left-to-right one is, mirrored across the width,
    // at the same height and size; the design system's exceptions (physical order: transport, the
    // seek bar, the volume) are the only ones that do not.
    for s in SCREENS {
        open(&ui, s);
        let ltr = named(&ui);
        ui.global::<Layout>().set_rtl(true);
        tick(50);
        let rtl = named(&ui);
        ui.global::<Layout>().set_rtl(false);
        tick(50);
        let mut off = Vec::new();
        // Physical order, never mirrored (design system §9.5): a label beside a physical control,
        // the seek bar's times.
        let physical = |l: &str| {
            l.starts_with("Action: ")
                || l.contains(':')
                    && l.chars().any(|c| c.is_ascii_digit())
                    && l.chars().count() <= 6
        };
        for (a, b) in ltr.iter().zip(&rtl) {
            if physical(&a.0) {
                continue;
            }
            let mirrored = 320.0 - a.1 - a.3;
            if a.0 != b.0
                || (b.1 - mirrored).abs() > 1.5
                || (b.2 - a.2).abs() > 1.5
                || (b.3 - a.3).abs() > 1.5
            {
                off.push(format!(
                    "{} at {:.0} (ltr {:.0}, mirrored {:.0})",
                    a.0, b.1, a.1, mirrored
                ));
            }
        }
        if ltr.len() != rtl.len() {
            off.push(format!(
                "{} elements left to right, {} right to left",
                ltr.len(),
                rtl.len()
            ));
        }
        run.check(&format!("mirrored {s}"), off.is_empty(), || off.join("; "));
    }
}

pub fn ends_right_to_left(window: &Testbed, run: &mut Run) {
    let ui = window.clone_strong();
    // nav-ends right to left: the ends hold whichever way the screen reads.
    ui.global::<Layout>().set_rtl(true);
    for s in ["Controls", "Layers", "Navigation"] {
        open(&ui, s);
        key(&ui, Key::Tab);
        walk(&ui, |ui| key(ui, Key::Tab));
        let end = focus(&ui);
        for _ in 0..4 {
            key(&ui, Key::Tab);
        }
        run.check(&format!("end {s} right to left"), focus(&ui) == end, || {
            format!("{end:?} → {:?}", focus(&ui))
        });
    }
    ui.global::<Layout>().set_rtl(false);
}

pub fn narrow(window: &Testbed, run: &mut Run) {
    let ui = window.clone_strong();
    // layout-narrow: on a narrower display (another faceplate, SYS-040) no named element is cut
    // by the window's edges: each is wholly inside, or wholly outside (a page of a pager, content
    // scrolled away). The T1's own width is checked by every scenario above.
    // A list that scrolls across (a shelf) clips what peeks in at its edges, as a hint that it
    // scrolls: what lies in it is its own; the list itself must fit.
    let cut_at = |ui: &Testbed, width: f32| -> Vec<(String, f32, f32, f32, f32)> {
        named(ui)
            .into_iter()
            .filter(|&(_, x, _, w, _)| {
                !(x >= -0.5 && x + w <= width + 0.5) && !(x + w <= 0.5 || x >= width - 0.5)
            })
            .collect()
    };
    let lists = |ui: &Testbed| -> Vec<(f32, f32, f32, f32)> {
        ElementQuery::from_root(ui)
            .match_predicate(|e| {
                e.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::List)
            })
            .find_all()
            .into_iter()
            .map(|e| {
                (
                    e.absolute_position().x,
                    e.absolute_position().y,
                    e.size().width,
                    e.size().height,
                )
            })
            .collect()
    };
    for width in [280.0f32, 240.0] {
        ui.window().set_size(slint::LogicalSize::new(width, 480.0));
        tick(16);
        let mut cut = Vec::new();
        for s in SCREENS {
            open(&ui, s);
            tick(300);
            let here = lists(&ui);
            for (name, x, y, w, h) in cut_at(&ui, width) {
                // In a list: within its height and overlapping it across (an item that peeks in).
                let cy = y + h / 2.0;
                let in_a_list = here.iter().any(|&(lx, ly, lw, lh)| {
                    cy >= ly && cy <= ly + lh && x < lx + lw && x + w > lx && lx + lw <= width + 0.5
                });
                if !in_a_list {
                    cut.push(format!("{s}: {name:?} at {x:.0}..{:.0}", x + w));
                }
            }
        }
        run.check(
            &format!("layout-narrow: nothing cut at {width} px"),
            cut.is_empty(),
            || cut.join("; "),
        );
    }
    ui.window().set_size(slint::LogicalSize::new(320.0, 480.0));
    tick(16);
}
