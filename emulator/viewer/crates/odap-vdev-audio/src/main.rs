//! odap-vdev-audio: a virtual device's sound, recorded as the guest plays it and measured.
//!
//! The recording comes from QEMU's D-Bus audio (virtual-devices ADR 0003), the PCM the guest sent
//! its sound card, in the guest's own format, with nothing of the host in between: what it
//! measures is the device's software, not the host's audio.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use opendap_audio_analysis::spectrum::{self, Spectrum};
use opendap_audio_analysis::{
    Format, amplitude_db, compare, decode, decode_exact, power_db, tone, wav,
};
use opendap_qemu_display::audio::{self, Sink};

const USAGE: &str = "usage:
  odap-vdev-audio record --qmp SOCKET [--direction out|in] [--seconds S] [--wait S] [--run CMD] --out FILE.wav
  odap-vdev-audio analyze FILE.wav [--from S] [--to S] [--bands N] [--blocks S] [--json]
  odap-vdev-audio compare REFERENCE.wav CAPTURE.wav [--json]
  odap-vdev-audio spectrum FILE.wav --out FILE.png [--from S] [--to S]
  odap-vdev-audio spectrogram FILE.wav --out FILE.png [--from S] [--to S] [--channel N]

  record    the guest's output, exactly as the guest played it (or, --direction in, what it was
            given to record), from its first sound for --seconds (default 5); --run starts CMD
            (sh -c) once listening, to make the guest play; gives up after --wait seconds
            (default 30) of nothing. With the simulator window open it asks the window (its
            view.sock, beside the QMP socket): QEMU's D-Bus display serves one client, and a
            second would displace the window. Without it, it attaches to QEMU itself (output
            only). QMP serves one client per socket: use odap-vdev's (status: qmp).
  analyze   format; per channel the sample peak, RMS, DC and samples at full scale; the loudest
            frequency; a sine's frequency, level, THD, THD+N (20 Hz-20 kHz) and SNR when the file
            carries one (on --from..--to, default its middle); --bands N log bands from 20 Hz to
            20 kHz (dBFS, the highest bin in each); --blocks S analyses the whole file S seconds at
            a time and reports the worst block (a dropout, a click or a phase jump in a continuous
            sine shows as one block's THD+N far above the others) and the longest run of exact
            zeros.
  compare   whether CAPTURE carries REFERENCE bit for bit (past leading silence and a late start):
            the alignment, the frames compared, the first difference and its size in LSB. Sound
            before the reference begins (another stream's tail the sound path held back) is found
            and reported as leading frames; the comparison then starts at the reference's own
            start, and the exit is non-zero.
  spectrum  the averaged spectrum as a PNG (1200 x 480: 20 Hz to Nyquist on a log axis, 0 to
            -160 dBFS, lines every decade and every 20 dB), for a person or an agent to look at.
  spectrogram  time across (1600 px for the whole file or --from..--to), 20 Hz to Nyquist up a log
            axis (600 px, lines at 100 Hz, 1 kHz, 10 kHz), level as colour (0 dBFS bright yellow
            to -160 dBFS black, inferno). Each column is the loudest of every 4096-point
            transform in its span, so a click of a millisecond in ten minutes still shows: a
            dropout or a click is a vertical line, a spur a horizontal one. Mono mix, or
            --channel N (from 1).";

fn fail(msg: impl std::fmt::Display) -> ExitCode {
    eprintln!("odap-vdev-audio: {msg}");
    ExitCode::FAILURE
}

/// `--name value` from the arguments.
fn opt(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

fn flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

/// The positional arguments (those not an option or its value).
fn positional(args: &[String]) -> Vec<String> {
    let valued = [
        "--qmp",
        "--direction",
        "--seconds",
        "--wait",
        "--run",
        "--out",
        "--from",
        "--to",
        "--bands",
        "--blocks",
        "--channel",
    ];
    let mut out = Vec::new();
    let mut skip = false;
    for a in args {
        if skip {
            skip = false;
        } else if valued.contains(&a.as_str()) {
            skip = true;
        } else if !a.starts_with("--") {
            out.push(a.clone());
        }
    }
    out
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest = args.get(1..).unwrap_or_default();
    let r = match args.first().map(String::as_str) {
        Some("record") => record(rest),
        Some("analyze") => analyze(rest),
        Some("compare") => compare_files(rest),
        Some("spectrum") => spectrum_png(rest),
        Some("spectrogram") => spectrogram_png(rest),
        _ => Err(USAGE.to_string()),
    };
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fail(e),
    }
}

enum Msg {
    Init(Format),
    Data(Vec<u8>),
}

struct Rec(Mutex<mpsc::Sender<Msg>>);

impl Sink for Rec {
    fn init(&self, _: u64, f: audio::Format) {
        let f = Format {
            rate: f.rate,
            channels: f.channels,
            bits: f.bits,
            signed: f.signed,
            float: f.float,
            big_endian: f.big_endian,
        };
        let _ = self
            .0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .send(Msg::Init(f));
    }
    fn fini(&self, _: u64) {}
    fn enabled(&self, _: u64, _: bool) {}
    fn write(&self, _: u64, data: &[u8]) {
        let _ = self
            .0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .send(Msg::Data(data.to_vec()));
    }
}

fn record(args: &[String]) -> Result<(), String> {
    let qmp = opt(args, "--qmp").ok_or("record needs --qmp SOCKET")?;
    let out = PathBuf::from(opt(args, "--out").ok_or("record needs --out FILE.wav")?);
    let seconds: f64 = opt(args, "--seconds")
        .map_or(Ok(5.0), |s| s.parse())
        .map_err(|_| "--seconds: a number")?;
    let wait: f64 = opt(args, "--wait")
        .map_or(Ok(30.0), |s| s.parse())
        .map_err(|_| "--wait: a number")?;
    let direction = opt(args, "--direction").unwrap_or_else(|| "out".into());
    let window = Path::new(&qmp).with_file_name("view.sock");
    if let Ok(stream) = std::os::unix::net::UnixStream::connect(&window) {
        return record_through_window(stream, args, &direction, &out, seconds, wait);
    }
    if direction != "out" {
        return Err(
            "recording the input needs the simulator window (it supplies the input)".into(),
        );
    }
    let (tx, rx) = mpsc::channel();
    let session = opendap_qemu_display::connect(Path::new(&qmp)).map_err(|e| e.to_string())?;
    let _listening = session
        .audio_out(Arc::new(Rec(Mutex::new(tx))))
        .map_err(|e| format!("{e} (is the device started with the D-Bus audio?)"))?;
    let mut child = match opt(args, "--run") {
        Some(cmd) => Some(
            Command::new("sh")
                .args(["-c", &cmd])
                .spawn()
                .map_err(|e| format!("--run: {e}"))?,
        ),
        None => None,
    };
    let deadline = Instant::now() + Duration::from_secs_f64(wait);
    // QEMU announces the stream when the listener registers, and again when the guest reopens it.
    let mut format: Option<Format> = None;
    let mut file: Option<BufWriter<File>> = None;
    let (mut written, mut wanted) = (0usize, 0usize);
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let msg = match rx.recv_timeout(if file.is_some() {
            Duration::from_secs(5)
        } else {
            left
        }) {
            Ok(m) => m,
            Err(_) if file.is_none() => return Err(format!("nothing played in {wait} s")),
            Err(_) => break, // the guest stopped playing: what it played is kept
        };
        match msg {
            Msg::Init(f) => {
                if file.is_some() && format != Some(f) {
                    eprintln!(
                        "odap-vdev-audio: the guest changed its format ({}): stopped there",
                        f.describe()
                    );
                    break;
                }
                format = Some(f);
            }
            Msg::Data(d) => {
                let Some(f) = format else { continue };
                if file.is_none() {
                    // From the first sound: leading digital silence is not recorded.
                    if d.iter().all(|&b| b == 0) {
                        continue;
                    }
                    let mut w = BufWriter::new(
                        File::create(&out).map_err(|e| format!("{}: {e}", out.display()))?,
                    );
                    wav::write_header(&mut w, &f, 0).map_err(|e| e.to_string())?;
                    file = Some(w);
                    wanted = (seconds * f64::from(f.rate)) as usize * f.bytes_per_frame();
                }
                let take = d.len().min(wanted - written);
                file.as_mut()
                    .unwrap()
                    .write_all(&d[..take])
                    .map_err(|e| e.to_string())?;
                written += take;
                if written >= wanted {
                    break;
                }
            }
        }
    }
    if let Some(c) = child.as_mut() {
        let _ = c.wait();
    }
    let (Some(w), Some(f)) = (file, format) else {
        return Err("nothing recorded".into());
    };
    let mut f_out = w.into_inner().map_err(|e| e.to_string())?;
    wav::finalize(&mut f_out, &f, written as u32).map_err(|e| e.to_string())?;
    let secs = written as f64 / f.bytes_per_frame() as f64 / f64::from(f.rate);
    println!("{} ({}, {secs:.2} s)", out.display(), f.describe());
    Ok(())
}

/// `record` asked of the simulator window over its control socket.
fn record_through_window(
    stream: std::os::unix::net::UnixStream,
    args: &[String],
    direction: &str,
    out: &Path,
    seconds: f64,
    wait: f64,
) -> Result<(), String> {
    use std::io::{BufRead, BufReader};
    let out = std::path::absolute(out).map_err(|e| e.to_string())?;
    let req =
        serde_json::json!({ "record": direction, "seconds": seconds, "wait": wait, "path": out });
    let mut w = &stream;
    writeln!(w, "{req}").map_err(|e| e.to_string())?;
    let mut lines = BufReader::new(&stream).lines();
    let mut next = || -> Result<serde_json::Value, String> {
        let line = lines
            .next()
            .ok_or("the window closed the connection")?
            .map_err(|e| e.to_string())?;
        serde_json::from_str(&line).map_err(|e| e.to_string())
    };
    let first = next()?;
    if let Some(e) = first["error"].as_str() {
        return Err(e.to_string());
    }
    let mut child = match opt(args, "--run") {
        Some(cmd) => Some(
            Command::new("sh")
                .args(["-c", &cmd])
                .spawn()
                .map_err(|e| format!("--run: {e}"))?,
        ),
        None => None,
    };
    let last = next()?;
    if let Some(c) = child.as_mut() {
        let _ = c.wait();
    }
    if let Some(e) = last["error"].as_str() {
        return Err(e.to_string());
    }
    println!(
        "{} ({}, {:.2} s)",
        out.display(),
        last["format"].as_str().unwrap_or("?"),
        last["seconds"].as_f64().unwrap_or(0.0)
    );
    Ok(())
}

fn load(path: &str) -> Result<(Format, Vec<u8>), String> {
    let mut f = File::open(path).map_err(|e| format!("{path}: {e}"))?;
    wav::read(&mut f).map_err(|e| format!("{path}: {e}"))
}

/// The frames of `--from`..`--to` (seconds), or the middle of the file (up to 2^18 frames).
fn segment(args: &[String], f: &Format, samples: &[f32]) -> (usize, usize) {
    let ch = usize::from(f.channels);
    let frames = samples.len() / ch;
    let at = |name: &str| {
        opt(args, name)
            .and_then(|s| s.parse::<f64>().ok())
            .map(|s| ((s * f64::from(f.rate)) as usize).min(frames))
    };
    match (at("--from"), at("--to")) {
        (None, None) => {
            let n = frames.min(tone::MAX_SAMPLES);
            let start = (frames - n) / 2;
            (start, start + n)
        }
        (a, b) => (a.unwrap_or(0), b.unwrap_or(frames).max(a.unwrap_or(0))),
    }
}

/// An averaged spectrum over the frames (each transform half overlapping the last).
fn averaged(f: &Format, mono: &[f32]) -> Spectrum {
    let size = (mono.len().next_power_of_two() / 2).clamp(1024, 65_536);
    let mut s = Spectrum::new(f.rate, size);
    s.smoothing(0.2);
    let mut at = 0;
    while at + size <= mono.len() {
        s.feed(&mono[at..at + size]);
        s.compute();
        at += size / 2;
    }
    if at == 0 {
        s.feed(mono);
        s.compute();
    }
    s
}

fn analyze(args: &[String]) -> Result<(), String> {
    let file = positional(args)
        .first()
        .cloned()
        .ok_or("analyze needs FILE.wav")?;
    let (f, data) = load(&file)?;
    let mut samples = Vec::new();
    decode(&f, &data, &mut samples);
    let ch = usize::from(f.channels);
    let (a, b) = segment(args, &f, &samples);
    let seg = &samples[a * ch..b * ch];
    let mut channels = Vec::new();
    for c in 0..ch {
        let mut x = Vec::new();
        spectrum::channel(seg, ch, c, &mut x);
        let peak = x.iter().fold(0f32, |m, v| m.max(v.abs()));
        let ms =
            x.iter().map(|v| f64::from(*v) * f64::from(*v)).sum::<f64>() / x.len().max(1) as f64;
        let dc = x.iter().map(|v| f64::from(*v)).sum::<f64>() / x.len().max(1) as f64;
        let clipped = x
            .iter()
            .filter(|v| v.abs() >= opendap_audio_analysis::meter::CLIP)
            .count();
        let t = tone::analyze(&x, f.rate);
        channels.push(serde_json::json!({
            "peak_dbfs": amplitude_db(f64::from(peak)),
            "rms_dbfs": power_db(ms),
            "dc": dc,
            "clipped": clipped,
            "tone": t.map(|t| serde_json::json!({
                "frequency_hz": t.frequency_hz, "level_dbfs": t.level_dbfs,
                "thd_percent": t.thd_percent, "thd_db": t.thd_db,
                "thd_n_percent": t.thd_n_percent, "thd_n_db": t.thd_n_db, "snr_db": t.snr_db,
            })),
        }));
    }
    let mut mono = Vec::new();
    spectrum::mono(seg, ch, &mut mono);
    let s = averaged(&f, &mono);
    let (peak_hz, peak_db) = s.peak();
    let bands: usize = opt(args, "--bands")
        .and_then(|n| n.parse().ok())
        .unwrap_or(0);
    let report = serde_json::json!({
        "file": file,
        "format": f.describe(),
        "seconds": samples.len() as f64 / ch as f64 / f64::from(f.rate),
        "segment_s": [a as f64 / f64::from(f.rate), b as f64 / f64::from(f.rate)],
        "channels": channels,
        "loudest": { "frequency_hz": peak_hz, "dbfs": peak_db },
        "spectrum_bin_hz": s.bin_hz(),
        "bands": (bands > 0).then(|| s.bands(bands, 20.0, 20_000.0)),
    });
    let mut report = report;
    if let Some(block_s) = opt(args, "--blocks").and_then(|v| v.parse::<f64>().ok()) {
        report["blocks"] = blocks(&f, &samples, block_s);
    }
    if flag(args, "--json") {
        println!("{report:#}");
        return Ok(());
    }
    if let Some(b) = report.get("blocks") {
        println!("{b:#}");
    }
    println!(
        "{}  {}  {:.2} s (measured {:.2}–{:.2} s)",
        file,
        f.describe(),
        report["seconds"].as_f64().unwrap_or(0.0),
        report["segment_s"][0].as_f64().unwrap_or(0.0),
        report["segment_s"][1].as_f64().unwrap_or(0.0)
    );
    for (i, c) in channels.iter().enumerate() {
        print!(
            "  ch {}: peak {:.2} dBFS, RMS {:.2} dBFS, DC {:+.2e}, {} at full scale",
            i + 1,
            c["peak_dbfs"].as_f64().unwrap_or(0.0),
            c["rms_dbfs"].as_f64().unwrap_or(0.0),
            c["dc"].as_f64().unwrap_or(0.0),
            c["clipped"]
        );
        match c["tone"].as_object() {
            Some(t) => println!(
                "\n        sine {:.3} Hz at {:.2} dBFS: THD {:.2} dB ({:.5} %), THD+N {:.2} dB ({:.5} %), SNR {:.2} dB",
                t["frequency_hz"].as_f64().unwrap_or(0.0),
                t["level_dbfs"].as_f64().unwrap_or(0.0),
                t["thd_db"].as_f64().unwrap_or(0.0),
                t["thd_percent"].as_f64().unwrap_or(0.0),
                t["thd_n_db"].as_f64().unwrap_or(0.0),
                t["thd_n_percent"].as_f64().unwrap_or(0.0),
                t["snr_db"].as_f64().unwrap_or(0.0)
            ),
            None => println!("; no sine"),
        }
    }
    println!(
        "  loudest: {peak_hz:.1} Hz at {peak_db:.2} dBFS (bins of {:.2} Hz)",
        s.bin_hz()
    );
    if bands > 0 {
        println!(
            "  bands (20 Hz–20 kHz, dBFS): {:?}",
            s.bands(bands, 20.0, 20_000.0)
                .iter()
                .map(|d| (d * 10.0).round() / 10.0)
                .collect::<Vec<_>>()
        );
    }
    Ok(())
}

/// The whole file `block_s` seconds at a time, per channel: each block's sine analysis, the worst
/// and best THD+N, and the longest run of exact zero samples (a dropout the sender filled).
fn blocks(f: &Format, samples: &[f32], block_s: f64) -> serde_json::Value {
    let ch = usize::from(f.channels);
    let n = ((block_s * f64::from(f.rate)) as usize).max(tone::MIN_SAMPLES);
    let frames = samples.len() / ch;
    let mut per_channel = Vec::new();
    for c in 0..ch {
        let mut x = Vec::new();
        spectrum::channel(samples, ch, c, &mut x);
        let (mut worst, mut best, mut no_tone) = ((f32::MIN, 0usize), (f32::MAX, 0usize), 0usize);
        let mut count = 0;
        for (i, b) in x.chunks_exact(n).enumerate() {
            count += 1;
            match tone::analyze(b, f.rate) {
                Some(t) => {
                    if t.thd_n_db > worst.0 {
                        worst = (t.thd_n_db, i);
                    }
                    if t.thd_n_db < best.0 {
                        best = (t.thd_n_db, i);
                    }
                }
                None => no_tone += 1,
            }
        }
        let (mut run, mut longest) = (0usize, 0usize);
        for v in &x {
            run = if *v == 0.0 { run + 1 } else { 0 };
            longest = longest.max(run);
        }
        per_channel.push(serde_json::json!({
            "blocks": count,
            "blocks_without_a_sine": no_tone,
            "worst_thd_n_db": (worst.0 > f32::MIN).then_some(worst.0),
            "worst_block_s": worst.1 as f64 * block_s,
            "best_thd_n_db": (best.0 < f32::MAX).then_some(best.0),
            "longest_zero_run_ms": longest as f64 * 1000.0 / f64::from(f.rate),
        }));
    }
    serde_json::json!({ "block_s": block_s, "frames": frames, "channels": per_channel })
}

fn compare_files(args: &[String]) -> Result<(), String> {
    let p = positional(args);
    let (Some(rp), Some(cp)) = (p.first(), p.get(1)) else {
        return Err("compare needs REFERENCE.wav CAPTURE.wav".into());
    };
    let (rf, rd) = load(rp)?;
    let (cf, cd) = load(cp)?;
    let mut fmt_note = String::new();
    if (rf.rate, rf.channels, rf.bits, rf.float) != (cf.rate, cf.channels, cf.bits, cf.float) {
        fmt_note = format!(
            "formats differ: {} against {}",
            rf.describe(),
            cf.describe()
        );
    }
    let (mut r, mut c) = (Vec::new(), Vec::new());
    decode_exact(&rf, &rd, &mut r).ok_or("the reference is float: bit-exact compares integers")?;
    decode_exact(&cf, &cd, &mut c).ok_or("the capture is float: bit-exact compares integers")?;
    let ch = usize::from(rf.channels.max(1));
    let frames = r.len() / ch;
    let mut result = if fmt_note.is_empty() {
        compare::bit_exact(&r, &c, ch, frames)
    } else {
        None
    };
    // The capture may begin with sound that is not the reference's (another stream's tail held
    // back by the sound path): find the reference's own start in the capture, compare from there,
    // and report what came before it.
    let mut leading = 0usize;
    if result.is_none() && fmt_note.is_empty() {
        let bpf = rf.bytes_per_frame();
        let first = rd
            .chunks_exact(bpf)
            .position(|f| f.iter().any(|&b| b != 0))
            .unwrap_or(0);
        let probe = &rd[first * bpf..((first + 256) * bpf).min(rd.len())];
        let found = cd
            .windows(probe.len().max(1))
            .enumerate()
            .find(|(i, w)| i % bpf == 0 && *w == probe)
            .map(|(i, _)| i / bpf);
        if let Some(k) = found {
            leading = k.saturating_sub(first);
            result = compare::bit_exact(&r[first * ch..], &c[k * ch..], ch, 0).map(|mut x| {
                x.offset += first;
                x.capture_start = k;
                x
            });
        }
    }
    let report = serde_json::json!({
        "reference": rp, "capture": cp,
        "format": rf.describe(),
        "exact": result.is_some_and(|x| x.exact),
        "aligned": result.is_some(),
        "note": fmt_note,
        "offset_frames": result.map(|x| x.offset),
        "capture_start_frames": result.map(|x| x.capture_start),
        "compared_frames": result.map(|x| x.compared),
        "reference_frames": frames,
        "first_mismatch_frame": result.and_then(|x| x.first_mismatch),
        "max_lsb": result.map(|x| x.max_lsb),
        "leading_frames": leading,
    });
    if flag(args, "--json") {
        println!("{report:#}");
    } else {
        match result {
            Some(x) if x.exact => {
                println!(
                    "bit-exact: {} of the reference's {} frames ({}), from reference frame {}",
                    x.compared,
                    frames,
                    rf.describe(),
                    x.offset
                );
                if leading > 0 {
                    println!(
                        "but {leading} frames ({:.1} ms) of other sound come before it (another stream's tail?)",
                        leading as f64 * 1000.0 / f64::from(rf.rate)
                    );
                }
            }
            Some(x) => println!(
                "NOT bit-exact: first difference at frame {} of {} compared, up to {} LSB",
                x.first_mismatch.unwrap_or(0),
                x.compared,
                x.max_lsb
            ),
            None if !fmt_note.is_empty() => println!("NOT bit-exact: {fmt_note}"),
            None => println!(
                "NOT bit-exact: the capture's first sound is nowhere in the reference (changed from its start: a gain, a resampler, dither)"
            ),
        }
    }
    if result.is_some_and(|x| x.exact) && leading == 0 {
        Ok(())
    } else if result.is_some_and(|x| x.exact) {
        Err("bit-exact, after sound that is not the reference's".into())
    } else {
        Err("not bit-exact".into())
    }
}

fn spectrum_png(args: &[String]) -> Result<(), String> {
    let file = positional(args)
        .first()
        .cloned()
        .ok_or("spectrum needs FILE.wav")?;
    let out = opt(args, "--out").ok_or("spectrum needs --out FILE.png")?;
    let (f, data) = load(&file)?;
    let mut samples = Vec::new();
    decode(&f, &data, &mut samples);
    let ch = usize::from(f.channels);
    let (a, b) = segment(args, &f, &samples);
    let mut mono = Vec::new();
    spectrum::mono(&samples[a * ch..b * ch], ch, &mut mono);
    let s = averaged(&f, &mono);
    let (w, h) = (1200usize, 480usize);
    let mut rgb = vec![17u8; w * h * 3];
    let nyquist = f.rate as f32 / 2.0;
    let decades = (nyquist / 20.0).log10();
    let x_of = |hz: f32| ((hz / 20.0).log10() / decades * (w - 1) as f32).round() as isize;
    let y_of = |db: f32| ((db / -160.0).clamp(0.0, 1.0) * (h - 1) as f32).round() as usize;
    let mut put = |x: usize, y: usize, c: [u8; 3]| {
        let i = (y * w + x) * 3;
        rgb[i..i + 3].copy_from_slice(&c);
    };
    for db in (0..=160).step_by(20) {
        let y = y_of(-(db as f32));
        (0..w).for_each(|x| put(x, y, [45, 49, 54]));
    }
    for hz in [20.0f32, 100.0, 1000.0, 10_000.0, 100_000.0]
        .into_iter()
        .filter(|&hz| hz <= nyquist)
    {
        let x = x_of(hz).clamp(0, w as isize - 1) as usize;
        (0..h).for_each(|y| put(x, y, [45, 49, 54]));
    }
    let bins = s.bins();
    let mut prev: Option<(usize, usize)> = None;
    for x in 0..w {
        let hz = 20.0 * 10f32.powf(x as f32 / (w - 1) as f32 * decades);
        let k = ((hz / s.bin_hz()).round() as usize).min(bins.len() - 1);
        // The highest bin under this pixel, so a peak is never missed.
        let k2 = (((20.0 * 10f32.powf((x + 1) as f32 / (w - 1) as f32 * decades)) / s.bin_hz())
            .round() as usize)
            .clamp(k, bins.len() - 1);
        let db = bins[k..=k2].iter().copied().fold(f32::MIN, f32::max);
        let y = y_of(db);
        let (lo, hi) = match prev {
            Some((_, py)) => (y.min(py), y.max(py)),
            None => (y, y),
        };
        (lo..=hi).for_each(|yy| put(x, yy, [91, 160, 230]));
        prev = Some((x, y));
    }
    let file_out = File::create(&out).map_err(|e| format!("{out}: {e}"))?;
    let mut enc = png::Encoder::new(BufWriter::new(file_out), w as u32, h as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .and_then(|mut wr| wr.write_image_data(&rgb))
        .map_err(|e| e.to_string())?;
    println!(
        "{out}: 20 Hz to {:.0} Hz (log; lines at 20, 100, 1k, 10k), 0 to -160 dBFS (lines every 20 dB), bins of {:.2} Hz",
        nyquist,
        s.bin_hz()
    );
    Ok(())
}

/// The inferno colour map (matplotlib's), by nine stops, for `t` from 0 (black) to 1 (yellow).
fn inferno(t: f32) -> [u8; 3] {
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
    let (i, f) = ((x.floor() as usize).min(7), x - x.floor().min(7.0));
    let (a, b) = (STOPS[i], STOPS[i + 1]);
    [0, 1, 2].map(|k| (a[k] + (b[k] - a[k]) * f).round() as u8)
}

fn spectrogram_png(args: &[String]) -> Result<(), String> {
    let file = positional(args)
        .first()
        .cloned()
        .ok_or("spectrogram needs FILE.wav")?;
    let out = opt(args, "--out").ok_or("spectrogram needs --out FILE.png")?;
    let (f, data) = load(&file)?;
    let mut samples = Vec::new();
    decode(&f, &data, &mut samples);
    let ch = usize::from(f.channels);
    let frames = samples.len() / ch;
    let at = |name: &str| {
        opt(args, name)
            .and_then(|s| s.parse::<f64>().ok())
            .map(|s| ((s * f64::from(f.rate)) as usize).min(frames))
    };
    let (a, b) = (at("--from").unwrap_or(0), at("--to").unwrap_or(frames));
    let mut x = Vec::new();
    match opt(args, "--channel").and_then(|c| c.parse::<usize>().ok()) {
        Some(c) if (1..=ch).contains(&c) => {
            spectrum::channel(&samples[a * ch..b * ch], ch, c - 1, &mut x)
        }
        Some(_) => return Err(format!("--channel: 1 to {ch}")),
        None => spectrum::mono(&samples[a * ch..b * ch], ch, &mut x),
    }
    const SIZE: usize = 4096;
    if x.len() < SIZE {
        return Err("shorter than one transform (4096 samples)".into());
    }
    let (w, h) = (1600usize, 600usize);
    let mut s = Spectrum::new(f.rate, SIZE);
    let nyquist = f.rate as f32 / 2.0;
    let decades = (nyquist / 20.0).log10();
    // Each row's bins: the range of bins its log band covers.
    let bin_hz = s.bin_hz();
    let rows: Vec<(usize, usize)> = (0..h)
        .map(|r| {
            let lo = 20.0 * 10f32.powf((h - 1 - r) as f32 / h as f32 * decades);
            let hi = 20.0 * 10f32.powf((h - r) as f32 / h as f32 * decades);
            let k0 = ((lo / bin_hz).round() as usize).clamp(1, SIZE / 2);
            (k0, ((hi / bin_hz).round() as usize).clamp(k0, SIZE / 2))
        })
        .collect();
    let transforms = (x.len() - SIZE) / SIZE + 1;
    let mut rgb = vec![0u8; w * h * 3];
    let mut column = vec![f32::MIN; h];
    let col_of = |t: usize| t * w / transforms;
    let mut current = 0usize;
    let flush = |col: usize, column: &mut Vec<f32>, rgb: &mut Vec<u8>| {
        for (r, &db) in column.iter().enumerate() {
            let c = if db == f32::MIN {
                [0, 0, 4]
            } else {
                inferno(1.0 + db / 160.0)
            };
            let i = (r * w + col) * 3;
            rgb[i..i + 3].copy_from_slice(&c);
        }
        column.fill(f32::MIN);
    };
    for t in 0..transforms {
        let col = col_of(t);
        if col != current {
            // Columns a long file skips get the one before them; a short file's are stretched.
            for c in current..col {
                let mut copy = column.clone();
                flush(c, &mut copy, &mut rgb);
            }
            column.fill(f32::MIN);
            current = col;
        }
        s.feed(&x[t * SIZE..(t + 1) * SIZE]);
        let bins = s.compute();
        for (r, &(k0, k1)) in rows.iter().enumerate() {
            let db = bins[k0..=k1].iter().copied().fold(f32::MIN, f32::max);
            column[r] = column[r].max(db);
        }
    }
    for c in current..w {
        let mut copy = column.clone();
        flush(c, &mut copy, &mut rgb);
    }
    // Frequency marks: a faint line at 100 Hz, 1 kHz and 10 kHz.
    for hz in [100.0f32, 1000.0, 10_000.0]
        .into_iter()
        .filter(|&hz| hz < nyquist)
    {
        let r = h - 1 - ((hz / 20.0).log10() / decades * h as f32) as usize;
        for c in 0..w {
            let i = (r * w + c) * 3;
            rgb[i..i + 3]
                .iter_mut()
                .for_each(|v| *v = v.saturating_add(40));
        }
    }
    let file_out = File::create(&out).map_err(|e| format!("{out}: {e}"))?;
    let mut enc = png::Encoder::new(BufWriter::new(file_out), w as u32, h as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .and_then(|mut wr| wr.write_image_data(&rgb))
        .map_err(|e| e.to_string())?;
    let secs = (b - a) as f64 / f64::from(f.rate);
    println!(
        "{out}: {:.2} s across ({:.3} s a column), 20 Hz to {:.0} Hz up (log; lines at 100, 1k, 10k), 0 dBFS yellow to -160 dBFS black",
        secs,
        secs / w as f64,
        nyquist
    );
    Ok(())
}
