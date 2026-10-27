//! The Instruments tab's drawings and words, from the device's sound and load: level bars, the
//! spectrum strip and the readouts. Pure, so they are tested without a window.

use opendap_audio_analysis::meter::{CLIP, Level};
use slint::{Rgba8Pixel, SharedPixelBuffer};

use crate::sound::Reading;
use crate::stats;

/// The meters' range: -60 dBFS to full scale.
pub const METER_FLOOR_DB: f32 = -60.0;
/// The spectrum's range: full scale at the top, -140 dBFS at the bottom (a 16-bit stream's noise
/// floor in an 8192-point bin sits near -134 dBFS), a grid line every 20 dB.
pub const SPECTRUM_FLOOR_DB: f32 = -140.0;
/// The strip's pixels: the sidebar's width less its padding, twice for a Retina display.
pub const STRIP_W: u32 = 424;
pub const STRIP_H: u32 = 144;

/// A level's position on a meter, 0 to 1.
pub fn position(db: f32) -> f32 {
    ((db - METER_FLOOR_DB) / -METER_FLOOR_DB).clamp(0.0, 1.0)
}

/// Levels below this read as "below" it: past what the readout claims to resolve.
const TEXT_FLOOR_DB: f32 = -120.0;

fn db_text(db: f32) -> String {
    if db <= TEXT_FLOOR_DB {
        "< −120".into()
    } else {
        format!("{db:.1}").replace('-', "−")
    }
}

/// A meter's peak: −∞ only for digital silence, as digital meters show it.
fn meter_text(db: f32) -> String {
    if db <= opendap_audio_analysis::FLOOR_DBFS + 1.0 {
        "−∞".into()
    } else {
        format!("{db:.1}").replace('-', "−")
    }
}

/// A meter bar per channel: L and R for two, numbers otherwise. With nothing playing, two silent
/// bars, so the instrument keeps its size.
pub fn bars(levels: &[Level]) -> Vec<crate::MeterBar> {
    let silent = Level {
        peak_dbfs: opendap_audio_analysis::FLOOR_DBFS,
        rms_dbfs: opendap_audio_analysis::FLOOR_DBFS,
        clipped: 0,
        dc: 0.0,
    };
    let levels = if levels.is_empty() {
        &[silent, silent][..]
    } else {
        levels
    };
    let full_scale_db = 20.0 * CLIP.log10();
    levels
        .iter()
        .enumerate()
        .map(|(i, l)| crate::MeterBar {
            label: match (levels.len(), i) {
                (2, 0) => "L".into(),
                (2, 1) => "R".into(),
                (1, _) => "M".into(),
                _ => format!("{}", i + 1).into(),
            },
            rms: position(l.rms_dbfs),
            peak: position(l.peak_dbfs),
            value: meter_text(l.peak_dbfs).into(),
            clipped: l.clipped > 0 || l.peak_dbfs >= full_scale_db,
        })
        .collect()
}

/// What a reading says in words: a sine's analysis when the stream carries one, its loudest
/// frequency otherwise, and the clips.
pub fn readout(r: &Reading) -> String {
    if !r.active {
        return String::new();
    }
    let clips: u64 = r.levels.iter().map(|l| l.clipped).sum();
    let mut s = match r.tone {
        // Three short lines, so the readout keeps its size and never wraps mid-figure.
        Some(t) => format!(
            "Sine {:.1} Hz, {} dBFS\nTHD+N {} dB ({:.4} %)\nTHD {} dB · SNR {:.1} dB",
            t.frequency_hz,
            db_text(t.level_dbfs),
            db_text(t.thd_n_db),
            t.thd_n_percent,
            db_text(t.thd_db),
            t.snr_db
        ),
        None if r.peak_dbfs > SPECTRUM_FLOOR_DB => format!(
            "Loudest at {:.0} Hz, {} dBFS",
            r.peak_hz,
            db_text(r.peak_dbfs)
        ),
        None => "Silence".into(),
    };
    if clips > 0 {
        s.push_str(&format!("\n{clips} samples at full scale"));
    }
    s
}

