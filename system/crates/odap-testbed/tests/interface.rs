//! The UI primitives activity driven headless (ADR 0014, level L2): its real screens, built from
//! Blackbird UI's real components and wired by the activity's own logic, on Slint's testing
//! backend with mock time. Keys go in as the composer delivers them (Volume − as Tab, Volume + as
//! Shift+Tab, Action as Return, back as Escape); what comes out is what the person would see: where
//! the focus is (the focus report every control makes, `FocusReveal`), whether its ring shows
//! (`FocusVisible`), the screen's state.
//!
//! Slint's testing backend takes one window per process, so the scenarios run in one test, each
//! from a fresh screen, and every failure is reported together.

use std::time::Duration;

use i_slint_backend_testing::{ElementHandle, ElementQuery};
use odap_testbed::{FocusReveal, FocusVisible, Layout, Testbed, wire};
use slint::platform::{Key, WindowEvent};
use slint::{ComponentHandle, Model, SharedString};

/// Every screen of the activity, as its list names them.
const SCREENS: &[&str] = &[
    "Calibration",
    "Targets",
    "Gestures",
    "Controls",
    "Actions",
    "Navigation",
    "Selection",
    "Status",
    "States",
    "Motion",
    "Opacity",
    "Layers",
    "Layout",
    "Spaces",
    "Turns",
    "Arena",
    "Virtual",
    "Media",
    "Exceptions",
    "Keyboard",
    "Transitions",
];

fn tick(ms: u64) {
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(ms));
    slint::platform::update_timers_and_animations();
}

fn press(ui: &Testbed, text: SharedString) {
    ui.window()
        .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    ui.window()
        .dispatch_event(WindowEvent::KeyReleased { text });
}

/// A key, then 16 ms (a frame): faster than any button repeats.
fn key(ui: &Testbed, k: Key) {
    press(ui, k.into());
    tick(16);
}

/// Volume +: Shift+Tab, as the composer sends it.
fn back_key(ui: &Testbed) {
    let shift: SharedString = Key::Shift.into();
    ui.window().dispatch_event(WindowEvent::KeyPressed {
        text: shift.clone(),
    });
    press(ui, Key::Tab.into());
    ui.window()
        .dispatch_event(WindowEvent::KeyReleased { text: shift });
    tick(16);
}

fn typed(ui: &Testbed, s: &str) {
    for c in s.chars() {
        press(ui, c.to_string().into());
    }
    tick(16);
}

/// Where the focus is: the focused element's top and height, in window coordinates.
fn focus(ui: &Testbed) -> (f32, f32) {
    let r = ui.global::<FocusReveal>();
    (r.get_y(), r.get_h())
}

/// What the field named `label` holds, as a screen reader reads it (its caption has the same name:
/// the field is the one with the text input role).
fn field_text(ui: &Testbed, label: &str) -> Option<String> {
    ElementHandle::find_by_accessible_label(ui, label)
        .find(|e| e.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::TextInput))
        .map(|e| e.accessible_value().unwrap_or_default().to_string())
}

fn ring(ui: &Testbed) -> bool {
    ui.global::<FocusVisible>().get_on()
}

/// Opens a screen as its list does, with the focus hidden as after a touch.
fn open(ui: &Testbed, screen: &str) {
    ui.set_screen("".into());
    tick(16);
    ui.global::<FocusVisible>().set_on(false);
    ui.set_screen(screen.into());
    tick(50);
}

/// Walks with `step` until the focus stops moving: the end it stays at, and the places it passed.
fn walk(ui: &Testbed, step: fn(&Testbed)) -> Vec<(f32, f32)> {
    let mut seen = vec![focus(ui)];
    for _ in 0..80 {
        let serial = ui.global::<FocusReveal>().get_serial();
        step(ui);
        if ui.global::<FocusReveal>().get_serial() == serial {
            break;
        }
        seen.push(focus(ui));
    }
    seen
}

