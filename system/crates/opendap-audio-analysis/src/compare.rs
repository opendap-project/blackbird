//! Whether a captured stream carries a reference bit for bit: the check behind "bit-perfect".
//! The capture may begin with silence (it was listening before the stream started) or late (the
//! reference's start was missed), and may end early; what both have is compared sample by sample
//! on the exact integers ([`crate::decode_exact`]).

/// The result: where the capture's first sound sits in the reference, how much was compared and
/// how it differed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Comparison {
    /// The reference frame the capture's first non-silent frame matches.
    pub offset: usize,
    /// The capture's first non-silent frame.
    pub capture_start: usize,
    /// Frames compared.
    pub compared: usize,
    pub exact: bool,
    /// The first differing frame, counted from the alignment.
    pub first_mismatch: Option<usize>,
    /// The largest difference, in LSB of the stream's depth.
    pub max_lsb: u64,
}

/// Frames that must match exactly to accept an alignment.
pub const PROBE_FRAMES: usize = 256;

/// `reference` and `captured` are interleaved exact samples of `channels`. The capture's first
/// non-silent frames are searched in the reference from frame 0 to `max_offset`; None when they
/// are found nowhere (a capture that is not the reference, or that altered it from its start).
pub fn bit_exact(
    reference: &[i64],
    captured: &[i64],
    channels: usize,
    max_offset: usize,
) -> Option<Comparison> {
    let ch = channels.max(1);
    let (rf, cf) = (reference.len() / ch, captured.len() / ch);
    let frame = |i: usize| i * ch..(i + 1) * ch;
    let capture_start = (0..cf).find(|&i| captured[frame(i)].iter().any(|&v| v != 0))?;
    let probe = PROBE_FRAMES.min(cf - capture_start);
    let want = &captured[capture_start * ch..(capture_start + probe) * ch];
    let offset = (0..=max_offset.min(rf.saturating_sub(probe)))
        .find(|&o| reference.get(o * ch..(o + probe) * ch) == Some(want))?;
    let compared = (rf - offset).min(cf - capture_start);
    let (mut first_mismatch, mut max_lsb) = (None, 0u64);
    for i in 0..compared {
        let (r, c) = (frame(offset + i), frame(capture_start + i));
        for (a, b) in reference[r].iter().zip(&captured[c]) {
            let d = a.abs_diff(*b);
            if d > 0 {
                first_mismatch.get_or_insert(i);
                max_lsb = max_lsb.max(d);
            }
        }
    }
    Some(Comparison {
        offset,
        capture_start,
        compared,
        exact: first_mismatch.is_none(),
        first_mismatch,
        max_lsb,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noise(n: usize) -> Vec<i64> {
        let mut s = 12345u64;
        (0..n)
            .map(|_| {
                s = s
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ((s >> 48) as i64) - 32768
            })
            .collect()
    }

    #[test]
    fn a_late_capture_after_silence_is_exact() {
        let reference = noise(2 * 10_000);
        let mut captured = vec![0; 2 * 500];
        captured.extend_from_slice(&reference[2 * 300..2 * 9000]);
        let c = bit_exact(&reference, &captured, 2, 4800).unwrap();
        assert_eq!(
            (c.offset, c.capture_start, c.compared, c.exact),
            (300, 500, 8700, true)
        );
    }

    #[test]
    fn one_lsb_off_is_found() {
        let reference = noise(2 * 4000);
        let mut captured = reference.clone();
        captured[2 * 3000 + 1] += 1;
        let c = bit_exact(&reference, &captured, 2, 100).unwrap();
        assert!(!c.exact);
        assert_eq!((c.first_mismatch, c.max_lsb), (Some(3000), 1));
    }

    #[test]
    fn a_scaled_capture_matches_nowhere() {
        let reference = noise(4000);
        let captured: Vec<i64> = reference.iter().map(|v| v / 2).collect();
        assert_eq!(bit_exact(&reference, &captured, 1, 1000), None);
        assert_eq!(bit_exact(&reference, &[0; 100], 1, 10), None, "silence");
    }
}