/// The spectrum strip: one band per logical pixel ([`crate::sound::BANDS`] is the strip's width),
/// as a filled curve in `tint` over a grid (20 dB rows; 100 Hz,
/// 1 kHz and 10 kHz columns), at [`STRIP_W`] × [`STRIP_H`].
pub fn strip(bands: &[f32], tint: [u8; 3]) -> SharedPixelBuffer<Rgba8Pixel> {
    let (w, h) = (STRIP_W as usize, STRIP_H as usize);
    let mut px = SharedPixelBuffer::<Rgba8Pixel>::new(STRIP_W, STRIP_H);
    let buf = px.make_mut_slice();
    let grid = Rgba8Pixel {
        r: 255,
        g: 255,
        b: 255,
        a: 18,
    };
    for row in 1..7 {
        let y = row * h / 7;
        buf[y * w..(y + 1) * w].fill(grid);
    }
    for hz in [100.0f32, 1000.0, 10_000.0] {
        let x = ((hz / 20.0).log10() / 3.0 * w as f32) as usize;
        for y in 0..h {
            buf[y * w + x.min(w - 1)] = grid;
        }
    }
    if bands.is_empty() {
        return px;
    }
    let top = |db: f32| -> usize {
        (((db / SPECTRUM_FLOOR_DB).clamp(0.0, 1.0)) * (h - 1) as f32) as usize
    };
    let [r, g, b] = tint;
    for x in 0..w {
        // Each column shows its own band, never an interpolation that would flatten a peak.
        let db = bands[x * bands.len() / w];
        let y0 = top(db);
        for y in y0..h {
            // The curve's edge bright, its body fading downwards.
            let alpha = if y < y0 + 2 {
                255
            } else {
                (110 - (y - y0) * 70 / h) as u8
            };
            buf[y * w + x] = Rgba8Pixel { r, g, b, a: alpha };
        }
    }
    px
}

/// A history graph's pixels: twice the logical size, for a Retina display.
pub const GRAPH_W: u32 = 324;
pub const GRAPH_H: u32 = 96;

/// A task manager's graph: `values` (oldest first, the newest at the right edge, one per
/// [`crate::history::SPAN`] slot) as a filled line in `tint`, scaled to `max`, over a grid of
/// quarters; empty slots on the left are left blank.
pub fn graph(values: &[f32], max: f32, tint: [u8; 3]) -> SharedPixelBuffer<Rgba8Pixel> {
    let (w, h) = (GRAPH_W as usize, GRAPH_H as usize);
    let mut px = SharedPixelBuffer::<Rgba8Pixel>::new(GRAPH_W, GRAPH_H);
    let buf = px.make_mut_slice();
    let grid = Rgba8Pixel {
        r: 255,
        g: 255,
        b: 255,
        a: 16,
    };
    for q in 1..4 {
        let y = q * h / 4;
        buf[y * w..(y + 1) * w].fill(grid);
    }
    let span = crate::history::SPAN;
    if values.is_empty() || max <= 0.0 {
        return px;
    }
    let [r, g, b] = tint;
    let top =
        |v: f32| -> usize { ((1.0 - (v / max).clamp(0.0, 1.0)) * (h - 1) as f32).round() as usize };
    for x in 0..w {
        // The slot this column shows, counted from the right edge (the newest sample).
        let slot = x * span / w;
        let back = span - 1 - slot;
        let Some(i) = values.len().checked_sub(1 + back) else {
            continue;
        };
        let y0 = top(values[i]);
        for y in y0..h {
            let alpha = if y < y0 + 2 { 255 } else { 70 };
            buf[y * w + x] = Rgba8Pixel { r, g, b, a: alpha };
        }
    }
    px
}

/// The inferno colour map (matplotlib's), by nine stops, for `t` from 0 (black) to 1 (yellow).
pub fn inferno(t: f32) -> [u8; 3] {
    const STOPS: [[f32; 3]; 9] = [
        [0.0, 0.0, 4.0],
        [31.0, 12.0, 72.0],
        [85.0, 15.0, 109.0],
        [136.0, 34.0, 106.0],
        [186.0, 54.0, 85.0],
        [227.0, 89.0, 51.0],
        [249.0, 140.0, 10.0],
        [249.0, 201.0, 50.0],
        [252.0, 255.0, 164.0],
    ];
    let x = t.clamp(0.0, 1.0) * 8.0;
    let i = (x.floor() as usize).min(7);
    let f = x - i as f32;
    let (a, b) = (STOPS[i], STOPS[i + 1]);
    [0, 1, 2].map(|k| (a[k] + (b[k] - a[k]) * f).round() as u8)
}

