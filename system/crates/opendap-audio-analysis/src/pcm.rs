//! PCM streams as they arrive (QEMU's D-Bus audio, ALSA, a WAV file): their format, and their
//! samples read two ways. [`decode`] gives f32 in [-1, 1) for meters and spectra; [`decode_exact`]
//! gives each sample as the integer the stream carries, for bit-perfect checks, where a float
//! conversion would hide a one-LSB difference.

/// The PCM format of a stream, as QEMU's D-Bus audio (and ALSA) describe it. `bits` is the
/// container width: 8, 16, 24 (packed in 3 bytes) or 32 for integers, 32 or 64 for floats.
/// Unsigned integers are offset binary (silence at half scale).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Format {
    pub rate: u32,
    pub channels: u8,
    pub bits: u8,
    pub signed: bool,
    pub float: bool,
    pub big_endian: bool,
}

impl Format {
    /// Signed little-endian integers, the common case (`s16le`, `s24le`, `s32le`).
    pub fn int(rate: u32, channels: u8, bits: u8) -> Self {
        Format {
            rate,
            channels,
            bits,
            signed: true,
            float: false,
            big_endian: false,
        }
    }

    /// 32-bit little-endian float (`f32le`).
    pub fn f32(rate: u32, channels: u8) -> Self {
        Format {
            rate,
            channels,
            bits: 32,
            signed: true,
            float: true,
            big_endian: false,
        }
    }

    /// Bytes in one frame: one sample of each channel.
    pub fn bytes_per_frame(&self) -> usize {
        usize::from(self.bits).div_ceil(8) * usize::from(self.channels)
    }

    /// Whether [`decode`] reads this format (and [`decode_exact`], unless it is float).
    pub fn is_supported(&self) -> bool {
        Kind::of(self).is_some()
    }

    /// The largest positive sample as [`decode`] returns it: `1 - 2^-(bits-1)` for integers (the
    /// positive full scale is one LSB short of 1), 1 for float.
    pub fn full_scale(&self) -> f32 {
        if self.float {
            1.0
        } else {
            (1.0 - (-(f64::from(self.bits) - 1.0)).exp2()) as f32
        }
    }

    /// For a person: "48 kHz · 16-bit · 2 ch", "44.1 kHz · 32-bit float · 2 ch".
    pub fn describe(&self) -> String {
        let khz = format!("{:.3}", f64::from(self.rate) / 1000.0);
        let khz = khz.trim_end_matches('0').trim_end_matches('.');
        let kind = if self.float {
            " float"
        } else if !self.signed && self.bits > 8 {
            " unsigned"
        } else {
            ""
        };
        let order = if self.big_endian && self.bits > 8 {
            " BE"
        } else {
            ""
        };
        format!(
            "{khz} kHz · {}-bit{kind}{order} · {} ch",
            self.bits, self.channels
        )
    }
}

#[derive(Debug, Clone, Copy)]
enum Kind {
    Int { width: usize, signed: bool },
    F32,
    F64,
}

impl Kind {
    fn of(f: &Format) -> Option<Kind> {
        if f.channels == 0 {
            return None;
        }
        match (f.float, f.bits) {
            (true, 32) => Some(Kind::F32),
            (true, 64) => Some(Kind::F64),
            (false, 8 | 16 | 24 | 32) => Some(Kind::Int {
                width: usize::from(f.bits / 8),
                signed: f.signed,
            }),
            _ => None,
        }
    }
}

fn raw(s: &[u8], big_endian: bool) -> u64 {
    let mut v = 0u64;
    if big_endian {
        for &b in s {
            v = (v << 8) | u64::from(b);
        }
    } else {
        for &b in s.iter().rev() {
            v = (v << 8) | u64::from(b);
        }
    }
    v
}

/// One integer sample, centred: signed as is, unsigned with its offset removed.
fn int(s: &[u8], signed: bool, big_endian: bool) -> i64 {
    let bits = s.len() as u32 * 8;
    let v = raw(s, big_endian);
    if signed {
        let shift = 64 - bits;
        ((v << shift) as i64) >> shift
    } else {
        v as i64 - (1i64 << (bits - 1))
    }
}

