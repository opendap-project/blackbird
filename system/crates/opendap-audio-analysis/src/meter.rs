//! A level meter per channel, as a monitoring display reads it: the sample peak with a hold, the
//! RMS over the last 300 ms, the samples at full scale (clips) and the DC offset. Fed interleaved
//! f32 samples ([`crate::decode`]); its buffers are sized at construction.
//!
//! Ballistics (for a UI, not a standard's PPM): the peak holds 1.5 s, then falls 20 dB a second
//! (the hold-and-fall most digital meters show); RMS is a moving average over 300 ms (the
//! integration time of a VU meter), so a full-scale sine reads 0 dBFS peak and -3.01 dBFS RMS.

use crate::{FLOOR_DBFS, amplitude_db, power_db};

/// How long a peak holds before it falls, and how fast it falls.
pub const PEAK_HOLD_S: f64 = 1.5;
pub const PEAK_FALL_DB_S: f64 = 20.0;
/// The RMS window.
pub const RMS_WINDOW_S: f64 = 0.3;
/// A sample counts as clipped at or beyond this magnitude: full scale within one 16-bit LSB (a
/// 16-bit stream's largest positive sample is 32767/32768).
pub const CLIP: f32 = 32767.0 / 32768.0;

/// One channel's reading.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Level {
    /// The held peak, dBFS.
    pub peak_dbfs: f32,
    /// The RMS over the window, dBFS.
    pub rms_dbfs: f32,
    /// Samples at or beyond full scale since the last reset.
    pub clipped: u64,
    /// The mean over the window (full scale = 1).
    pub dc: f32,
}

#[derive(Debug, Clone)]
struct Channel {
    /// The window's squares and values, a ring.
    squares: Vec<f64>,
    values: Vec<f64>,
    sum_squares: f64,
    sum: f64,
    held_db: f64,
    /// Samples since the held peak was set.
    since_peak: u64,
    clipped: u64,
}

#[derive(Debug, Clone)]
pub struct Meter {
    rate: u32,
    channels: Vec<Channel>,
    window: usize,
    at: usize,
    filled: usize,
    /// Frames since the sums were last recomputed from the ring (a running sum drifts).
    since_exact: usize,
}

impl Meter {
    pub fn new(channels: usize, rate: u32) -> Self {
        let window = ((f64::from(rate) * RMS_WINDOW_S).round() as usize).max(1);
        let channel = Channel {
            squares: vec![0.0; window],
            values: vec![0.0; window],
            sum_squares: 0.0,
            sum: 0.0,
            held_db: f64::from(FLOOR_DBFS),
            since_peak: 0,
            clipped: 0,
        };
        Meter {
            rate,
            channels: vec![channel; channels.max(1)],
            window,
            at: 0,
            filled: 0,
            since_exact: 0,
        }
    }

    /// Interleaved samples; a trailing partial frame is ignored.
    pub fn feed(&mut self, interleaved: &[f32]) {
        let n = self.channels.len();
        let hold = (PEAK_HOLD_S * f64::from(self.rate)) as u64;
        let fall_per_sample = PEAK_FALL_DB_S / f64::from(self.rate);
        for frame in interleaved.chunks_exact(n) {
            for (ch, &x) in self.channels.iter_mut().zip(frame) {
                let x = f64::from(x);
                let sq = x * x;
                ch.sum_squares += sq - ch.squares[self.at];
                ch.sum += x - ch.values[self.at];
                ch.squares[self.at] = sq;
                ch.values[self.at] = x;
                if x.abs() >= f64::from(CLIP) {
                    ch.clipped += 1;
                }
                // The peak: a new high sets the hold; past the hold it falls.
                let db = f64::from(amplitude_db(x.abs()));
                ch.since_peak += 1;
                if ch.since_peak > hold {
                    ch.held_db = (ch.held_db - fall_per_sample).max(f64::from(FLOOR_DBFS));
                }
                if db >= ch.held_db {
                    ch.held_db = db;
                    ch.since_peak = 0;
                }
            }
            self.at = (self.at + 1) % self.window;
            self.filled = (self.filled + 1).min(self.window);
            self.since_exact += 1;
            if self.since_exact >= self.window {
                self.since_exact = 0;
                for ch in &mut self.channels {
                    ch.sum_squares = ch.squares.iter().sum();
                    ch.sum = ch.values.iter().sum();
                }
            }
        }
    }

