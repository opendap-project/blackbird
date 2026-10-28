//! Spectra and spectrograms as pictures, for a person or an agent to look at.

use super::*;

pub(crate) fn spectrum_png(args: &[String]) -> Result<(), String> {
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
pub(crate) fn inferno(t: f32) -> [u8; 3] {
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

pub(crate) fn spectrogram_png(args: &[String]) -> Result<(), String> {
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
    if x.len() < SG_SIZE {
        return Err("shorter than one transform (4096 samples)".into());
    }
    let (w, h) = (1600usize, 600usize);
    let nyquist = f.rate as f32 / 2.0;
    let mut rgb = waterfall(&x, f.rate, w, h);
    marks(&mut rgb, w, h, nyquist);
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

/// The spectrogram's transform size.
const SG_SIZE: usize = 4096;

/// Time across `w` columns, 20 Hz to Nyquist up `h` rows on a log axis, each pixel the loudest
/// bin of its band in its columns' transforms, as inferno colours (RGB).
fn waterfall(x: &[f32], rate: u32, w: usize, h: usize) -> Vec<u8> {
    const SIZE: usize = SG_SIZE;
    let mut s = Spectrum::new(rate, SIZE);
    let nyquist = rate as f32 / 2.0;
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
    rgb
}

/// Frequency marks: a faint line at 100 Hz, 1 kHz and 10 kHz.
fn marks(rgb: &mut [u8], w: usize, h: usize, nyquist: f32) {
    let decades = (nyquist / 20.0).log10();
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
}
