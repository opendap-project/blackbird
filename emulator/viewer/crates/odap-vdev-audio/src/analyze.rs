//! Measuring a recording: levels, the loudest frequency, a sine's distortion, bands and blocks.

use super::*;

pub(crate) fn load(path: &str) -> Result<(Format, Vec<u8>), String> {
    let mut f = File::open(path).map_err(|e| format!("{path}: {e}"))?;
    wav::read(&mut f).map_err(|e| format!("{path}: {e}"))
}

/// The frames of `--from`..`--to` (seconds), or the middle of the file (up to 2^18 frames).
pub(crate) fn segment(args: &[String], f: &Format, samples: &[f32]) -> (usize, usize) {
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
pub(crate) fn averaged(f: &Format, mono: &[f32]) -> Spectrum {
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

pub(crate) fn analyze(args: &[String]) -> Result<(), String> {
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
    let channels: Vec<serde_json::Value> = (0..ch)
        .map(|c| channel_report(seg, ch, c, f.rate))
        .collect();
    let mut mono = Vec::new();
    spectrum::mono(seg, ch, &mut mono);
    let s = averaged(&f, &mono);
    let (peak_hz, peak_db) = s.peak();
    let bands: usize = opt(args, "--bands")
        .and_then(|n| n.parse().ok())
        .unwrap_or(0);
    let mut report = serde_json::json!({
        "file": file,
        "format": f.describe(),
        "seconds": samples.len() as f64 / ch as f64 / f64::from(f.rate),
        "segment_s": [a as f64 / f64::from(f.rate), b as f64 / f64::from(f.rate)],
        "channels": channels,
        "loudest": { "frequency_hz": peak_hz, "dbfs": peak_db },
        "spectrum_bin_hz": s.bin_hz(),
        "bands": (bands > 0).then(|| s.bands(bands, 20.0, 20_000.0)),
    });
    if let Some(block_s) = opt(args, "--blocks").and_then(|v| v.parse::<f64>().ok()) {
        report["blocks"] = blocks(&f, &samples, block_s);
    }
    if flag(args, "--json") {
        println!("{report:#}");
    } else {
        print_text(&report, &file, &f, &channels, &s, bands);
    }
    Ok(())
}

/// One channel's levels and, when it carries one, its sine's analysis.
fn channel_report(seg: &[f32], ch: usize, c: usize, rate: u32) -> serde_json::Value {
    let mut x = Vec::new();
    spectrum::channel(seg, ch, c, &mut x);
    let peak = x.iter().fold(0f32, |m, v| m.max(v.abs()));
    let ms = x.iter().map(|v| f64::from(*v) * f64::from(*v)).sum::<f64>() / x.len().max(1) as f64;
    let dc = x.iter().map(|v| f64::from(*v)).sum::<f64>() / x.len().max(1) as f64;
    let clipped = x
        .iter()
        .filter(|v| v.abs() >= blackbird_services_audio_analysis::meter::CLIP)
        .count();
    let t = tone::analyze(&x, rate);
    serde_json::json!({
        "peak_dbfs": amplitude_db(f64::from(peak)),
        "rms_dbfs": power_db(ms),
        "dc": dc,
        "clipped": clipped,
        "tone": t.map(|t| serde_json::json!({
            "frequency_hz": t.frequency_hz, "level_dbfs": t.level_dbfs,
            "thd_percent": t.thd_percent, "thd_db": t.thd_db,
            "thd_n_percent": t.thd_n_percent, "thd_n_db": t.thd_n_db, "snr_db": t.snr_db,
        })),
    })
}

/// The report as lines for a person.
fn print_text(
    report: &serde_json::Value,
    file: &str,
    f: &Format,
    channels: &[serde_json::Value],
    s: &Spectrum,
    bands: usize,
) {
    let (peak_hz, peak_db) = s.peak();
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
}

/// The whole file `block_s` seconds at a time, per channel: each block's sine analysis, the worst
/// and best THD+N, and the longest run of exact zero samples (a dropout the sender filled).
pub(crate) fn blocks(f: &Format, samples: &[f32], block_s: f64) -> serde_json::Value {
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