/// Interleaved samples to f32 in [-1, 1): s8/u8, s16/u16, s24/u24 (3 bytes), s32/u32, f32, f64;
/// little- and big-endian. Appends to `out` and returns the bytes consumed: a trailing partial
/// frame is left for the next call. An unsupported format consumes nothing.
pub fn decode(format: &Format, bytes: &[u8], out: &mut Vec<f32>) -> usize {
    let Some(kind) = Kind::of(format) else {
        return 0;
    };
    let bpf = format.bytes_per_frame();
    let used = bytes.len() / bpf * bpf;
    let be = format.big_endian;
    match kind {
        Kind::Int { width, signed } => {
            let scale = 1.0 / (1u64 << (width * 8 - 1)) as f64;
            out.extend(
                bytes[..used]
                    .chunks_exact(width)
                    .map(|s| (int(s, signed, be) as f64 * scale) as f32),
            );
        }
        Kind::F32 => out.extend(
            bytes[..used]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|s| f32::from_bits(raw(s, be) as u32)),
        ),
        Kind::F64 => out.extend(
            bytes[..used]
                .as_chunks::<8>()
                .0
                .iter()
                .map(|s| f64::from_bits(raw(s, be)) as f32),
        ),
    }
    used
}

/// The exact integer view for bit-perfect checks: each sample as i64 at the stream's own bit depth
/// (unsigned formats re-centred, so silence is 0 in both). Appends to `out` and returns the bytes
/// consumed, like [`decode`]. None for float formats, whose samples are not integers, and for
/// unsupported ones.
pub fn decode_exact(format: &Format, bytes: &[u8], out: &mut Vec<i64>) -> Option<usize> {
    let Some(Kind::Int { width, signed }) = Kind::of(format) else {
        return None;
    };
    let bpf = format.bytes_per_frame();
    let used = bytes.len() / bpf * bpf;
    out.extend(
        bytes[..used]
            .chunks_exact(width)
            .map(|s| int(s, signed, format.big_endian)),
    );
    Some(used)
}

/// Samples as bytes in `format`, the inverse of [`decode_exact`] (for writing a capture or a test
/// signal). Values are clamped to the format's range. Nothing is written for float or unsupported
/// formats; returns whether it wrote.
pub fn encode_exact(format: &Format, samples: &[i64], out: &mut Vec<u8>) -> bool {
    let Some(Kind::Int { width, signed }) = Kind::of(format) else {
        return false;
    };
    let bits = width as u32 * 8;
    let (lo, hi) = (-(1i64 << (bits - 1)), (1i64 << (bits - 1)) - 1);
    for &s in samples {
        let v = s.clamp(lo, hi);
        let u = if signed {
            v as u64
        } else {
            (v + (1i64 << (bits - 1))) as u64
        };
        for i in 0..width {
            let byte = if format.big_endian { width - 1 - i } else { i };
            out.push((u >> (8 * byte)) as u8);
        }
    }
    true
}