/// What a screen reader sees on screen: each element with a role and a name, and its box.
fn named(ui: &Testbed) -> Vec<(String, f32, f32, f32, f32)> {
    use i_slint_backend_testing::AccessibleRole as R;
    let mut out: Vec<(String, f32, f32, f32, f32)> = ElementQuery::from_root(ui)
        .match_predicate(|e| {
            !matches!(e.accessible_role(), None | Some(R::None))
                && e.accessible_label().is_some_and(|l| !l.is_empty())
        })
        .find_all()
        .into_iter()
        .filter_map(|e| {
            let (p, z) = (e.absolute_position(), e.size());
            (z.width > 0.0 && z.height > 0.0 && p.y >= 0.0 && p.y + z.height <= 480.0).then(|| {
                (
                    e.accessible_label().unwrap().to_string(),
                    p.x,
                    p.y,
                    z.width,
                    z.height,
                )
            })
        })
        .collect();
    out.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(a.2.total_cmp(&b.2))
            .then(a.1.total_cmp(&b.1))
    });
    out
}

struct Run {
    failures: Vec<String>,
    passed: usize,
    /// Every check, as it ran: its name, whether it passed and why not.
    results: Vec<(String, bool, String)>,
}

impl Run {
    fn check(&mut self, scenario: &str, ok: bool, detail: impl FnOnce() -> String) {
        if ok {
            self.passed += 1;
            self.results
                .push((scenario.to_string(), true, String::new()));
        } else {
            let why = detail();
            self.failures.push(format!("{scenario}: {why}"));
            self.results.push((scenario.to_string(), false, why));
        }
    }

    /// The results as an `opendap.check-result/v1` document (toolkit-host's workspace schemas),
    /// for the verification matrix to collect: one result per check, its id `interface/<name>`,
    /// level V2 (ADR 0014's L2). Written to `ODAP_CHECK_RESULTS` (a folder) or the workspace's
    /// `target/check-results`.
    fn write_results(&self) {
        let esc = |s: &str| {
            let mut o = String::new();
            for c in s.chars() {
                match c {
                    '"' => o.push_str("\\\""),
                    '\\' => o.push_str("\\\\"),
                    '\n' => o.push_str("\\n"),
                    c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
                    c => o.push(c),
                }
            }
            o
        };
        let sh = |cmd: &[&str]| {
            std::process::Command::new(cmd[0])
                .args(&cmd[1..])
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|s| !s.is_empty())
        };
        let commit = sh(&[
            "git",
            "-C",
            env!("CARGO_MANIFEST_DIR"),
            "rev-parse",
            "--short=12",
            "HEAD",
        ]);
        let now = sh(&["date", "-u", "+%Y-%m-%dT%H:%M:%SZ"])
            .unwrap_or_else(|| "1970-01-01T00:00:00Z".into());
        let results: Vec<String> = self
            .results
            .iter()
            .map(|(name, ok, why)| {
                format!(
                    "{{\"id\": \"interface/{}\", \"level\": \"V2\", \"verdict\": \"{}\", \"message\": \"{}\"}}",
                    esc(name),
                    if *ok { "pass" } else { "fail" },
                    esc(if *ok { "as expected" } else { why })
                )
            })
            .collect();
        let doc = format!(
            "{{\n  \"schema\": \"opendap.check-result/v1\",\n  \"tool\": {{\"name\": \"odap-testbed interface tests\"}},\n  \"subject\": {{\"repo\": \"system\", \"path\": \"crates/odap-testbed/tests/interface.rs\"{}, \"variant\": \"headless\"}},\n  \"generated\": \"{now}\",\n  \"results\": [\n    {}\n  ]\n}}\n",
            commit
                .map(|c| format!(", \"commit\": \"{c}\""))
                .unwrap_or_default(),
            results.join(",\n    ")
        );
        let dir = std::env::var_os("ODAP_CHECK_RESULTS")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/check-results")
            });
        if std::fs::create_dir_all(&dir).is_ok() {
            let _ = std::fs::write(dir.join("interface.json"), doc);
        }
    }
}

