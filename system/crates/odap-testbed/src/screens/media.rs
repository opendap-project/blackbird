//! The Media screen. A spring (opendap-motion): the block follows the finger, and when let go
//! springs back to the centre with the finger's velocity; frames run only while it moves (the
//! budget), and stop at rest. A waveform decimated to its strip, and covers decoded at the size
//! they show.

use std::{cell::RefCell, rc::Rc, time::Duration, time::Instant};

use opendap_media::{fit, plot};
use opendap_motion::{
    animator::{Animator, Kind as AnimKind},
    budget::Budget,
    spring::Spring,
};
use slint::{ComponentHandle, ModelRc, Timer, VecModel};

use crate::{
    Testbed,
    keys::{Arrows, Router},
};

struct SpringState {
    animator: Animator,
    dragging: bool,
    x: f32,
    press_x: f32,
    last: (f32, f32),
    velocity: f32,
    frames: u32,
    budget: Budget,
}

type Shared = Rc<RefCell<SpringState>>;

pub(super) fn wire(ui: &Testbed, keys: &mut Router<Testbed>, opened: &mut super::Openers<Testbed>) {
    let spring: Shared = Rc::new(RefCell::new(SpringState {
        animator: Animator::new(0.0, AnimKind::Spring(Spring::SETTLE)),
        dragging: false,
        x: 0.0,
        press_x: 0.0,
        last: (0.0, 0.0),
        velocity: 0.0,
        frames: 0,
        budget: Budget::default(),
    }));
    let started = Instant::now();
    let timer = Rc::new(Timer::default());
    let spring_k = spring.clone();
    let weak = ui.as_weak();
    ui.on_spring_drag(move |kind, x| {
        let Some(ui) = weak.upgrade() else { return };
        let now = started.elapsed().as_secs_f32();
        let mut s = spring.borrow_mut();
        match kind.as_str() {
            "down" => {
                s.dragging = true;
                s.press_x = x - s.x;
                s.last = (x, now);
                s.velocity = 0.0;
                s.budget.stop(1);
                timer.stop();
            }
            "move" if s.dragging => {
                let dt = (now - s.last.1).max(1e-3);
                s.velocity = (x - s.last.0) / dt;
                s.last = (x, now);
                s.x = x - s.press_x;
                ui.set_spring_x(s.x);
            }
            "up" | "kick" => {
                s.dragging = false;
                if kind.as_str() == "up" {
                    let (x0, v0) = (s.x, s.velocity);
                    s.animator = Animator::new(x0, AnimKind::Spring(Spring::SETTLE));
                    s.animator.retarget(0.0, now, Some(v0));
                }
                s.frames = 0;
                s.budget.start(1, None);
                settle(&spring, &ui, &timer, started);
            }
            "open" => open(&ui),
            _ => {}
        }
    });
    keys.add(&["spring"], Arrows::Values, move |ui, _, key| {
        if key != "activate" {
            return false;
        }
        // Action throws the block: the spring takes it with that velocity and brings it back.
        let mut s = spring_k.borrow_mut();
        let now = started.elapsed().as_secs_f32();
        s.animator = Animator::new(0.0, AnimKind::Spring(Spring::SETTLE));
        s.animator.retarget(0.0, now, Some(1800.0));
        s.frames = 0;
        drop(s);
        ui.invoke_spring_drag("kick".into(), 0.0);
        true
    });
    // Opened from the list: the waveform and the decode sizes.
    opened.add("Media", |ui| ui.invoke_spring_drag("open".into(), 0.0));
}

/// Frames until the spring rests, then none.
fn settle(sp: &Shared, ui: &Testbed, timer: &Rc<Timer>, started: Instant) {
    let (sp, weak, timer_w) = (sp.clone(), ui.as_weak(), Rc::downgrade(timer));
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(16),
        move || {
            let Some(ui) = weak.upgrade() else { return };
            let now = started.elapsed().as_secs_f32();
            let mut s = sp.borrow_mut();
            let (x, _) = s.animator.sample(now);
            s.x = x;
            s.frames += 1;
            ui.set_spring_x(x);
            if !s.animator.running(now) {
                s.budget.stop(1);
                s.x = 0.0;
                ui.set_spring_x(0.0);
            }
            let needs = s.budget.needs_frame(now);
            ui.set_spring_text(
                format!(
                    "{}: {} frames{}",
                    if needs { "moving" } else { "at rest" },
                    s.frames,
                    if needs { "" } else { ", then none" }
                )
                .into(),
            );
            if !needs && let Some(t) = timer_w.upgrade() {
                t.stop();
            }
        },
    );
}

/// The screen opened: the waveform, a million samples decimated to the strip's columns, and the
/// sizes a cover decodes at.
fn open(ui: &Testbed) {
    let n = 1_000_000usize;
    let samples: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let env = (t * std::f32::consts::PI).sin();
            let v = env * 0.6 * ((i as f32) * 0.0031).sin() * ((i as f32) * 0.00011).cos();
            if i == 612_345 { 0.98 } else { v }
        })
        .collect();
    let cols = plot::min_max(&samples, 144);
    let lo: Vec<f32> = cols.iter().map(|c| c.0).collect();
    let hi: Vec<f32> = cols.iter().map(|c| c.1).collect();
    ui.set_wave_lo(ModelRc::new(VecModel::from(lo)));
    ui.set_wave_hi(ModelRc::new(VecModel::from(hi)));
    let tile = fit::decode_size((3000, 3000), (96.0, 96.0), 1.0, fit::Fit::Cover);
    let screen = fit::decode_size((3000, 3000), (320.0, 320.0), 1.0, fit::Fit::Contain);
    ui.set_media_text(
        format!(
            "{n} samples to {} columns, peak {:.2} kept\na 3000 px cover decodes at {}x{} in a tile,\n{}x{} on Now Playing",
            cols.len(),
            cols.iter().map(|c| c.1).fold(0.0f32, f32::max),
            tile.0,
            tile.1,
            screen.0,
            screen.1
        )
        .into(),
    );
    ui.set_spring_text("at rest: 0 frames".into());
}