/// A spectrogram that scrolls: each reading adds a column at the right edge (one every 100 ms,
/// so the strip holds the last 42 s), frequency up a log axis from 20 Hz to 20 kHz, level as colour
/// from [`SPECTRUM_FLOOR_DB`] (black) to full scale (yellow). Its buffer is fixed.
pub struct Waterfall {
    px: SharedPixelBuffer<Rgba8Pixel>,
}

impl Default for Waterfall {
    fn default() -> Self {
        let mut px = SharedPixelBuffer::<Rgba8Pixel>::new(STRIP_W, STRIP_H);
        px.make_mut_slice().fill(Rgba8Pixel {
            r: 0,
            g: 0,
            b: 4,
            a: 255,
        });
        Waterfall { px }
    }
}

impl Waterfall {
    /// One column: `rows` dBFS from the lowest frequency to the highest.
    pub fn push(&mut self, rows: &[f32]) {
        let (w, h) = (STRIP_W as usize, STRIP_H as usize);
        let buf = self.px.make_mut_slice();
        for y in 0..h {
            buf.copy_within(y * w + 1..(y + 1) * w, y * w);
            let db = rows.get(h - 1 - y).copied().unwrap_or(SPECTRUM_FLOOR_DB);
            let [r, g, b] = inferno(1.0 - db / SPECTRUM_FLOOR_DB);
            buf[(y + 1) * w - 1] = Rgba8Pixel { r, g, b, a: 255 };
        }
    }

    pub fn image(&self) -> SharedPixelBuffer<Rgba8Pixel> {
        self.px.clone()
    }
}

/// The Energy group's words, from the simulated hardware's power.
pub struct Energy {
    pub level: String,
    pub power: String,
    pub status: String,
    pub voltage: String,
    pub current: String,
    pub temperature: String,
    pub time: String,
    pub charger: String,
}

fn duration(s: u32) -> String {
    format!("{} h {:02} min", s / 3600, s % 3600 / 60)
}

pub fn energy(p: &stats::Power) -> Energy {
    let charging = p.current_a > 0.0;
    Energy {
        level: format!("{:.0} %", p.capacity_percent),
        power: format!("{}{:.2} W", if charging { "+" } else { "" }, p.watts()).replace('-', "−"),
        status: [p.status.as_str(), p.health.as_str(), p.technology.as_str()]
            .iter()
            .filter(|x| !x.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(" · "),
        voltage: format!("{:.3} V", p.voltage_v),
        current: format!("{:.3} A", p.current_a).replace('-', "−"),
        temperature: format!("{:.1} °C", p.temperature_c),
        time: match (charging, p.time_to_empty_s, p.time_to_full_s) {
            (true, _, Some(t)) => format!("{} to full", duration(t)),
            (false, Some(t), _) => format!("{} left", duration(t)),
            _ => "—".into(),
        },
        charger: match (p.usb_online, p.ac_online) {
            (true, true) => "USB-C and AC".into(),
            (true, false) => "USB-C".into(),
            (false, true) => "AC".into(),
            (false, false) => "None".into(),
        },
    }
}

/// The load's words and bars, from a `top` frame.
pub struct Load {
    pub cpu: String,
    pub cpu_busy: f32,
    pub mem: String,
    pub mem_busy: f32,
    pub load: String,
    pub uptime: String,
}

pub fn load(f: &stats::Frame) -> Load {
    let used = f.mem_total_mib - f.mem_available_mib;
    Load {
        cpu: format!("{:.0} %", f.cpu_percent),
        cpu_busy: (f.cpu_percent / 100.0).clamp(0.0, 1.0),
        mem: format!("{:.0} of {:.0} MiB in use", used, f.mem_total_mib),
        mem_busy: if f.mem_total_mib > 0.0 {
            (used / f.mem_total_mib).clamp(0.0, 1.0)
        } else {
            0.0
        },
        load: format!("{:.2} · {:.2} · {:.2}", f.load[0], f.load[1], f.load[2]),
        uptime: f.uptime.clone(),
    }
}

/// A process's state, in words.
pub fn state_word(c: char) -> &'static str {
    match c {
        'R' => "Running",
        'S' => "Sleeping",
        'D' => "Waiting",
        'T' | 't' => "Stopped",
        'Z' => "Zombie",
        'I' => "Idle",
        _ => "Unknown",
    }
}

