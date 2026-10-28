//! A sine test signal's analysis, as an audio analyzer reports it: the fundamental's frequency and
//! level, THD (harmonics 2 to 10 below Nyquist), THD+N (everything but the fundamental, 20 Hz to
//! 20 kHz or Nyquist) and SNR (the fundamental against what remains without the harmonics).
//!
//! Method: a fitted-sine notch. A four-parameter sine fit (IEEE 1057: amplitude, phase, offset
//! and frequency, by Gauss-Newton from the windowed FFT's peak) removes the fundamental exactly
//! in time, whatever its frequency against the bins, so nothing of it leaks into the residual.
//! Each harmonic's amplitude is then a three-parameter fit at its exact multiple; the residual's
//! power in the band comes from its Blackman-Harris spectrum. Works on up to 2^18 samples (the
//! last ones given), with buffers allocated per call.

use crate::spectrum::{Spectrum, blackman_harris};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Report {
    pub frequency_hz: f32,
    /// The fundamental's peak level, dBFS (0 for a full-scale sine).
    pub level_dbfs: f32,
    pub thd_percent: f32,
    pub thd_db: f32,
    pub thd_n_percent: f32,
    pub thd_n_db: f32,
    pub snr_db: f32,
}

/// The fewest samples analysed, and the most.
pub const MIN_SAMPLES: usize = 4096;
pub const MAX_SAMPLES: usize = 1 << 18;
/// Harmonics counted in THD.
pub const HARMONICS: u32 = 10;

// Index loops read as the textbook elimination they are.
#[allow(clippy::needless_range_loop)]
/// Solves `m · x = v` (4 × 4 at most) by Gaussian elimination with partial pivoting.
fn solve<const N: usize>(mut m: [[f64; N]; N], mut v: [f64; N]) -> Option<[f64; N]> {
    for col in 0..N {
        let p = (col..N).max_by(|&a, &b| m[a][col].abs().total_cmp(&m[b][col].abs()))?;
        if m[p][col].abs() < 1e-300 {
            return None;
        }
        m.swap(col, p);
        v.swap(col, p);
        for r in col + 1..N {
            let f = m[r][col] / m[col][col];
            for c in col..N {
                m[r][c] -= f * m[col][c];
            }
            v[r] -= f * v[col];
        }
    }
    let mut x = [0.0; N];
    for r in (0..N).rev() {
        x[r] = (v[r] - (r + 1..N).map(|c| m[r][c] * x[c]).sum::<f64>()) / m[r][r];
    }
    Some(x)
}

#[allow(clippy::needless_range_loop)]
/// Least squares of `columns` (each a function of the sample index) against `x`.
fn least_squares<const N: usize>(
    x: &[f64],
    columns: impl Fn(usize) -> [f64; N],
) -> Option<[f64; N]> {
    let mut m = [[0.0; N]; N];
    let mut v = [0.0; N];
    for (i, &xi) in x.iter().enumerate() {
        let c = columns(i);
        for r in 0..N {
            v[r] += c[r] * xi;
            for k in r..N {
                m[r][k] += c[r] * c[k];
            }
        }
    }
    for r in 0..N {
        for k in 0..r {
            m[r][k] = m[k][r];
        }
    }
    solve(m, v)
}

/// The amplitude of a sine at `w` radians a sample in `x` (a three-parameter fit: cos, sin and
/// an offset).
fn amplitude_at(x: &[f64], w: f64, t0: f64) -> f64 {
    least_squares(x, |i| {
        let t = i as f64 - t0;
        [(w * t).cos(), (w * t).sin(), 1.0]
    })
    .map_or(0.0, |[a, b, _]| a.hypot(b))
}

