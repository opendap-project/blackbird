//! The PCM plumbing: the rings between QEMU's listeners and the host's streams, the listening
//! resampler, and encoding samples into the guest's format.

use super::*;

pub(super) const RING_S: f64 = 0.25;
/// Sound buffered before the host starts playing, and again after it ran dry.
pub(super) const PRIME_S: f64 = 0.06;
/// A bounded queue of interleaved samples between a listener and a host stream.
pub(super) struct Ring {
    pub(super) q: VecDeque<f32>,
    pub(super) cap: usize,
    pub(super) prime: usize,
    pub(super) primed: bool,
}

impl Ring {
    pub(super) fn new(rate: u32, channels: usize) -> Self {
        let per_s = f64::from(rate) * channels as f64;
        Ring {
            q: VecDeque::with_capacity((per_s * RING_S) as usize),
            cap: (per_s * RING_S) as usize,
            prime: (per_s * PRIME_S) as usize,
            primed: false,
        }
    }

    pub(super) fn push(&mut self, samples: &[f32]) {
        self.q.extend(samples);
        let over = self.q.len().saturating_sub(self.cap);
        self.q.drain(..over);
    }

    /// Fills `out`, with silence while priming or once dry.
    pub(super) fn pull(&mut self, out: &mut [f32]) {
        if !self.primed && self.q.len() >= self.prime {
            self.primed = true;
        }
        for s in out.iter_mut() {
            *s = if self.primed {
                self.q.pop_front().unwrap_or(0.0)
            } else {
                0.0
            };
        }
        if self.q.is_empty() {
            self.primed = false;
        }
    }
}

/// A linear resampler and channel map, streaming (its position and last frame carry over).
pub(super) struct Resampler {
    pub(super) step: f64,
    pub(super) pos: f64,
    pub(super) last: Vec<f32>,
    pub(super) in_ch: usize,
    pub(super) out_ch: usize,
}

impl Resampler {
    pub(super) fn new(in_rate: u32, in_ch: usize, out_rate: u32, out_ch: usize) -> Self {
        Resampler {
            step: f64::from(in_rate) / f64::from(out_rate),
            pos: 0.0,
            last: vec![0.0; in_ch],
            in_ch,
            out_ch,
        }
    }

    pub(super) fn identity(&self) -> bool {
        self.step == 1.0 && self.in_ch == self.out_ch
    }

    pub(super) fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        if self.identity() {
            out.extend_from_slice(input);
            return;
        }
        let frames = input.len() / self.in_ch;
        let at = |i: isize, c: usize| -> f32 {
            if i < 0 {
                self.last[c]
            } else {
                input[i as usize * self.in_ch + c]
            }
        };
        // Positions are counted from the last frame of the previous call (index -1).
        while self.pos < frames as f64 {
            let i = self.pos.floor() as isize - 1;
            let t = (self.pos - self.pos.floor()) as f32;
            for c in 0..self.out_ch {
                let ic = c.min(self.in_ch - 1);
                out.push(at(i, ic) + (at(i + 1, ic) - at(i, ic)) * t);
            }
            self.pos += self.step;
        }
        self.pos -= frames as f64;
        if frames > 0 {
            self.last
                .copy_from_slice(&input[(frames - 1) * self.in_ch..frames * self.in_ch]);
        }
    }
}

/// f32 samples as bytes in `f` (integers rounded and clamped to the format's range).
pub(super) fn encode(f: &Format, samples: &[f32], out: &mut Vec<u8>) {
    if f.float {
        encode_float(f, samples, out);
        return;
    }
    let scale = (1u64 << (f.bits - 1)) as f64;
    let ints: Vec<i64> = samples
        .iter()
        .map(|&s| (f64::from(s) * scale).round() as i64)
        .collect();
    encode_exact(f, &ints, out);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub(super) fn the_resampler_keeps_identity_exact_and_converts_otherwise() {
        let mut same = Resampler::new(48_000, 2, 48_000, 2);
        let mut out = Vec::new();
        same.process(&[0.1, 0.2, 0.3, 0.4], &mut out);
        assert_eq!(out, [0.1, 0.2, 0.3, 0.4]);
        // Doubling the rate of a ramp, one channel to two: twice the frames, both channels equal.
        let mut up = Resampler::new(24_000, 1, 48_000, 2);
        let ramp: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let mut out = Vec::new();
        up.process(&ramp, &mut out);
        up.process(&ramp, &mut out);
        assert_eq!(out.len(), 2 * 400, "400 frames of two channels");
        assert!(out.as_chunks::<2>().0.iter().all(|f| f[0] == f[1]));
    }

    #[test]
    pub(super) fn the_ring_primes_and_stays_bounded() {
        let mut r = Ring::new(1000, 1);
        let mut out = [1.0f32; 10];
        r.push(&[0.5; 30]);
        r.pull(&mut out);
        assert_eq!(out, [0.0; 10], "below the 60 ms prime: silence");
        r.push(&[0.5; 1000]);
        assert_eq!(r.q.len(), 250, "250 ms at most");
        r.pull(&mut out);
        assert_eq!(out, [0.5; 10]);
    }

    #[test]
    pub(super) fn encoding_rounds_and_clamps() {
        let f = Format::int(48_000, 1, 16);
        let mut b = Vec::new();
        encode(&f, &[0.5, -1.0, 1.0], &mut b);
        assert_eq!(b, [0x00, 0x40, 0x00, 0x80, 0xff, 0x7f]);
    }
}
