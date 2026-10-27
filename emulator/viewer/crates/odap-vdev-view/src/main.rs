//! `odap-vdev-view`: the simulator window of a virtual device (virtual-devices ADR 0001).
//!
//! It shows the guest's display over QEMU's D-Bus display (opendap-qemu-display, ADR 0002), framed
//! by the device as `odap-vdev frame` describes it, and sends the device's controls with real
//! presses and releases, so long presses and combinations work as on the hardware. Everything it
//! does to the device goes through the OpenDAP CLI's crates, the use cases `opendap` and OpenDAP
//! Desktop share: the controls over a held QMP session (opendap-transport-vm `LiveKeys`), the rest
//! on a worker thread (src/device.rs).
//!
//! Started by `odap-vdev start --window`; it ends when the device stops.

use std::{
    path::PathBuf,
    rc::Rc,
    sync::{Arc, mpsc},
};

use opendap_core::InputSink;
use opendap_core::hardware::Switch;
use opendap_core::power::Phase;
use opendap_model::{Attachment, BootMode, InputEvent, PowerAction, Role};
use opendap_transport_vm::live::LiveKeys;

use serde::{Deserialize, Serialize};
use slint::{ComponentHandle, Model, ModelRc, Rgba8Pixel, SharedPixelBuffer, VecModel};

mod control;
mod device;
mod frame;
mod history;
mod instruments;
mod sound;
mod stats;

slint::include_modules!();

#[derive(Deserialize)]
struct FrameDoc {
    case: Option<frame::Case>,
    front: Option<frame::Front>,
    display: Display,
    controls: Vec<Control>,
    /// The window's keyboard: a host key presses the control of its role (runner only).
    #[serde(default)]
    host_keys: Vec<HostKey>,
    /// The signals the layers declare for the simulator to observe (VD-010, runner only).
    #[serde(default)]
    probes: Vec<ProbeDoc>,
}

#[derive(Deserialize, Clone)]
struct ProbeDoc {
    name: String,
    what: String,
    source: String,
}

/// What each declared probe reads now, kept on the window's thread (the Instruments page's
/// PROBES group): filled from the device's state, the sound and the load as they arrive.
#[derive(Default)]
struct ProbeValues {
    list: Vec<ProbeDoc>,
    led: Option<[u8; 3]>,
    lux: Option<u32>,
    out: String,
    input: String,
    battery: String,
}

thread_local! {
    static PROBES: std::cell::RefCell<ProbeValues> = std::cell::RefCell::default();
}

/// Updates what the probes read and shows them; `change` sets the values that moved.
fn probes_update(ui: &View, change: impl FnOnce(&mut ProbeValues)) {
    PROBES.with(|p| {
        let mut p = p.borrow_mut();
        change(&mut p);
        let rows: Vec<ProbeRow> = p
            .list
            .iter()
            .map(|d| {
                let value = match d.source.as_str() {
                    "audio:out" => p.out.clone(),
                    "audio:in" => p.input.clone(),
                    "board:led" => p.led.map_or("–".into(), |[r, g, b]| {
                        if [r, g, b] == [0, 0, 0] {
                            "off".into()
                        } else {
                            format!("#{r:02x}{g:02x}{b:02x}")
                        }
                    }),
                    "board:als" => p.lux.map_or("–".into(), |l| format!("{l} lx")),
                    "power_supply:battery" => p.battery.clone(),
                    _ => "not read by the simulator".into(),
                };
                ProbeRow {
                    name: d.name.clone().into(),
                    what: d.what.clone().into(),
                    value: if value.is_empty() {
                        "–".into()
                    } else {
                        value.into()
                    },
                }
            })
            .collect();
        ui.set_probes(ModelRc::from(Rc::new(VecModel::from(rows))));
    });
}

#[derive(Deserialize, Clone)]
struct HostKey {
    key: String,
    role: String,
}

#[derive(Deserialize)]
struct Display {
    width_px: u32,
    height_px: u32,
    width_mm: f32,
    height_mm: f32,
    #[serde(default)]
    shape: String,
}

#[derive(Deserialize, Clone)]
struct Control {
    role: String,
    qcode: Option<String>,
}

struct Args {
    frame: PathBuf,
    qmp: PathBuf,
    checkout: PathBuf,
    name: String,
    framed: bool,
    on_top: bool,
}

/// A switch's state on the window, read and set (the sidebar's attachments).
type Getter = fn(&View) -> i32;
type Setter = fn(&View, i32);

/// The window's own choices, kept beside the device's state (view.json, next to the frame) so
/// they hold when the window or the device restarts.
#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
struct Prefs {
    tab: i32,
    out_analyze: bool,
    out_play: Option<bool>,
    in_analyze: bool,
    /// "default", "tone", "silence", or a host input's id.
    in_source: Option<String>,
    /// How each strip shows the sound: 0 the spectrum, 1 the spectrogram.
    out_view: i32,
    in_view: i32,
}

impl Prefs {
    fn path(frame: &std::path::Path) -> PathBuf {
        frame.with_file_name("view.json")
    }

    fn load(frame: &std::path::Path) -> Prefs {
        std::fs::read_to_string(Prefs::path(frame))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    fn save(&self, frame: &std::path::Path) {
        if let Ok(t) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(Prefs::path(frame), t);
        }
    }
}