pub fn task_row(p: &stats::Process) -> crate::TaskRow {
    crate::TaskRow {
        pid: p.pid as i32,
        name: p.name.clone().into(),
        user: p.user.clone().into(),
        state: state_word(p.state).into(),
        nice: p.nice,
        cpu: format!("{:.1} %", p.cpu_percent).into(),
        mem: format!("{:.1} MiB", p.resident_mib).into(),
        busy: (p.cpu_percent / 100.0).clamp(0.0, 1.0),
        kernel: p.kernel,
    }
}

/// The Tasks list: the frame's processes, kernel threads only when asked for, sorted by `by`
/// ("name", "pid", "state", "cpu", "mem"); ties by PID, so rows do not jump between frames.
pub fn tasks(f: &stats::Frame, by: &str, descending: bool, kernel: bool) -> Vec<stats::Process> {
    let mut list: Vec<stats::Process> = f
        .processes
        .iter()
        .filter(|p| kernel || !p.kernel)
        .cloned()
        .collect();
    list.sort_by(|a, b| {
        let o = match by {
            "name" => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            "state" => state_word(a.state).cmp(state_word(b.state)),
            "cpu" => a.cpu_percent.total_cmp(&b.cpu_percent),
            "mem" => a.resident_mib.total_cmp(&b.resident_mib),
            _ => a.pid.cmp(&b.pid),
        };
        (if descending { o.reverse() } else { o }).then(a.pid.cmp(&b.pid))
    });
    list
}