pub fn analyze(mono: &[f32], rate: u32) -> Option<Report> {
    if mono.len() < MIN_SAMPLES || rate == 0 {
        return None;
    }
    let x: Vec<f64> = mono[mono.len().saturating_sub(MAX_SAMPLES)..]
        .iter()
        .map(|&s| f64::from(s))
        .collect();
    let n = x.len();
    let pow2 = 1usize << (usize::BITS - 1 - n.leading_zeros());
    // The fundamental's first estimate: the windowed spectrum's peak (above 10 Hz).
    let mut s = Spectrum::new(rate, pow2);
    s.feed(&mono[mono.len() - pow2..]);
    s.compute();
    let (f0, level0) = s.peak();
    if level0 < -140.0 || f0 < 10.0 {
        return None;
    }
    // Gauss-Newton on amplitude (as cos and sin terms), offset and frequency; time centred.
    let t0 = (n as f64 - 1.0) / 2.0;
    let mut w = 2.0 * std::f64::consts::PI * f64::from(f0) / f64::from(rate);
    let [mut a, mut b, mut c] = least_squares(&x, |i| {
        let t = i as f64 - t0;
        [(w * t).cos(), (w * t).sin(), 1.0]
    })?;
    for _ in 0..12 {
        // One Gauss-Newton step: the fit's partial derivatives against its residual.
        let r: Vec<f64> = x
            .iter()
            .enumerate()
            .map(|(i, &xi)| {
                let t = i as f64 - t0;
                xi - (a * (w * t).cos() + b * (w * t).sin() + c)
            })
            .collect();
        let [da, db, dc, dw] = least_squares(&r, |i| {
            let t = i as f64 - t0;
            let (cs, sn) = ((w * t).cos(), (w * t).sin());
            [cs, sn, 1.0, t * (-a * sn + b * cs)]
        })?;
        a += da;
        b += db;
        c += dc;
        w += dw;
        if dw.abs() < 1e-15 {
            break;
        }
    }
    let fundamental = a.hypot(b);
    if fundamental <= 0.0 || !w.is_finite() || w <= 0.0 {
        return None;
    }
    let residual: Vec<f64> = x
        .iter()
        .enumerate()
        .map(|(i, &xi)| {
            let t = i as f64 - t0;
            xi - (a * (w * t).cos() + b * (w * t).sin() + c)
        })
        .collect();
    let p_fund = fundamental * fundamental / 2.0;
    // Harmonics 2..=10 below Nyquist, each fitted at its exact multiple.
    let harmonics: Vec<(f64, f64)> = (2..=HARMONICS)
        .map(|h| f64::from(h) * w)
        .take_while(|&wh| wh < std::f64::consts::PI)
        .map(|wh| (wh, amplitude_at(&residual, wh, t0)))
        .collect();
    let p_harm: f64 = harmonics.iter().map(|(_, amp)| amp * amp / 2.0).sum();
    // The residual's power in the band, from its windowed spectrum (Parseval, one-sided).
    let nyquist = f64::from(rate) / 2.0;
    let (lo, hi) = (20.0, 20_000.0f64.min(nyquist));
    let win = blackman_harris(pow2);
    let seg = &residual[n - pow2..];
    let mut planner = realfft::RealFftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(pow2);
    let mut input: Vec<f64> = seg
        .iter()
        .zip(&win)
        .map(|(r, &w)| r * f64::from(w))
        .collect();
    let mut out = fft.make_output_vec();
    fft.process(&mut input, &mut out).ok()?;
    let w2: f64 = win.iter().map(|&w| f64::from(w) * f64::from(w)).sum();
    let bin = f64::from(rate) / pow2 as f64;
    let band: f64 = out
        .iter()
        .enumerate()
        .filter(|(k, _)| {
            let f = *k as f64 * bin;
            *k > 0 && *k < pow2 / 2 && f >= lo && f <= hi
        })
        .map(|(_, c)| c.norm_sqr())
        .sum::<f64>()
        * 2.0
        / (pow2 as f64 * w2);
    let p_harm_in_band: f64 = harmonics
        .iter()
        .filter(|(wh, _)| (wh / (2.0 * std::f64::consts::PI) * f64::from(rate)) <= hi)
        .map(|(_, amp)| amp * amp / 2.0)
        .sum();
    let p_noise = (band - p_harm_in_band).max(band * 1e-6).max(1e-300);
    // No tone: the "fundamental" is not clearly above what remains.
    if p_fund < 10.0 * band {
        return None;
    }
    let ratio_db = |p: f64| crate::power_db(p / p_fund);
    let pct = |p: f64| ((p / p_fund).sqrt() * 100.0) as f32;
    Some(Report {
        frequency_hz: (w / (2.0 * std::f64::consts::PI) * f64::from(rate)) as f32,
        level_dbfs: crate::amplitude_db(fundamental),
        thd_percent: pct(p_harm),
        thd_db: ratio_db(p_harm),
        thd_n_percent: pct(band),
        thd_n_db: ratio_db(band),
        snr_db: crate::power_db(p_fund / p_noise),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48_000;

    fn signal(f: f64, parts: &[(u32, f64)], n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| {
                let t = i as f64 / f64::from(RATE);
                parts
                    .iter()
                    .map(|&(h, a)| {
                        a * (2.0 * std::f64::consts::PI * f * f64::from(h) * t + 0.3).sin()
                    })
                    .sum()
            })
            .collect()
    }

    fn f32s(x: &[f64]) -> Vec<f32> {
        x.iter().map(|&v| v as f32).collect()
    }

    #[test]
    fn a_pure_sine_off_bin() {
        let r = analyze(&f32s(&signal(997.0, &[(1, 0.5)], 65_536)), RATE).unwrap();
        assert!((r.frequency_hz - 997.0).abs() < 0.001, "{}", r.frequency_hz);
        assert!((r.level_dbfs + 6.0206).abs() < 0.001, "{}", r.level_dbfs);
        assert!(r.thd_n_db < -120.0, "THD+N {}", r.thd_n_db);
    }

    #[test]
    fn a_known_second_harmonic() {
        let r = analyze(
            &f32s(&signal(1000.0, &[(1, 0.5), (2, 0.005)], 65_536)),
            RATE,
        )
        .unwrap();
        assert!(
            (r.thd_percent - 1.0).abs() < 0.05,
            "THD {} %",
            r.thd_percent
        );
        assert!((r.thd_db + 40.0).abs() < 0.1, "{}", r.thd_db);
        assert!((r.thd_n_percent - 1.0).abs() < 0.05);
    }

    #[test]
    fn a_16_bit_full_scale_sine() {
        let x: Vec<f32> = signal(997.0, &[(1, 1.0)], 131_072)
            .iter()
            .map(|&v| ((v * 32767.0).round() / 32768.0) as f32)
            .collect();
        let r = analyze(&x, RATE).unwrap();
        // 16-bit quantization: 6.02 × 16 + 1.76 = 98.1 dB over the whole band; 20 Hz to 20 kHz
        // of 24 kHz keeps 5/6 of the noise (0.8 dB less).
        assert!((r.thd_n_db + 98.9).abs() < 2.0, "THD+N {}", r.thd_n_db);
        assert!((r.snr_db - 98.9).abs() < 2.0, "SNR {}", r.snr_db);
    }

    #[test]
    fn white_noise_at_a_known_level() {
        // A deterministic uniform noise of RMS 1e-3 (-60 dB against a 0.707 RMS tone: SNR 57 dB
        // over the whole band, 57.8 in 20 Hz–20 kHz).
        let mut seed = 0x2545F4914F6CDD1Du64;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 11) as f64 / (1u64 << 53) as f64 - 0.5
        };
        let amp = 1e-3 * 12f64.sqrt();
        let x: Vec<f32> = signal(1000.0, &[(1, 1.0)], 131_072)
            .iter()
            .map(|&v| (v + amp * rnd()) as f32)
            .collect();
        let r = analyze(&x, RATE).unwrap();
        let expected = 10.0 * (0.5f64 / 1e-6).log10() + 10.0 * (24.0f64 / 19.98).log10();
        assert!(
            (f64::from(r.snr_db) - expected).abs() < 1.0,
            "SNR {} for {expected}",
            r.snr_db
        );
    }

    #[test]
    fn too_short_or_silent() {
        assert!(analyze(&[0.0; 100], RATE).is_none());
        assert!(analyze(&[0.0; 8192], RATE).is_none());
    }
}
