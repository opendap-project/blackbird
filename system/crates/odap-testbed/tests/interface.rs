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
use odap_testbed::{FocusReveal, FocusVisible, Layout, SCREENS, Testbed, wire};
use slint::platform::{Key, WindowEvent};
use slint::{ComponentHandle, Model, SharedString};

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

// The scenarios, one module a screen or area (tests/interface/), run in this order on one window.
#[path = "interface/arena.rs"]
mod arena;
#[path = "interface/controls.rs"]
mod controls;
#[path = "interface/every_screen.rs"]
mod every_screen;
#[path = "interface/gestures.rs"]
mod gestures;
#[path = "interface/keyboard.rs"]
mod keyboard;
#[path = "interface/navigation.rs"]
mod navigation;
#[path = "interface/states.rs"]
mod states;
#[path = "interface/transitions.rs"]
mod transitions;
#[path = "interface/virtual_list.rs"]
mod virtual_list;

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

    every_screen::entry(&ui, &mut run);
    every_screen::ends(&ui, &mut run);
    keyboard::opens_and_closes(&ui, &mut run);
    keyboard::binding_and_filter(&ui, &mut run);
    keyboard::next_field_and_issues(&ui, &mut run);
    keyboard::digits_variant(&ui, &mut run);
    every_screen::left_to_right_as_kept(&ui, &mut run);
    every_screen::right_to_left_mirrors(&ui, &mut run);
    every_screen::ends_right_to_left(&ui, &mut run);
    navigation::stop_ends(&ui, &mut run);
    virtual_list::jump(&ui, &mut run);
    virtual_list::recycled_slots(&ui, &mut run);
    controls::adjustable(&ui, &mut run);
    controls::window_focus_loss(&ui, &mut run);
    transitions::pager_buttons(&ui, &mut run);
    gestures::pad(&ui, &mut run);
    states::disabled_walk(&ui, &mut run);
    states::disabled_touch(&ui, &mut run);
    states::disabled_dims(&ui, &mut run);
    arena::touch_after_buttons(&ui, &mut run);
    virtual_list::placeholder(&ui, &mut run);
    virtual_list::fling(&ui, &mut run);
    virtual_list::sticky_push(&ui, &mut run);
    every_screen::narrow(&ui, &mut run);

    println!("{} checks passed", run.passed);
    run.write_results();
    assert!(
        run.failures.is_empty(),
        "{} failed:\n{}",
        run.failures.len(),
        run.failures.join("\n")
    );
}