/// f32 samples as bytes in a float `format` (32 or 64 bits). Returns whether it wrote.
pub fn encode_float(format: &Format, samples: &[f32], out: &mut Vec<u8>) -> bool {
    match (Kind::of(format), format.big_endian) {
        (Some(Kind::F32), false) => samples.iter().for_each(|s| out.extend(s.to_le_bytes())),
        (Some(Kind::F32), true) => samples.iter().for_each(|s| out.extend(s.to_be_bytes())),
        (Some(Kind::F64), false) => samples
            .iter()
            .for_each(|s| out.extend(f64::from(*s).to_le_bytes())),
        (Some(Kind::F64), true) => samples
            .iter()
            .for_each(|s| out.extend(f64::from(*s).to_be_bytes())),
        _ => return false,
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(bits: u8, signed: bool, float: bool, big_endian: bool) -> Format {
        Format {
            rate: 48_000,
            channels: 2,
            bits,
            signed,
            float,
            big_endian,
        }
    }

    #[test]
    fn describes_like_a_person() {
        assert_eq!(
            Format::int(48_000, 2, 16).describe(),
            "48 kHz · 16-bit · 2 ch"
        );
        assert_eq!(
            Format::f32(44_100, 2).describe(),
            "44.1 kHz · 32-bit float · 2 ch"
        );
        assert_eq!(
            Format::int(22_050, 1, 24).describe(),
            "22.05 kHz · 24-bit · 1 ch"
        );
        assert_eq!(
            Format::int(384_000, 8, 32).describe(),
            "384 kHz · 32-bit · 8 ch"
        );
    }

    #[test]
    fn frame_sizes() {
        assert_eq!(Format::int(48_000, 2, 16).bytes_per_frame(), 4);
        assert_eq!(Format::int(48_000, 2, 24).bytes_per_frame(), 6);
        assert_eq!(Format::f32(48_000, 6).bytes_per_frame(), 24);
        assert_eq!(fmt(8, false, false, false).bytes_per_frame(), 2);
    }

    #[test]
    fn signed_16_bit_little_endian() {
        let bytes = [0x00, 0x80, 0xff, 0x7f, 0x00, 0x00, 0x00, 0x40, 0x12]; // trailing partial frame
        let mut out = Vec::new();
        assert_eq!(decode(&fmt(16, true, false, false), &bytes, &mut out), 8);
        assert_eq!(out, [-1.0, 32767.0 / 32768.0, 0.0, 0.5]);
        let mut exact = Vec::new();
        assert_eq!(
            decode_exact(&fmt(16, true, false, false), &bytes, &mut exact),
            Some(8)
        );
        assert_eq!(exact, [-32768, 32767, 0, 16384]);
    }

    #[test]
    fn big_endian_and_unsigned() {
        let mut out = Vec::new();
        decode(
            &fmt(16, true, false, true),
            &[0x80, 0x00, 0x40, 0x00],
            &mut out,
        );
        assert_eq!(out, [-1.0, 0.5]);
        out.clear();
        decode(&fmt(8, false, false, false), &[0x80, 0x00], &mut out);
        assert_eq!(out, [0.0, -1.0]);
        let mut exact = Vec::new();
        decode_exact(
            &fmt(16, false, false, true),
            &[0x80, 0x00, 0xff, 0xff],
            &mut exact,
        );
        assert_eq!(exact, [0, 32767]);
    }

    #[test]
    fn packed_24_bit() {
        let bytes = [0x00, 0x00, 0x80, 0xff, 0xff, 0x7f];
        let mut exact = Vec::new();
        decode_exact(&fmt(24, true, false, false), &bytes, &mut exact);
        assert_eq!(exact, [-(1 << 23), (1 << 23) - 1]);
        let mut out = Vec::new();
        decode(&fmt(24, true, false, false), &bytes, &mut out);
        assert_eq!(out[0], -1.0);
    }

    #[test]
    fn signed_32_bit_and_floats() {
        let mut exact = Vec::new();
        decode_exact(
            &fmt(32, true, false, false),
            &i32::MIN.to_le_bytes().repeat(2),
            &mut exact,
        );
        assert_eq!(exact, [i64::from(i32::MIN); 2]);
        let mut out = Vec::new();
        let mut bytes = 0.25f32.to_be_bytes().to_vec();
        bytes.extend((-0.5f32).to_be_bytes());
        decode(&fmt(32, true, true, true), &bytes, &mut out);
        assert_eq!(out, [0.25, -0.5]);
        assert_eq!(
            decode_exact(&fmt(32, true, true, false), &bytes, &mut exact),
            None
        );
    }

    #[test]
    fn unsupported_consumes_nothing() {
        let mut out = Vec::new();
        assert_eq!(decode(&fmt(12, true, false, false), &[0; 8], &mut out), 0);
        assert!(out.is_empty());
        assert!(!fmt(16, true, true, false).is_supported());
    }

    #[test]
    fn full_scale() {
        assert_eq!(Format::int(48_000, 2, 16).full_scale(), 32767.0 / 32768.0);
        assert_eq!(Format::f32(48_000, 2).full_scale(), 1.0);
    }
}
