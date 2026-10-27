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
mod wire;

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
    // QEMU runs the sound's input without its mixing engine (the runner's --input-direct).
    input_direct: bool,
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
        input_direct: false,
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
            "--input-direct" => a.input_direct = true,
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
        "usage: odap-vdev-view --frame FRAME.json --qmp SOCKET --checkout VIRTUAL_DEVICES_DIR [--name NAME] [--no-frame] [--no-on-top] [--input-direct]"
    );
    std::process::exit(if msg.is_empty() { 0 } else { 2 });
}

fn main() -> Result<(), slint::PlatformError> {
    let a = args();
    let text = std::fs::read_to_string(&a.frame)
        .unwrap_or_else(|e| usage(&format!("{}: {e}", a.frame.display())));
    let doc: FrameDoc =
        serde_json::from_str(&text).unwrap_or_else(|e| usage(&format!("frame: {e}")));

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
    let (key_rects, sprites) = wire::front(&ui, &doc, a.framed);
    let (keys_model, qcode_of) = wire::keys(&ui, &doc, &key_rects, &sprites);
    let key_tx = wire::sender(&a.qmp, &qcode_of);
    let press = wire::press(&ui, key_tx.clone(), keys_model);
    wire::mouse(&ui, &press);
    wire::host_keys(&ui, &doc, &press, key_tx);
    let _worker = wire::sidebar(&ui, &a.checkout, &a.name);
    let display = wire::display(&ui, &a.qmp);
    let sound = wire::sound(&ui, &display, a.input_direct);
    // Tools ask the window for the device's sound instead of attaching to QEMU themselves.
    control::serve(&a.frame, sound.clone());
    let (prefs, save) = wire::prefs(&ui, &a.frame);
    wire::sound_controls(&ui, sound, prefs, save);
    wire::system(&ui, &a.checkout);
    wire::close_with(&display);
    wire::snapshot(&ui);
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
    show_status(ui, st);
}

/// The line under the name (the device's state only) and the line at the bottom (what is in
/// flight, and the notes).
fn show_status(ui: &View, st: &device::State) {
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