    pub fn levels(&self) -> Vec<Level> {
        let n = self.filled.max(1) as f64;
        self.channels
            .iter()
            .map(|ch| Level {
                peak_dbfs: ch.held_db as f32,
                rms_dbfs: power_db((ch.sum_squares / n).max(0.0)),
                clipped: ch.clipped,
                dc: (ch.sum / n) as f32,
            })
            .collect()
    }

    /// Back to silence: the window, the held peaks and the clip counts.
    pub fn reset(&mut self) {
        *self = Meter::new(self.channels.len(), self.rate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(amplitude: f32, freq: f32, rate: u32, frames: usize, channels: usize) -> Vec<f32> {
        (0..frames)
            .flat_map(|i| {
                let v =
                    amplitude * (2.0 * std::f32::consts::PI * freq * i as f32 / rate as f32).sin();
                std::iter::repeat_n(v, channels)
            })
            .collect()
    }

    #[test]
    fn a_full_scale_sine_reads_0_peak_and_minus_3_rms() {
        let mut m = Meter::new(2, 48_000);
        m.feed(&sine(1.0, 997.0, 48_000, 48_000, 2));
        for l in m.levels() {
            assert!(l.peak_dbfs > -0.01, "peak {}", l.peak_dbfs);
            assert!((l.rms_dbfs + 3.0103).abs() < 0.01, "rms {}", l.rms_dbfs);
            assert!(l.dc.abs() < 1e-3);
        }
    }

    #[test]
    fn silence_reads_the_floor() {
        let mut m = Meter::new(1, 48_000);
        assert_eq!(m.levels()[0].rms_dbfs, FLOOR_DBFS);
        m.feed(&[0.0; 4800]);
        let l = m.levels()[0];
        assert_eq!(
            (l.peak_dbfs, l.rms_dbfs, l.clipped),
            (FLOOR_DBFS, FLOOR_DBFS, 0)
        );
    }

    #[test]
    fn the_peak_holds_then_falls() {
        let rate = 48_000;
        let mut m = Meter::new(1, rate);
        m.feed(&sine(0.5, 1000.0, rate, 4800, 1));
        m.feed(&vec![0.0; (rate as f64 * 1.4) as usize]);
        let held = m.levels()[0].peak_dbfs;
        assert!((held + 6.02).abs() < 0.05, "held {held}");
        m.feed(&vec![0.0; rate as usize]); // 1.5 s hold over, about 0.9 s falling
        let fell = m.levels()[0].peak_dbfs;
        assert!((held - fell - 18.0).abs() < 2.0, "fell to {fell}");
    }

    #[test]
    fn rms_covers_only_the_window() {
        let mut m = Meter::new(1, 48_000);
        m.feed(&sine(1.0, 1000.0, 48_000, 48_000, 1));
        m.feed(&vec![0.0; 48_000]);
        assert_eq!(m.levels()[0].rms_dbfs, FLOOR_DBFS);
    }

    #[test]
    fn clips_dc_and_channels_apart() {
        let mut m = Meter::new(2, 48_000);
        let frames: Vec<f32> = (0..1000)
            .flat_map(|i| [if i % 10 == 0 { 1.0 } else { 0.0 }, 0.25])
            .collect();
        m.feed(&frames);
        let l = m.levels();
        assert_eq!(l[0].clipped, 100);
        assert_eq!(l[1].clipped, 0);
        assert!((l[1].dc - 0.25).abs() < 1e-6);
        m.reset();
        assert_eq!(m.levels()[0].clipped, 0);
    }

    #[test]
    fn a_long_run_does_not_drift() {
        let mut m = Meter::new(1, 8_000);
        let s = sine(0.1, 440.0, 8_000, 8_000, 1);
        for _ in 0..600 {
            m.feed(&s);
        }
        let expected = 20.0 * (0.1f32 / 2f32.sqrt()).log10();
        assert!((m.levels()[0].rms_dbfs - expected).abs() < 0.01);
    }
}