fn source_key(s: &sound::InSource) -> String {
    match s {
        sound::InSource::Device(None) => "default".into(),
        sound::InSource::Device(Some(id)) => id.clone(),
        sound::InSource::Tone => "tone".into(),
        sound::InSource::Silence => "silence".into(),
    }
}

fn args() -> Args {
    let mut a = Args {
        frame: PathBuf::new(),
        qmp: PathBuf::new(),
        checkout: PathBuf::new(),
        name: "Virtual device".into(),
        framed: true,
        on_top: true,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut value = || {
            it.next()
                .unwrap_or_else(|| usage(&format!("{arg} needs a value")))
        };
        match arg.as_str() {
            "--frame" => a.frame = value().into(),
            "--qmp" => a.qmp = value().into(),
            "--checkout" => a.checkout = value().into(),
            "--name" => a.name = value(),
            "--no-frame" => a.framed = false,
            "--no-on-top" => a.on_top = false,
            "-h" | "--help" => usage(""),
            other => usage(&format!("unknown argument {other}")),
        }
    }
    a
}

fn usage(msg: &str) -> ! {
    if !msg.is_empty() {
        eprintln!("odap-vdev-view: {msg}");
    }
    eprintln!(
        "usage: odap-vdev-view --frame FRAME.json --qmp SOCKET --checkout VIRTUAL_DEVICES_DIR [--name NAME] [--no-frame] [--no-on-top]"
    );
    std::process::exit(if msg.is_empty() { 0 } else { 2 });
}