#[test]
fn the_activity_driven_headless() {
    i_slint_backend_testing::init_integration_test_with_mock_time();
    let ui = Testbed::new().unwrap();
    wire(&ui, None);
    ui.show().unwrap();
    ui.window().set_size(slint::LogicalSize::new(320.0, 480.0));
    tick(16);
    let mut run = Run {
        failures: Vec::new(),
        passed: 0,
        results: Vec::new(),
    };
    let height = 480.0;

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

    // kb-layout, kb-dismiss: Action on a field opens the keyboard and its editing level; back
    // closes it and leaves the focus on the field.
    open(&ui, "Keyboard");
    key(&ui, Key::Tab);
    let field = focus(&ui);
    key(&ui, Key::Return);
    run.check(
        "keyboard opens",
        ui.get_kb_open() && ui.get_kb_field() == 0,
        || format!("open {} field {}", ui.get_kb_open(), ui.get_kb_field()),
    );
    run.check(
        "keyboard keeps the field in view",
        {
            let (y, h) = focus(&ui);
            y >= 0.0 && y + h <= height
        },
        || format!("{:?}", focus(&ui)),
    );
    key(&ui, Key::Escape);
    run.check("back closes the keyboard", !ui.get_kb_open(), || {
        "still open".into()
    });
    run.check(
        "back stays on the screen",
        ui.get_screen() == "Keyboard",
        || ui.get_screen().to_string(),
    );
    run.check("the focus stays on the field", focus(&ui) == field, || {
        format!("{field:?} → {:?}", focus(&ui))
    });

    // field-binding, field-filter: typing reaches the model; a kind refuses what it cannot hold.
    key(&ui, Key::Return);
    typed(&ui, "Studio");
    let name = field_text(&ui, "Profile name");
    run.check("typing shows", name.as_deref() == Some("Studio"), || {
        format!("{name:?}")
    });
    let model = ui.get_kb_fields().row_data(0).map(|f| f.text.to_string());
    run.check(
        "typing reaches the model",
        model.as_deref() == Some("Studio"),
        || format!("{model:?}"),
    );
    key(&ui, Key::Escape);
    key(&ui, Key::Tab);
    key(&ui, Key::Return);
    let before = field_text(&ui, "Preamp (dB)");
    typed(&ui, "x");
    let after = field_text(&ui, "Preamp (dB)");
    run.check(
        "a letter in a gain is refused",
        before.is_some() && before == after,
        || format!("{before:?} → {after:?}"),
    );
    key(&ui, Key::Escape);

    // kb-next: with the keyboard open, Volume goes to the next field and the keyboard stays, editing
    // it.
    open(&ui, "Keyboard");
    key(&ui, Key::Tab);
    key(&ui, Key::Return);
    let first = ui.get_kb_field();
    key(&ui, Key::Tab);
    run.check("kb-next: the keyboard stays open", ui.get_kb_open(), || {
        "closed".into()
    });
    run.check(
        "kb-next: it edits the next field",
        ui.get_kb_field() == first + 1,
        || format!("{first} → {}", ui.get_kb_field()),
    );
    // field-shift: an issue appearing above the edited field (the previous one, left with a bad
    // value) moves it down; it is revealed again, whole.
    key(&ui, Key::Escape);
    open(&ui, "Keyboard");
    key(&ui, Key::Tab);
    key(&ui, Key::Tab); // Preamp
    key(&ui, Key::Return);
    typed(&ui, "99");
    key(&ui, Key::Tab); // leaves it: its issue (above the range) shows above the next field
    tick(100);
    let (y, h) = focus(&ui);
    let issue = ui
        .get_kb_fields()
        .row_data(1)
        .map(|f| f.issue.to_string())
        .unwrap_or_default();
    run.check(
        "field-shift: the left field shows its issue",
        !issue.is_empty(),
        || "no issue".into(),
    );
    run.check(
        "field-shift: the edited field is revealed whole",
        y >= 76.0 && y + h <= 480.0,
        || format!("{y} + {h}"),
    );
    // field-a11y: a screen reader reads the issue after the field's name: it is the field's
    // description (aria-describedby), on the element that holds its text, and only while it stands.
    let described = |ui: &Testbed, label: &str| {
        ElementHandle::find_by_accessible_label(ui, label)
            .find(|e| {
                e.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::TextInput)
            })
            .and_then(|e| e.accessible_description())
            .map(|d| d.to_string())
            .unwrap_or_default()
    };
    let told = described(&ui, "Preamp (dB)");
    run.check(
        "field-a11y: the issue follows the field's name",
        !issue.is_empty() && told == issue,
        || format!("{told:?} for {issue:?}"),
    );
    run.check(
        "field-a11y: a field without an issue describes nothing",
        described(&ui, "Profile name").is_empty(),
        || described(&ui, "Profile name"),
    );
    key(&ui, Key::Escape);

    // kb-variant: a digits field asks for the digit keyboard.
    open(&ui, "Keyboard");
    for _ in 0..5 {
        key(&ui, Key::Tab);
    }
    key(&ui, Key::Return);
    let digits = ui.get_kb_digits();
    let at = ui.get_kb_field();
    run.check("a code asks for digits", at == 4 && digits, || {
        format!("field {at}, digits {digits}")
    });
    key(&ui, Key::Escape);

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

    // Past the ends (opendap-input's ends): the days of the week go round, the shelf of albums stops.
    open(&ui, "Navigation");
    let days = |ui: &Testbed, k: &str| {
        ui.invoke_control_key("days".into(), k.into());
        tick(16);
        (
            ui.get_nav_days().focus.to_string(),
            ui.get_nav_text().to_string(),
        )
    };
    days(&ui, "enter");
    let (at, _) = days(&ui, "activate");
    run.check("days: Action enters on Monday", at == "Mon", || at.clone());
    let (at, said) = days(&ui, "prev");
    run.check(
        "days: before Monday, Sunday",
        at == "Sun" && said.starts_with("round to Sun"),
        || format!("{at}: {said}"),
    );
    let (at, said) = days(&ui, "next");
    run.check(
        "days: after Sunday, Monday",
        at == "Mon" && said.starts_with("round to Mon"),
        || format!("{at}: {said}"),
    );
    let (at, said) = days(&ui, "next");
    run.check(
        "days: a move inside is a move",
        at == "Tue" && said.starts_with("moved to Tue"),
        || format!("{at}: {said}"),
    );
    ui.invoke_control_key("shelf".into(), "enter".into());
    ui.invoke_control_key("shelf".into(), "activate".into());
    ui.invoke_control_key("shelf".into(), "prev".into());
    tick(16);
    let said = ui.get_nav_text().to_string();
    run.check(
        "shelf: a list stops; past its start, out",
        said.starts_with("released shelf"),
        || said.clone(),
    );

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

    // An adjustable (Blackbird UI's Adjustable): Action locks it, locked Volume + raises it,
    // Escape unlocks, unlocked Volume moves the focus on; ← → step it, mirrored right to left.
    open(&ui, "Controls");
    key(&ui, Key::Tab); // shows the focus, on the slider (focus on entry)
    let l0 = ui.get_level();
    let at = focus(&ui);
    back_key(&ui);
    run.check(
        "slider: unlocked, Volume moves the focus, not the value",
        ui.get_level() == l0,
        || format!("{l0} → {}", ui.get_level()),
    );
    key(&ui, Key::Tab);
    run.check("slider: back on it", focus(&ui) == at, || {
        format!("{at:?} → {:?}", focus(&ui))
    });
    key(&ui, Key::Return);
    back_key(&ui);
    run.check(
        "slider: locked, Volume + raises it 5",
        (ui.get_level() - (l0 + 5.0)).abs() < 0.01,
        || format!("{l0} → {}", ui.get_level()),
    );
    key(&ui, Key::Tab);
    run.check(
        "slider: locked, Volume − lowers it 5",
        (ui.get_level() - l0).abs() < 0.01,
        || format!("{l0} → {}", ui.get_level()),
    );
    key(&ui, Key::Escape);
    run.check(
        "slider: Escape unlocks, the screen stays",
        ui.get_screen() == "Controls",
        || ui.get_screen().to_string(),
    );
    key(&ui, Key::Tab);
    run.check(
        "slider: unlocked again, Volume moves on",
        focus(&ui) != at && (ui.get_level() - l0).abs() < 0.01,
        || format!("{:?}, level {}", focus(&ui), ui.get_level()),
    );
    back_key(&ui);
    key(&ui, Key::RightArrow);
    run.check(
        "slider: → raises it left to right",
        (ui.get_level() - (l0 + 5.0)).abs() < 0.01,
        || format!("{}", ui.get_level()),
    );
    ui.global::<Layout>().set_rtl(true);
    key(&ui, Key::RightArrow);
    run.check(
        "slider: → lowers it right to left",
        (ui.get_level() - l0).abs() < 0.01,
        || format!("{}", ui.get_level()),
    );
    ui.global::<Layout>().set_rtl(false);

    // The window losing and getting back the focus (the display off and on) leaves a range on the
    // knob it was on: after it, → moves the high knob, as before.
    open(&ui, "Controls");
    key(&ui, Key::Tab);
    key(&ui, Key::Tab); // the range, low knob
    key(&ui, Key::Tab); // its high knob
    let (lo, hi) = (ui.get_range_low(), ui.get_range_high());
    ui.window()
        .dispatch_event(WindowEvent::WindowActiveChanged(false));
    tick(16);
    ui.window()
        .dispatch_event(WindowEvent::WindowActiveChanged(true));
    tick(16);
    key(&ui, Key::RightArrow);
    run.check(
        "range: the window's focus back keeps the knob",
        ui.get_range_low() == lo && ui.get_range_high() > hi,
        || {
            format!(
                "low {lo} → {}, high {hi} → {}",
                ui.get_range_low(),
                ui.get_range_high()
            )
        },
    );

    // tr-pager-buttons: locked, Volume + turns the pager forward, Volume − back.
    open(&ui, "Transitions");
    key(&ui, Key::Tab);
    key(&ui, Key::Return);
    let p0 = ui.get_tr_page();
    back_key(&ui);
    tick(600);
    let p1 = ui.get_tr_page();
    run.check(
        "pager: locked, Volume + turns forward",
        p1 == p0 + 1,
        || format!("page {p0} → {p1}"),
    );
    key(&ui, Key::Tab);
    tick(600);
    let p2 = ui.get_tr_page();
    run.check("pager: locked, Volume − turns back", p2 == p0, || {
        format!("page {p1} → {p2}")
    });

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

    // comp-disabled-each: on the States screen every focusable component appears disabled. The
    // buttons never land on one, a touch on one takes no focus, and each is dimmed once (its own
    // opacity, not compounded by a disabled parent).
    let off_all = |ui: &Testbed| -> Vec<(String, f32, f32, f32, f32)> {
        ElementQuery::from_root(ui)
            .match_predicate(|e| {
                e.accessible_label().is_some_and(|l| l.starts_with("Off "))
                    && e.accessible_role().is_some()
            })
            .find_all()
            .into_iter()
            .map(|e| {
                let (p, z) = (e.absolute_position(), e.size());
                (
                    e.accessible_label().unwrap().to_string(),
                    p.x,
                    p.y,
                    z.width,
                    z.height,
                )
            })
            .filter(|e| e.3 > 0.0 && e.4 > 0.0)
            .collect()
    };
    open(&ui, "States");
    // The query sees what the scroll area shows: the gallery, at the screen's end, comes into view
    // as the walk reveals what follows the focus; a focus landing on one would be revealed too.
    key(&ui, Key::Tab);
    let mut landed = Vec::new();
    for _ in 0..60 {
        let (y, h) = focus(&ui);
        for e in off_all(&ui) {
            if (y - e.2).abs() < 1.0 && (h - e.4).abs() < 1.0 {
                landed.push(e.0.clone());
            }
        }
        let serial = ui.global::<FocusReveal>().get_serial();
        key(&ui, Key::Tab);
        if ui.global::<FocusReveal>().get_serial() == serial {
            break;
        }
    }
    run.check(
        "disabled: the buttons skip every one",
        landed.is_empty(),
        || landed.join(", "),
    );
    // Nothing focusable follows the error state, so the buttons stop there; a finger scrolls the
    // rest into view (the gallery), then touches each.
    for _ in 0..6 {
        if off_all(&ui)
            .iter()
            .filter(|e| e.2 >= 0.0 && e.2 + e.4 <= 480.0)
            .count()
            >= 6
        {
            break;
        }
        use slint::platform::PointerEventButton as B;
        let p = |y: f32| slint::LogicalPosition::new(300.0, y);
        ui.window()
            .dispatch_event(WindowEvent::PointerMoved { position: p(420.0) });
        ui.window().dispatch_event(WindowEvent::PointerPressed {
            position: p(420.0),
            button: B::Left,
        });
        for i in 1..=12 {
            ui.window().dispatch_event(WindowEvent::PointerMoved {
                position: p(420.0 - i as f32 * 25.0),
            });
            tick(16);
        }
        ui.window().dispatch_event(WindowEvent::PointerReleased {
            position: p(120.0),
            button: B::Left,
        });
        tick(400);
    }
    let visible: Vec<_> = off_all(&ui)
        .into_iter()
        .filter(|e| e.2 >= 0.0 && e.2 + e.4 <= 480.0)
        .collect();
    run.check(
        "disabled: the gallery in view at the end",
        visible.len() >= 4,
        || format!("{} in view", visible.len()),
    );
    let mut acted = Vec::new();
    for e in &visible {
        let serial = ui.global::<FocusReveal>().get_serial();
        let at = slint::LogicalPosition::new(e.1 + e.3 / 2.0, e.2 + e.4 / 2.0);
        use slint::platform::PointerEventButton as B;
        ui.window()
            .dispatch_event(WindowEvent::PointerMoved { position: at });
        ui.window().dispatch_event(WindowEvent::PointerPressed {
            position: at,
            button: B::Left,
        });
        tick(30);
        ui.window().dispatch_event(WindowEvent::PointerReleased {
            position: at,
            button: B::Left,
        });
        tick(30);
        if ui.global::<FocusReveal>().get_serial() != serial {
            acted.push(e.0.clone());
        }
    }
    run.check(
        "disabled: a touch gives none the focus",
        acted.is_empty(),
        || acted.join(", "),
    );
    let dims: Vec<String> = ElementQuery::from_root(&ui)
        .match_predicate(|e| {
            e.accessible_label().is_some_and(|l| l.starts_with("Off "))
                && e.accessible_role().is_some()
        })
        .find_all()
        .into_iter()
        .filter_map(|e| {
            // Dimmed as a whole, or part by part (a line's label, a toggle's switch): each part once.
            let o = e.computed_opacity();
            let parts: Vec<f32> = e
                .query_descendants()
                .match_predicate(|c| {
                    c.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::Text)
                })
                .find_all()
                .into_iter()
                .map(|c| c.computed_opacity())
                .collect();
            let whole = (o - 0.45).abs() <= 0.02;
            let by_parts = (o - 1.0).abs() <= 0.02
                && !parts.is_empty()
                && parts.iter().all(|p| (p - 0.45).abs() <= 0.02);
            (!whole && !by_parts)
                .then(|| format!("{} {o:.2} parts {parts:?}", e.accessible_label().unwrap()))
        })
        .collect();
    run.check("disabled: each dimmed once (0.45)", dims.is_empty(), || {
        dims.join(", ")
    });

    // Touch after the buttons: a drag on the nested areas hides the ring and scrolls, the first
    // drag included (it showed the ring and scrolled nothing).
    open(&ui, "Arena");
    // The ring shown on the screen's entry (the pad), not on the nested areas: the drag's press is
    // the first thing to reach them.
    key(&ui, Key::Tab);
    let text0 = ui.get_nested_text().to_string();
    let rows = ElementHandle::find_by_accessible_label(&ui, "outer row 0").next();
    if let Some(r) = rows {
        let p = r.absolute_position();
        let (x, y0) = (p.x + 100.0, p.y + 10.0);
        let at = |y: f32| slint::LogicalPosition::new(x, y);
        use slint::platform::PointerEventButton as B;
        ui.window()
            .dispatch_event(WindowEvent::PointerMoved { position: at(y0) });
        ui.window().dispatch_event(WindowEvent::PointerPressed {
            position: at(y0),
            button: B::Left,
        });
        tick(16);
        for i in 1..=10 {
            ui.window().dispatch_event(WindowEvent::PointerMoved {
                position: at(y0 - i as f32 * 5.0),
            });
            tick(16);
        }
        ui.window().dispatch_event(WindowEvent::PointerReleased {
            position: at(y0 - 50.0),
            button: B::Left,
        });
        tick(16);
        let text1 = ui.get_nested_text().to_string();
        run.check("touch after buttons: the ring hides", !ring(&ui), || {
            "the ring shows".into()
        });
        run.check(
            "touch after buttons: the first drag scrolls",
            text1 != text0,
            || format!("{text0} → {text1}"),
        );
    } else {
        run.check(
            "touch after buttons: the outer list is there",
            false,
            || "no 'outer row 0'".into(),
        );
    }

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

    println!("{} checks passed", run.passed);
    run.write_results();
    assert!(
        run.failures.is_empty(),
        "{} failed:\n{}",
        run.failures.len(),
        run.failures.join("\n")
    );
}
