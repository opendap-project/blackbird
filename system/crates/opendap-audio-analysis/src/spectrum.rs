//! The spectrum of the last `size` samples: a Blackman-Harris (4-term) window and a real FFT,
//! in dBFS per bin, amplitude-corrected so a full-scale sine reads 0 dBFS at its bin (within the
//! window's scalloping, under 0.83 dB off-bin). The window's sidelobes sit at -92 dB, so a 16-bit
//! stream's noise floor shows. For a display strip, [`Spectrum::bands`] folds the bins into
//! log-spaced bands.

use std::sync::Arc;

use realfft::{RealFftPlanner, RealToComplex};

use crate::FLOOR_DBFS;

/// The 4-term Blackman-Harris window (-92 dB sidelobes; Harris 1978).
pub fn blackman_harris(n: usize) -> Vec<f32> {
    let (a0, a1, a2, a3) = (0.35875, 0.48829, 0.14128, 0.01168);
    (0..n)
        .map(|i| {
            let x = 2.0 * std::f64::consts::PI * i as f64 / n as f64;
            (a0 - a1 * x.cos() + a2 * (2.0 * x).cos() - a3 * (3.0 * x).cos()) as f32
        })
        .collect()
}

/// Interleaved frames mixed to mono (the mean of the channels), appended to `out`.
pub fn mono(interleaved: &[f32], channels: usize, out: &mut Vec<f32>) {
    let c = channels.max(1);
    out.extend(
        interleaved
            .chunks_exact(c)
            .map(|f| f.iter().sum::<f32>() / c as f32),
    );
}

/// One channel of interleaved frames, appended to `out`.
pub fn channel(interleaved: &[f32], channels: usize, which: usize, out: &mut Vec<f32>) {
    out.extend(
        interleaved
            .chunks_exact(channels.max(1))
            .filter_map(|f| f.get(which).copied()),
    );
}

pub struct Spectrum {
    rate: u32,
    ring: Vec<f32>,
    at: usize,
    window: Vec<f32>,
    /// What turns a bin's magnitude into an amplitude: 2 / sum(window).
    gain: f64,
    fft: Arc<dyn RealToComplex<f32>>,
    input: Vec<f32>,
    output: Vec<realfft::num_complex::Complex<f32>>,
    scratch: Vec<realfft::num_complex::Complex<f32>>,
    /// Power per bin, smoothed across computes.
    power: Vec<f64>,
    db: Vec<f32>,
    alpha: f32,
    fresh: bool,
}

impl Spectrum {
    /// `size` samples per transform (a power of two: 8192 gives 5.9 Hz bins at 48 kHz).
    pub fn new(rate: u32, size: usize) -> Self {
        assert!(
            size.is_power_of_two() && size >= 16,
            "a power of two of at least 16"
        );
        let fft = RealFftPlanner::<f32>::new().plan_fft_forward(size);
        let window = blackman_harris(size);
        let gain = 2.0 / window.iter().map(|&w| f64::from(w)).sum::<f64>();
        Spectrum {
            rate,
            ring: vec![0.0; size],
            at: 0,
            input: fft.make_input_vec(),
            output: fft.make_output_vec(),
            scratch: fft.make_scratch_vec(),
            power: vec![0.0; size / 2 + 1],
            db: vec![FLOOR_DBFS; size / 2 + 1],
            fft,
            window,
            gain,
            alpha: 1.0,
            fresh: true,
        }
    }

    pub fn rate(&self) -> u32 {
        self.rate
    }

    pub fn size(&self) -> usize {
        self.ring.len()
    }

    /// The width of a bin, Hz.
    pub fn bin_hz(&self) -> f32 {
        self.rate as f32 / self.ring.len() as f32
    }

    /// Exponential averaging of the power across computes: 1 (the default) shows each transform
    /// as it is; 0.2 averages about the last five.
    pub fn smoothing(&mut self, alpha: f32) {
        self.alpha = alpha.clamp(0.01, 1.0);
    }

    pub fn feed(&mut self, mono: &[f32]) {
        let n = self.ring.len();
        for &x in mono.iter().skip(mono.len().saturating_sub(n)) {
            self.ring[self.at] = x;
            self.at = (self.at + 1) % n;
        }
    }

    /// dBFS per bin, from DC to Nyquist, of the last `size` samples.
    pub fn compute(&mut self) -> &[f32] {
        let n = self.ring.len();
        for i in 0..n {
            self.input[i] = self.ring[(self.at + i) % n] * self.window[i];
        }
        if self
            .fft
            .process_with_scratch(&mut self.input, &mut self.output, &mut self.scratch)
            .is_err()
        {
            return &self.db;
        }
        let a = f64::from(self.alpha);
        for (k, c) in self.output.iter().enumerate() {
            // DC and Nyquist have no mirrored half: their amplitude is half the doubled one.
            let edge = if k == 0 || k == n / 2 { 0.5 } else { 1.0 };
            let amp = f64::from(c.norm()) * self.gain * edge;
            let p = amp * amp;
            self.power[k] = if self.fresh {
                p
            } else {
                self.power[k] + a * (p - self.power[k])
            };
            self.db[k] = crate::power_db(self.power[k]);
        }
        self.fresh = false;
        &self.db
    }