fn main() -> Result<(), slint::PlatformError> {
    let a = args();
    let text = std::fs::read_to_string(&a.frame)
        .unwrap_or_else(|e| usage(&format!("{}: {e}", a.frame.display())));
    let doc: FrameDoc =
        serde_json::from_str(&text).unwrap_or_else(|e| usage(&format!("frame: {e}")));
    let px_mm = doc.display.width_px as f32 / doc.display.width_mm;

    // Kept on top from the start: the window is created with that level (winit's attributes).
    if a.on_top {
        slint::BackendSelector::new()
            .with_winit_window_attributes_hook(|attrs| {
                attrs.with_window_level(slint::winit_030::winit::window::WindowLevel::AlwaysOnTop)
            })
            .select()?;
    }
    let ui = View::new()?;
    ui.set_name(a.name.clone().into());
    {
        let list = doc.probes.clone();
        probes_update(&ui, move |p| p.list = list);
    }
    ui.set_framed(a.framed);
    ui.set_screen_w(doc.display.width_px as f32);
    ui.set_screen_h(doc.display.height_px as f32);
    // The front, drawn at twice the logical density so it stays sharp on a Retina display.
    let framed = a.framed && doc.case.is_some() && doc.front.is_some();
    ui.set_framed(framed);
    let mut key_rects: Vec<(String, frame::Rect)> = Vec::new();
    let mut sprites: Vec<frame::Sprite> = Vec::new();
    if let (true, Some(case), Some(front)) = (framed, doc.case.as_ref(), doc.front.as_ref()) {
        let panel = frame::Panel {
            width_mm: doc.display.width_mm,
            height_mm: doc.display.height_mm,
            round: doc.display.shape == "round",
        };
        let drawn = frame::draw(case, front, &panel, px_mm, 2.0);
        let buffer = SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
            &drawn.rgba,
            drawn.width,
            drawn.height,
        );
        ui.set_front(slint::Image::from_rgba8_premultiplied(buffer));
        ui.set_front_w(drawn.size.0);
        ui.set_front_h(drawn.size.1);
        ui.set_screen_x(drawn.screen.x);
        ui.set_screen_y(drawn.screen.y);
        ui.set_led_x(drawn.led.0);
        ui.set_led_y(drawn.led.1);
        ui.set_led_d(drawn.led.2);
        key_rects = drawn.keys;
        sprites = drawn.sprites;
    }

    // One key per role, with the shape the concept art gives it (T1-059).
    let mut keys: Vec<Key> = Vec::new();
    let mut qcode_of: Vec<(String, String)> = Vec::new();
    let host_keys: Vec<(String, String)> = doc
        .host_keys
        .iter()
        .map(|h| (h.key.clone(), h.role.clone()))
        .collect();
    for c in &doc.controls {
        let Some(q) = c.qcode.clone() else { continue };
        if keys.iter().any(|k| k.role == c.role.as_str()) {
            continue;
        }
        let r = key_rects
            .iter()
            .find(|(role, _)| *role == c.role)
            .map(|(_, r)| *r)
            .unwrap_or_default();
        let mut key = Key {
            role: c.role.clone().into(),
            x: r.x,
            y: r.y,
            w: r.w,
            h: r.h,
            pressed: false,
            ..Default::default()
        };
        if let Some(sp) = sprites.iter().find(|s| s.role == c.role) {
            let buffer = SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
                &sp.rgba, sp.width, sp.height,
            );
            key.sprite = slint::Image::from_rgba8_premultiplied(buffer);
            key.has_sprite = true;
            (key.sx, key.sy, key.sw, key.sh, key.travel) =
                (sp.rect.x, sp.rect.y, sp.rect.w, sp.rect.h, sp.travel);
        }
        keys.push(key);
        qcode_of.push((c.role.clone(), q));
    }
    let keys_model = Rc::new(VecModel::from(keys));
    ui.set_keys(ModelRc::from(keys_model.clone()));

    // The controls, in order, on a thread of their own: the window never waits for QMP.
    let (key_tx, key_rx) = mpsc::channel::<InputEvent>();
    // A keyboard's keys (← →) go the same way, in order with the buttons.
    let keyboard_tx = key_tx.clone();
    {
        let live = LiveKeys::new(
            &a.qmp,
            qcode_of
                .iter()
                .map(|(r, q)| (Role::from(r.as_str()), q.clone()))
                .collect(),
        );
        std::thread::spawn(move || {
            // ODAP_VIEW_TRACE=1: every control event, as sent, on stderr (window.log).
            let trace = std::env::var_os("ODAP_VIEW_TRACE").is_some();
            for event in key_rx {
                if trace {
                    eprintln!("odap-vdev-view: {event:?}");
                }
                if let Err(e) = live.send(&event) {
                    eprintln!("odap-vdev-view: {}", e.message);
                }
            }
        });
    }
    let press = {
        let keys_model = keys_model.clone();
        let weak = ui.as_weak();
        move |role: &str, down: bool| {
            // Already so (a key the OS repeats, the mouse and a key on the same control): nothing.
            let same = (0..keys_model.row_count())
                .filter_map(|i| keys_model.row_data(i))
                .any(|k| k.role == role && k.pressed == down);
            if same {
                return;
            }
            let role = Role::from(role);
            let _ = key_tx.send(if down {
                InputEvent::KeyDown { role: role.clone() }
            } else {
                InputEvent::KeyUp { role: role.clone() }
            });
            let role = role.as_str();
            if let Some(ui) = weak.upgrade() {
                match role {
                    "action" => ui.set_action_down(down),
                    "main" => ui.set_main_down(down),
                    "volume-up" => ui.set_volume_up_down(down),
                    "volume-down" => ui.set_volume_down_down(down),
                    _ => {}
                }
            }
            for i in 0..keys_model.row_count() {
                let mut k = keys_model.row_data(i).unwrap();
                if k.role == role {
                    k.pressed = down;
                    keys_model.set_row_data(i, k);
                }
            }
        }
    };
    let press = Rc::new(press);
    {
        let press = press.clone();
        let trace = std::env::var_os("ODAP_VIEW_TRACE").is_some();
        ui.on_key(move |role, down| {
            if trace {
                eprintln!("odap-vdev-view: key {role} {down}");
            }
            press(role.as_str(), down)
        });
    }
    {
        let press = press.clone();
        let trace = std::env::var_os("ODAP_VIEW_TRACE").is_some();
        ui.on_combo(move |down| {
            if trace {
                eprintln!("odap-vdev-view: combo {down}");
            }
            press("main", down);
            press("volume-down", down);
        });
    }
    {
        // The host keys the faceplate gives (Space, ↑, ↓, P): their role, by their qcode.
        let press = press.clone();
        let trace = std::env::var_os("ODAP_VIEW_TRACE").is_some();
        ui.on_host_key(move |text, down| {
            if trace {
                eprintln!("odap-vdev-view: host key {:?} {down}", text.as_str());
            }
            // ↑ and ↓ are the buttons by their place (Volume + above, Volume − below), as the
            // faceplate's host keys say. ← and → are no button (the buttons stand vertically):
            // they reach the device as a keyboard's arrows, so a focused slider takes them
            // (→ more, ← less). Enter is Action, as Space is: the key that takes the default
            // action or the focused element; which of the two Action does is the device's call.
            match text.as_str() {
                "\n" | "\r" => return press("action", down),
                "\u{F702}" | "\u{F703}" => {
                    let key = if text.as_str() == "\u{F702}" {
                        "left"
                    } else {
                        "right"
                    };
                    let _ = keyboard_tx.send(InputEvent::Keyboard {
                        key: key.into(),
                        down,
                    });
                    return;
                }
                _ => {}
            }
            let qcode = match text.as_str() {
                " " => "spc",
                "\u{F700}" => "up",
                "\u{F701}" => "down",
                t => t,
            };
            if let Some((_, role)) = host_keys.iter().find(|(q, _)| q == qcode) {
                press(role, down);
            }
        });
    }

    // The sidebar's other actions, on the worker; the window shows the state it publishes.
    let worker = {
        let weak = ui.as_weak();
        device::Worker::start(a.checkout.clone(), move |st| {
            let weak = weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = weak.upgrade() {
                    show(&ui, &st);
                }
            });
        })
    };
    let jobs = worker.jobs.clone();
    {
        let jobs = jobs.clone();
        let device_name = a.name.clone();
        let weak = ui.as_weak();
        ui.on_action(move |what| {
            let Some(ui) = weak.upgrade() else { return };
            let job = match what.as_str() {
                // Save PNG: the display as the host sees it, to the Desktop, with no part of the OS involved.
                "screenshot" => {
                    // In flight from the click: a second click never saves a second picture.
                    if ui.get_saving() {
                        return;
                    }
                    ui.set_saving(true);
                    let shots = std::env::var_os("HOME")
                        .map(PathBuf::from)
                        .unwrap_or_default()
                        .join("Desktop");
                    device::Job::SavePng(shots.join(format!("{device_name} {}.png", stamp())))
                }
                "reboot" => device::Job::Power(PowerAction::Reboot {
                    mode: BootMode::Normal,
                }),
                "reboot-debug" => device::Job::Power(PowerAction::Reboot {
                    mode: BootMode::Debug,
                }),
                "poweroff" => device::Job::Power(PowerAction::Poweroff),
                _ => return,
            };
            let _ = jobs.send(job);
        });
    }
    {
        let jobs = jobs.clone();
        ui.on_battery_set(move |level| {
            let _ = jobs.send(device::Job::Battery(level.round().clamp(0.0, 100.0) as u8));
        });
    }
    {
        let jobs = jobs.clone();
        ui.on_light_set(move |position| {
            let _ = jobs.send(device::Job::Light(slider_to_lux(position)));
        });
    }
    {
        let jobs = jobs.clone();
        let weak = ui.as_weak();
        let debug_jobs = jobs.clone();
        let debug_weak = ui.as_weak();
        ui.on_set_debug_ui(move |on| {
            if let Some(ui) = debug_weak.upgrade() {
                // In flight from the click, as the other switches.
                match (ui.get_debug_ui(), on) {
                    (0, true) => ui.set_debug_ui(1),
                    (2, false) => ui.set_debug_ui(3),
                    _ => return,
                }
            }
            let _ = debug_jobs.send(device::Job::DebugUi(on));
        });
        ui.on_attach(move |what, on| {
            let Some(ui) = weak.upgrade() else { return };
            let (a, get, set): (Attachment, Getter, Setter) = match what.as_str() {
                "microsd" => (Attachment::Microsd, View::get_sd, View::set_sd),
                "usb-storage" => (
                    Attachment::UsbStorage,
                    View::get_usb_disk,
                    View::set_usb_disk,
                ),
                "usb-audio" => (Attachment::UsbAudio, View::get_usb_dac, View::set_usb_dac),
                "charger" => (Attachment::Charger, View::get_charger, View::set_charger),
                _ => return,
            };
            // In flight from the click, even while the worker finishes another change: the
            // worker's next state confirms it, or puts it back if it does not apply.
            match (get(&ui), on) {
                (0, true) => set(&ui, 1),
                (2, false) => set(&ui, 3),
                _ => return,
            }
            let _ = jobs.send(device::Job::Attach(a, on));
        });
    }

    // The display: QEMU's D-Bus display pushes each change as the guest draws it (virtual-devices
    // ADR 0002); the frame is drawn on the event loop. Attached over the window's QMP socket before
    // any key uses it (QMP serves one client at a time; the keys open their session on first use).
    let weak = ui.as_weak();
    // ODAP_VIEW_FRAMES_LOG=FILE: one line per change the window receives (seconds since it opened,
    // and the rectangle), to measure the window's own frame rate (odapkit motion measures the
    // device's). Off unless asked for.
    let frames_log = std::env::var_os("ODAP_VIEW_FRAMES_LOG")
        .and_then(|p| std::fs::File::create(p).ok())
        .map(|f| std::sync::Mutex::new(std::io::LineWriter::new(f)));
    let opened = std::time::Instant::now();
    let display = match opendap_qemu_display::attach(&a.qmp, move |frame, change| {
        if let Some(log) = &frames_log {
            use std::io::Write as _;
            let at = opened.elapsed().as_secs_f64();
            let line = match change {
                opendap_qemu_display::Change::Pixels { x, y, w, h } => {
                    format!("{at:.4} {x} {y} {w} {h}")
                }
                c => format!("{at:.4} {c:?}"),
            };
            let _ = writeln!(log.lock().unwrap_or_else(|p| p.into_inner()), "{line}");
        }
        if change == opendap_qemu_display::Change::Disabled {
            return;
        }
        let (w, h) = (frame.width, frame.height);
        let mut rgba = Vec::with_capacity(frame.bgrx.len());
        for p in frame.bgrx.as_chunks::<4>().0 {
            rgba.extend_from_slice(&[p[2], p[1], p[0], 255]);
        }
        let weak = weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = weak.upgrade() {
                let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&rgba, w, h);
                ui.set_screen(slint::Image::from_rgba8(buffer));
            }
        });
    }) {
        Ok(d) => Arc::new(d),
        Err(e) => usage(&format!("display: {e}")),
    };
    {
        // The pointer over the display, pressed or not, so the composer's touch indicator follows it.
        let display = display.clone();
        ui.on_screen_pointer(move |kind, x, y| {
            let pressed = match kind.as_str() {
                "down" => Some(true),
                "up" => Some(false),
                _ => None,
            };
            let _ = display.pointer(x.max(0.0) as u32, y.max(0.0) as u32, pressed);
        });
    }
    // The device's sound through the same display session (virtual-devices ADR 0003): played
    // on this computer, and measured on the Instruments tab.
    let sound = {
        let weak = ui.as_weak();
        // The spectrograms scroll on the listeners' threads; the window gets each new picture.
        let waterfalls: Arc<std::sync::Mutex<[instruments::Waterfall; 2]>> = Arc::default();
        let attached = sound::Sound::attach(display.session(), move |direction, reading| {
            let waterfall = {
                let mut w = waterfalls.lock().unwrap_or_else(|p| p.into_inner());
                let w = &mut w[usize::from(direction == sound::Direction::Input)];
                if reading.active {
                    w.push(&reading.rows);
                }
                w.image()
            };
            let tint = match direction {
                sound::Direction::Output => [59, 130, 196],
                sound::Direction::Input => [255, 159, 10],
            };
            let bars = instruments::bars(&reading.levels);
            let strip = instruments::strip(if reading.active { &reading.bands } else { &[] }, tint);
            let text = instruments::readout(&reading);
            let weak = weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                let Some(ui) = weak.upgrade() else { return };
                let meters = ModelRc::from(Rc::new(VecModel::from(bars)));
                let image = slint::Image::from_rgba8(strip);
                match direction {
                    sound::Direction::Output => {
                        let f = if reading.active {
                            reading.format.clone()
                        } else {
                            "nothing playing".into()
                        };
                        probes_update(&ui, |p| p.out = f);
                        ui.set_out_active(reading.active);
                        ui.set_out_format(reading.format.into());
                        ui.set_out_meters(meters);
                        ui.set_out_spectrum(image);
                        ui.set_out_waterfall(slint::Image::from_rgba8(waterfall));
                        ui.set_out_readout(text.into());
                        ui.set_out_note(reading.note.into());
                    }
                    sound::Direction::Input => {
                        let f = if reading.active {
                            reading.format.clone()
                        } else {
                            "not recording".into()
                        };
                        probes_update(&ui, |p| p.input = f);
                        ui.set_in_active(reading.active);
                        ui.set_in_format(reading.format.into());
                        ui.set_in_meters(meters);
                        ui.set_in_spectrum(image);
                        ui.set_in_waterfall(slint::Image::from_rgba8(waterfall));
                        ui.set_in_readout(text.into());
                        ui.set_in_note(reading.note.into());
                    }
                }
            });
        });
        match attached {
            Ok(s) => Some(Arc::new(s)),
            Err(e) => {
                // A device started without the D-Bus audio (QEMU's own window): no sound here.
                eprintln!("odap-vdev-view: sound: {e}");
                ui.set_out_note("No sound: the device was started without its D-Bus audio".into());
                None
            }
        }
    };
    // Tools ask the window for the device's sound instead of attaching to QEMU themselves.
    control::serve(&a.frame, sound.clone());

    // The window's choices from last time (view.json), and kept as they change.
    let prefs = Rc::new(std::cell::RefCell::new(Prefs::load(&a.frame)));
    let save = {
        let prefs = prefs.clone();
        let frame = a.frame.clone();
        move |change: &dyn Fn(&mut Prefs)| {
            change(&mut prefs.borrow_mut());
            prefs.borrow().save(&frame);
        }
    };
    {
        let p = prefs.borrow().clone();
        ui.set_tab(p.tab.clamp(0, 2));
        ui.set_out_analyze(p.out_analyze);
        ui.set_out_play(p.out_play.unwrap_or(true));
        ui.set_in_analyze(p.in_analyze);
        ui.set_out_view(p.out_view.clamp(0, 1));
        ui.set_in_view(p.in_view.clamp(0, 1));
    }
    {
        let save = save.clone();
        ui.on_sound_view(move |which, view| {
            let out = which == "out";
            save(&|p| {
                if out {
                    p.out_view = view
                } else {
                    p.in_view = view
                }
            });
        });
    }
    if let Some(sound) = sound.clone() {
        let p = prefs.borrow().clone();
        // Measuring starts once the Instruments page shows (the window's `changed` handlers).
        let shown = ui.get_sidebar() && ui.get_tab() == 1;
        sound.set_analyze(sound::Direction::Output, p.out_analyze && shown);
        sound.set_analyze(sound::Direction::Input, p.in_analyze && shown);
        if p.out_play == Some(false) {
            let s = sound.clone();
            std::thread::spawn(move || s.set_play(false));
        }
        let (sa, sb, sc) = (sound.clone(), sound.clone(), sound);
        {
            let save = save.clone();
            ui.on_sound_analyze(move |which, on| {
                sa.set_analyze(
                    if which == "out" {
                        sound::Direction::Output
                    } else {
                        sound::Direction::Input
                    },
                    on,
                );
            });
            ui.on_sound_measure_wish(move |which, on| {
                let out = which == "out";
                save(&|p| {
                    if out {
                        p.out_analyze = on
                    } else {
                        p.in_analyze = on
                    }
                });
            });
        }
        {
            let save = save.clone();
            // A switch's change can wait on the host's audio: off the window's thread.
            ui.on_sound_play(move |on| {
                let sb = sb.clone();
                std::thread::spawn(move || sb.set_play(on));
                save(&|p| p.out_play = Some(on));
            });
        }
        // The sources the list offers, by index: the default input, each of the host's inputs, the
        // test tone, silence. Read again whenever the Instruments page opens (a microphone plugged
        // in since shows up). The choice is kept by its key, so it survives a list that changed.
        let sources: Rc<std::cell::RefCell<Vec<sound::InSource>>> = Rc::default();
        let refresh = {
            let sources = sources.clone();
            let prefs = prefs.clone();
            let weak = ui.as_weak();
            move || {
                let (devices, default) = sound::input_devices();
                let Some(ui) = weak.upgrade() else { return };
                let mut list = vec![sound::InSource::Device(None)];
                let mut names: Vec<slint::SharedString> = vec![
                    default
                        .map_or("This computer's default input".into(), |d| {
                            format!("Default input ({d})")
                        })
                        .into(),
                ];
                for d in devices {
                    names.push(d.name.into());
                    list.push(sound::InSource::Device(Some(d.id)));
                }
                list.extend([sound::InSource::Tone, sound::InSource::Silence]);
                names.extend(["Test tone 997 Hz".into(), "Silence".into()]);
                let key = prefs
                    .borrow()
                    .in_source
                    .clone()
                    .unwrap_or_else(|| "default".into());
                let index = list.iter().position(|x| source_key(x) == key).unwrap_or(0);
                *sources.borrow_mut() = list;
                ui.set_in_sources(ModelRc::from(Rc::new(VecModel::from(names))));
                ui.set_in_source(index as i32);
            }
        };
        refresh();
        // The kept source, applied once the list is known.
        if let Some(source) = sources.borrow().get(ui.get_in_source() as usize).cloned() {
            let sc = sc.clone();
            std::thread::spawn(move || sc.set_source(source));
        }
        {
            let refresh = refresh.clone();
            let save = save.clone();
            ui.on_tab_changed(move |tab| {
                save(&|p| p.tab = tab);
                if tab == 1 {
                    refresh();
                }
            });
        }
        ui.on_sound_source(move |i| {
            let sc = sc.clone();
            let Some(source) = sources.borrow().get(i as usize).cloned() else {
                return;
            };
            let key = source_key(&source);
            save(&|p| p.in_source = Some(key.clone()));
            std::thread::spawn(move || sc.set_source(source));
        });
    } else {
        let save = save.clone();
        ui.on_tab_changed(move |tab| save(&|p| p.tab = tab));
    }
    {
        // The device's load, read while the system group shows (stats.rs).
        let watch: Rc<std::cell::RefCell<Option<stats::Watch>>> = Rc::default();
        // The last frame, for the Tasks list to sort and filter again without waiting for the next.
        let last: Arc<std::sync::Mutex<Option<stats::Frame>>> = Arc::default();
        let history: Arc<std::sync::Mutex<history::History>> = Arc::default();
        let again = {
            let (last, weak, history) = (last.clone(), ui.as_weak(), history.clone());
            move || {
                let frame = last.lock().unwrap_or_else(|p| p.into_inner()).clone();
                if let (Some(ui), Some(f)) = (weak.upgrade(), frame) {
                    show_tasks(&ui, &f, &history.lock().unwrap_or_else(|p| p.into_inner()));
                }
            }
        };
        {
            let again = again.clone();
            let weak = ui.as_weak();
            ui.on_task_sort_by(move |key| {
                let Some(ui) = weak.upgrade() else { return };
                // The same column again turns the order; a new one starts from its natural end
                // (busiest, largest first; names and PIDs from the start).
                if ui.get_task_sort() == key {
                    ui.set_task_descending(!ui.get_task_descending());
                } else {
                    ui.set_task_descending(matches!(key.as_str(), "cpu" | "mem"));
                    ui.set_task_sort(key);
                }
                again();
            });
        }
        {
            let again = again.clone();
            ui.on_task_show_kernel(move |_| again());
        }
        {
            // The pointer left the list: it takes its sorted order again.
            let again = again.clone();
            ui.on_task_hover_changed(again);
        }
        {
            let again = again.clone();
            let weak = ui.as_weak();
            ui.on_task_choose(move |pid| {
                if let Some(ui) = weak.upgrade() {
                    ui.set_task_pid(pid);
                }
                again();
            });
        }
        {
            let last = last.clone();
            let weak = ui.as_weak();
            let checkout = a.checkout.clone();
            ui.on_task_act(move |what| {
                let Some(ui) = weak.upgrade() else { return };
                let action = match what.as_str() {
                    "end" => stats::Action::End,
                    "kill" => stats::Action::Kill,
                    "pause" => stats::Action::Pause,
                    "resume" => stats::Action::Resume,
                    "lower" => stats::Action::Lower,
                    "raise" => stats::Action::Raise,
                    _ => return,
                };
                let pid = ui.get_task_pid();
                let chosen = last
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .as_ref()
                    .and_then(|f| f.processes.iter().find(|p| p.pid as i32 == pid).cloned());
                let Some(p) = chosen else { return };
                let (checkout, weak) = (checkout.clone(), ui.as_weak());
                // Over SSH: off the window's thread; the next frame shows the result.
                std::thread::spawn(move || {
                    let r = stats::act(&checkout, action, &p);
                    let _ = slint::invoke_from_event_loop(move || {
                        if let (Some(ui), Err(e)) = (weak.upgrade(), r) {
                            ui.set_note(format!("{}: {e}", p.name).into());
                        }
                    });
                });
            });
        }
        let weak = ui.as_weak();
        let checkout = a.checkout.clone();
        ui.on_sys_watch(move |on| {
            if !on {
                watch.borrow_mut().take();
                return;
            }
            let (weak, last, history) = (weak.clone(), last.clone(), history.clone());
            *watch.borrow_mut() = Some(stats::Watch::start(checkout.clone(), move |frame| {
                *last.lock().unwrap_or_else(|p| p.into_inner()) = Some(frame.clone());
                history
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .record(&frame);
                let history = history.clone();
                let weak = weak.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(ui) = weak.upgrade() else { return };
                    let l = instruments::load(&frame);
                    ui.set_sys_known(true);
                    ui.set_sys_cpu(l.cpu.into());
                    ui.set_sys_cpu_busy(l.cpu_busy);
                    ui.set_sys_mem(l.mem.into());
                    ui.set_sys_mem_busy(l.mem_busy);
                    ui.set_sys_load(l.load.into());
                    ui.set_sys_uptime(l.uptime.into());
                    if let Some(p) = &frame.power {
                        let e = instruments::energy(p);
                        let summary = format!("{}, {}", e.level, e.power);
                        probes_update(&ui, |v| v.battery = summary);
                        ui.set_energy_known(true);
                        ui.set_energy_level(e.level.into());
                        ui.set_energy_power(e.power.into());
                        ui.set_energy_status(e.status.into());
                        ui.set_energy_voltage(e.voltage.into());
                        ui.set_energy_current(e.current.into());
                        ui.set_energy_temperature(e.temperature.into());
                        ui.set_energy_time(e.time.into());
                        ui.set_energy_charger(e.charger.into());
                        let h = history.lock().unwrap_or_else(|p| p.into_inner());
                        let green = [48, 164, 108];
                        ui.set_graph_battery(slint::Image::from_rgba8(instruments::graph(
                            &h.battery.values(),
                            100.0,
                            green,
                        )));
                        // Power drawn shows as positive; the scale follows the largest seen.
                        let drawn: Vec<f32> = h.power.values().iter().map(|w| w.abs()).collect();
                        let top = drawn.iter().copied().fold(1.0f32, f32::max);
                        ui.set_graph_power(slint::Image::from_rgba8(instruments::graph(
                            &drawn,
                            top * 1.25,
                            [255, 159, 10],
                        )));
                    }
                    show_tasks(
                        &ui,
                        &frame,
                        &history.lock().unwrap_or_else(|p| p.into_inner()),
                    );
                });
            }));
        });
    }
    {
        // The device stopped: the window goes with it.
        let display = display.clone();
        std::thread::spawn(move || {
            display.wait_closed();
            let _ = slint::invoke_from_event_loop(|| {
                let _ = slint::quit_event_loop();
            });
        });
    }

    // ODAP_VIEW_SNAPSHOT=FILE.png: the window as drawn, saved once the device has drawn, for
    // checks without a person (an agent, a test) and with the host's screen locked.
    if let Some(path) = std::env::var_os("ODAP_VIEW_SNAPSHOT") {
        let weak = ui.as_weak();
        slint::Timer::single_shot(std::time::Duration::from_secs(3), move || {
            let Some(ui) = weak.upgrade() else { return };
            match ui.window().take_snapshot() {
                Ok(shot) => {
                    let saved = save_png(
                        &PathBuf::from(&path),
                        shot.width(),
                        shot.height(),
                        shot.as_bytes(),
                    );
                    if let Err(e) = saved {
                        eprintln!("odap-vdev-view: snapshot: {e}");
                    }
                }
                Err(e) => eprintln!("odap-vdev-view: snapshot: {e}"),
            }
        });
    }
    let ran = ui.run();
    let _ = std::fs::remove_file(control::path(&a.frame));
    ran
}