/// The Tasks tab's first line.
pub fn tasks_summary(f: &stats::Frame) -> String {
    let user = f.processes.iter().filter(|p| !p.kernel).count();
    format!(
        "{} processes, {} kernel threads · load {:.2} · up {}",
        user,
        f.processes.len() - user,
        f.load[0],
        f.uptime
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tasks_sort_and_filter() {
        let p = |pid: u32, name: &str, cpu: f32, kernel: bool| stats::Process {
            pid,
            name: name.into(),
            cpu_percent: cpu,
            kernel,
            state: 'S',
            ..Default::default()
        };
        let f = stats::Frame {
            processes: vec![
                p(3, "b", 1.0, false),
                p(2, "kworker", 0.0, true),
                p(1, "A", 1.0, false),
            ],
            ..Default::default()
        };
        let by = |k: &str, d: bool, kern: bool| {
            tasks(&f, k, d, kern)
                .iter()
                .map(|x| x.pid)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            by("cpu", true, false),
            [1, 3],
            "ties by PID, kernel threads hidden"
        );
        assert_eq!(by("name", false, true), [1, 3, 2]);
        assert_eq!(by("pid", true, true), [3, 2, 1]);
        assert_eq!(
            tasks_summary(&f),
            "2 processes, 1 kernel threads · load 0.00 · up "
        );
    }

    #[test]
    fn a_graph_puts_the_newest_sample_at_the_right_edge() {
        let mut v = vec![0.0f32; 10];
        v[9] = 100.0;
        let px = graph(&v, 100.0, [59, 130, 196]);
        let s = px.as_slice();
        let (w, h) = (GRAPH_W as usize, GRAPH_H as usize);
        assert_eq!(
            s[w - 1].a,
            255,
            "the newest, at full scale, reaches the top right"
        );
        assert_eq!(
            s[(h - 1) * w].a,
            0,
            "slots before the first sample stay blank"
        );
    }

    #[test]
    fn the_waterfall_scrolls_left_and_keeps_its_size() {
        let mut wf = Waterfall::default();
        let mut rows = vec![SPECTRUM_FLOOR_DB; STRIP_H as usize];
        rows[0] = 0.0; // the lowest row, full scale
        wf.push(&rows);
        let (w, h) = (STRIP_W as usize, STRIP_H as usize);
        let s = wf.image();
        let bottom_right = s.as_slice()[(h - 1) * w + w - 1];
        assert_eq!(
            (bottom_right.r, bottom_right.g, bottom_right.b),
            (252, 255, 164),
            "full scale is yellow, low frequencies at the bottom"
        );
        wf.push(&vec![SPECTRUM_FLOOR_DB; h]);
        let moved = wf.image().as_slice()[(h - 1) * w + w - 2];
        assert_eq!(moved.r, 252, "it moved one column left");
        assert_eq!(
            (wf.image().width(), wf.image().height()),
            (STRIP_W, STRIP_H)
        );
    }

    #[test]
    fn energy_words() {
        let p = stats::Power {
            capacity_percent: 50.0,
            status: "Discharging".into(),
            health: "Good".into(),
            voltage_v: 3.3,
            current_a: -1.6,
            temperature_c: 26.0,
            time_to_empty_s: Some(3600),
            usb_online: true,
            ..Default::default()
        };
        let e = energy(&p);
        assert_eq!(
            (
                e.power.as_str(),
                e.current.as_str(),
                e.time.as_str(),
                e.charger.as_str()
            ),
            ("−5.28 W", "−1.600 A", "1 h 00 min left", "USB-C")
        );
    }

    #[test]
    fn meter_positions() {
        assert_eq!(position(0.0), 1.0);
        assert_eq!(position(-30.0), 0.5);
        assert_eq!(position(-240.0), 0.0);
    }

    #[test]
    fn bars_name_their_channels_and_mark_clips() {
        let l = Level {
            peak_dbfs: -6.0,
            rms_dbfs: -9.0,
            clipped: 0,
            dc: 0.0,
        };
        let b = bars(&[l, Level { clipped: 3, ..l }]);
        assert_eq!((b[0].label.as_str(), b[1].label.as_str()), ("L", "R"));
        assert_eq!(b[0].value.as_str(), "−6.0");
        assert!(!b[0].clipped && b[1].clipped);
    }

    #[test]
    fn silence_keeps_two_bars() {
        let b = bars(&[]);
        assert_eq!(b.len(), 2);
        assert_eq!((b[0].value.as_str(), b[0].rms), ("−∞", 0.0));
    }

    #[test]
    fn readouts() {
        assert_eq!(readout(&Reading::default()), "");
        let silent = Reading {
            active: true,
            peak_dbfs: -240.0,
            ..Default::default()
        };
        assert_eq!(readout(&silent), "Silence");
        let loud = Reading {
            active: true,
            peak_hz: 440.4,
            peak_dbfs: -12.04,
            ..Default::default()
        };
        assert_eq!(readout(&loud), "Loudest at 440 Hz, −12.0 dBFS");
    }

    #[test]
    fn the_strip_draws_a_peak_where_its_band_is() {
        let mut bands = vec![-140.0f32; 212];
        bands[106] = 0.0;
        let px = strip(&bands, [59, 130, 196]);
        let s = px.as_slice();
        let (w, h) = (STRIP_W as usize, STRIP_H as usize);
        // The curve's top in each column: its first opaque pixel of the tint.
        let tops: Vec<usize> = (0..w)
            .map(|x| {
                (0..h)
                    .find(|&y| s[y * w + x].a == 255 && s[y * w + x].b == 196)
                    .unwrap_or(h)
            })
            .collect();
        let (x, &y) = tops.iter().enumerate().min_by_key(|(_, y)| **y).unwrap();
        assert!(y <= 2, "the peak reaches the top: {y}");
        let expected = 106.5 / 212.0 * w as f32;
        assert!(
            (x as f32 - expected).abs() <= 3.0,
            "at column {x}, its band's centre {expected}"
        );
        assert!(tops[10] >= h - 2, "elsewhere the curve lies at the floor");
    }
}