    /// The last computed bins.
    pub fn bins(&self) -> &[f32] {
        &self.db
    }

    /// `n` log-spaced bands from `lo_hz` to `hi_hz` (clamped to Nyquist), each the highest bin in
    /// it; a band narrower than a bin takes the bin at its centre.
    pub fn bands(&self, n: usize, lo_hz: f32, hi_hz: f32) -> Vec<f32> {
        let nyquist = self.rate as f32 / 2.0;
        let (lo, hi) = (lo_hz.max(self.bin_hz() / 2.0), hi_hz.min(nyquist));
        if n == 0 || hi <= lo {
            return vec![FLOOR_DBFS; n];
        }
        let ratio = (hi / lo).powf(1.0 / n as f32);
        let bin = |hz: f32| ((hz / self.bin_hz()).round() as usize).min(self.db.len() - 1);
        (0..n)
            .map(|i| {
                let (a, b) = (lo * ratio.powi(i as i32), lo * ratio.powi(i as i32 + 1));
                let (ka, kb) = (
                    (a / self.bin_hz()).ceil() as usize,
                    (b / self.bin_hz()).floor() as usize,
                );
                if ka > kb || ka >= self.db.len() {
                    self.db[bin((a * b).sqrt())]
                } else {
                    self.db[ka..=kb.min(self.db.len() - 1)]
                        .iter()
                        .copied()
                        .fold(FLOOR_DBFS, f32::max)
                }
            })
            .collect()
    }

    /// The highest bin (DC excluded): its frequency, interpolated on a parabola through the bins
    /// around it in dB, and its level.
    pub fn peak(&self) -> (f32, f32) {
        let (k, &top) = self
            .db
            .iter()
            .enumerate()
            .skip(1)
            .fold((1, &FLOOR_DBFS), |m, x| if x.1 > m.1 { x } else { m });
        let shift = if k + 1 < self.db.len() {
            let (a, b, c) = (self.db[k - 1], top, self.db[k + 1]);
            let d = a - 2.0 * b + c;
            if d.abs() > f32::EPSILON {
                (0.5 * (a - c) / d).clamp(-0.5, 0.5)
            } else {
                0.0
            }
        } else {
            0.0
        };
        ((k as f32 + shift) * self.bin_hz(), top)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(amplitude: f32, freq: f32, rate: u32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| amplitude * (2.0 * std::f32::consts::PI * freq * i as f32 / rate as f32).sin())
            .collect()
    }

    #[test]
    fn a_full_scale_sine_on_a_bin_reads_0_dbfs() {
        let mut s = Spectrum::new(48_000, 8192);
        let f = 128.0 * s.bin_hz();
        s.feed(&sine(1.0, f, 48_000, 8192));
        let db = s.compute().to_vec();
        assert!(db[128].abs() < 0.1, "{}", db[128]);
        let (freq, level) = s.peak();
        assert!((freq - f).abs() < 0.1 * s.bin_hz());
        assert!(level.abs() < 0.1);
        // Far from it, the window's sidelobes and f32's own noise: below -100 dB.
        assert!(db[1000] < -100.0, "{}", db[1000]);
    }

    #[test]
    fn off_bin_within_the_scalloping_and_the_peak_interpolated() {
        let mut s = Spectrum::new(48_000, 8192);
        let f = 170.5 * s.bin_hz(); // halfway between two bins: the worst scalloping
        s.feed(&sine(0.5, f, 48_000, 8192));
        s.compute();
        let (freq, level) = s.peak();
        assert!((level + 6.02).abs() < 0.9, "{level}");
        assert!((freq - f).abs() < 0.25 * s.bin_hz(), "{freq} for {f}");
    }

    #[test]
    fn silence_reads_the_floor_and_bands_cover_the_range() {
        let mut s = Spectrum::new(48_000, 1024);
        s.feed(&[0.0; 1024]);
        assert!(s.compute().iter().all(|&d| d == FLOOR_DBFS));
        s.feed(&sine(1.0, 1000.0, 48_000, 1024));
        s.compute();
        let bands = s.bands(32, 20.0, 20_000.0);
        assert_eq!(bands.len(), 32);
        let loudest = bands
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap()
            .0;
        // 1 kHz on a log axis from 20 Hz to 20 kHz: 56.6 % of the way.
        assert!(
            (loudest as f32 / 32.0 - 0.566).abs() < 0.05,
            "band {loudest}"
        );
    }

    #[test]
    fn smoothing_averages() {
        let mut s = Spectrum::new(8_000, 256);
        s.smoothing(0.5);
        s.feed(&sine(1.0, 1000.0, 8_000, 256));
        let first = s.compute()[32];
        s.feed(&[0.0; 256]);
        let second = s.compute()[32];
        assert!(
            (first - second - 3.01).abs() < 0.05,
            "{first} then {second}"
        );
    }

    #[test]
    fn mono_and_channel() {
        let mut m = Vec::new();
        mono(&[1.0, 0.0, 0.5, 0.5], 2, &mut m);
        assert_eq!(m, [0.5, 0.5]);
        let mut c = Vec::new();
        channel(&[1.0, 2.0, 3.0, 4.0], 2, 1, &mut c);
        assert_eq!(c, [2.0, 4.0]);
    }
}