thread_local! {
    /// The Tasks list's order as last shown, by PID: kept while the pointer is over the list.
    static TASK_ORDER: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// The Tasks tab from a frame: the list as sorted and filtered, the chosen process and whether
/// it can be acted on.
fn show_tasks(ui: &View, f: &stats::Frame, h: &history::History) {
    let blue = [59, 130, 196];
    ui.set_graph_cpu(slint::Image::from_rgba8(instruments::graph(
        &h.cpu.values(),
        100.0,
        blue,
    )));
    ui.set_graph_mem(slint::Image::from_rgba8(instruments::graph(
        &h.mem.values(),
        h.mem_total,
        [140, 110, 220],
    )));
    ui.set_sys_mem_short(
        format!(
            "{:.0} / {:.0} MiB",
            h.mem.last().unwrap_or(0.0),
            h.mem_total
        )
        .into(),
    );
    if let Some(p) = h.processes.get(&(ui.get_task_pid() as u32)) {
        // A process's CPU can pass 100 % (one core each); its memory scales to its own peak.
        let cpu_max = p.cpu.max().max(100.0);
        ui.set_graph_task_cpu(slint::Image::from_rgba8(instruments::graph(
            &p.cpu.values(),
            cpu_max,
            blue,
        )));
        ui.set_graph_task_mem(slint::Image::from_rgba8(instruments::graph(
            &p.mem.values(),
            p.mem.max() * 1.25,
            [140, 110, 220],
        )));
        ui.set_task_cpu_now(format!("{:.1} %", p.cpu.last().unwrap_or(0.0)).into());
        ui.set_task_mem_now(format!("{:.1} MiB", p.mem.last().unwrap_or(0.0)).into());
    }
    let mut list = instruments::tasks(
        f,
        &ui.get_task_sort(),
        ui.get_task_descending(),
        ui.get_task_kernel(),
    );
    TASK_ORDER.with(|order| {
        let mut order = order.borrow_mut();
        if ui.get_tasks_hovered() && !order.is_empty() {
            // Held: the processes shown before keep their places; new ones go at the end, ended
            // ones leave.
            let rank = |pid: u32| order.iter().position(|p| *p == pid).unwrap_or(usize::MAX);
            list.sort_by_key(|p| rank(p.pid));
        }
        *order = list.iter().map(|p| p.pid).collect();
    });
    let chosen = f
        .processes
        .iter()
        .find(|p| p.pid as i32 == ui.get_task_pid());
    ui.set_task_found(chosen.is_some());
    if let Some(p) = chosen {
        ui.set_task_chosen(instruments::task_row(p));
        ui.set_task_refused(stats::Action::End.refused(p).unwrap_or("").into());
    }
    ui.set_tasks_summary(instruments::tasks_summary(f).into());
    let rows: Vec<TaskRow> = list.iter().map(instruments::task_row).collect();
    ui.set_tasks(ModelRc::from(Rc::new(VecModel::from(rows))));
}

/// The worker's state, on the window.
fn show(ui: &View, st: &device::State) {
    ui.set_phase(match st.phase {
        Phase::Off => 0,
        Phase::Starting => 1,
        Phase::Ready => 2,
        Phase::Rebooting { .. } => 3,
        Phase::Stopping => 4,
    });
    ui.set_sd(st.code(Attachment::Microsd));
    ui.set_usb_disk(st.code(Attachment::UsbStorage));
    ui.set_usb_dac(st.code(Attachment::UsbAudio));
    ui.set_charger(st.code(Attachment::Charger));
    ui.set_battery_known(st.battery.is_some());
    ui.set_battery_busy(st.battery_busy);
    // The status LED as the device drives it; off, nothing is drawn over the unlit part.
    ui.set_led(match st.status_led {
        Some([0, 0, 0]) | None => slint::Color::from_argb_u8(0, 0, 0, 0),
        Some([r, g, b]) => slint::Color::from_rgb_u8(r, g, b),
    });
    ui.set_light_known(st.light.is_some());
    probes_update(ui, |p| {
        p.led = st.status_led;
        p.lux = st.light;
    });
    ui.set_mic_live(st.mic_live);
    // Read the sensor's light once, as the battery: the slider then shows what it sent.
    if let Some(lux) = st.light
        && !ui.get_light_seeded()
    {
        ui.set_light(lux_to_slider(lux));
        ui.set_light_seeded(true);
    }

    // Read the hardware's charge once; after that the slider shows only what it sent.
    if let Some(p) = st.battery
        && !ui.get_battery_seeded()
    {
        ui.set_battery(p as f32);
        ui.set_battery_seeded(true);
    }
    ui.set_saving(st.saving);
    ui.set_debug_ui(match st.debug_ui {
        Switch::Off => 0,
        Switch::Attaching => 1,
        Switch::On => 2,
        Switch::Detaching => 3,
    });
    ui.set_debug_reboot(st.debug_reboot);
    let phase = match st.phase {
        Phase::Off => "Off",
        Phase::Starting => "Starting…",
        Phase::Ready => "Running",
        Phase::Rebooting { .. } => "Restarting…",
        Phase::Stopping => "Powering off…",
    };
    let mut busy: Vec<&str> = Attachment::ALL
        .into_iter()
        .filter(|a| st.switch(*a).busy())
        .map(|a| match a {
            Attachment::Microsd => "microSD",
            Attachment::UsbStorage => "USB disk",
            Attachment::UsbAudio => "USB DAC",
            Attachment::Charger => "charger",
        })
        .collect();
    if st.debug_ui.busy() {
        busy.push("blueprint mode");
    }
    // The line under the name is the device's state only; what is in flight and the notes go to
    // the line at the bottom.
    ui.set_status(phase.into());
    let mut note = String::new();
    if !busy.is_empty() {
        note = busy.join(", ") + "…";
    }
    if !st.note.is_empty() {
        if !note.is_empty() {
            note += " · ";
        }
        note += &st.note;
    }
    ui.set_note(note.into());
}

fn save_png(path: &PathBuf, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().map_err(|e| e.to_string())?;
    w.write_image_data(rgba).map_err(|e| e.to_string())
}

/// `YYYY-MM-DD HH.MM.SS` (UTC) for a saved picture's name, as macOS names screenshots.
fn stamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}.{:02}.{:02}",
        rem / 3600,
        rem / 60 % 60,
        rem % 60
    )
}

/// The light slider is logarithmic, as the eye is: 0 is darkness, 1 is 10 lx, 5 is 100 000 lx
/// (direct sun); 0.0 to 5.0 maps to `10^p − 1`.
fn slider_to_lux(position: f32) -> u32 {
    (10f64.powf(position.clamp(0.0, 5.0) as f64) - 1.0).round() as u32
}

fn lux_to_slider(lux: u32) -> f32 {
    ((lux as f64) + 1.0).log10() as f32
}
