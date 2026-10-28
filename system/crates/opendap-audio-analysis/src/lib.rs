//! The audio measurements of OpenDAP: what a level meter, a spectrum and an audio analyzer read
//! from a PCM stream, precise enough to audit the audio path (bit-perfect transport, THD+N, the
//! noise floor), so engineers have monitoring and analysis one step away (SYS-041). Pure and
//! platform-agnostic: the virtual T1's instruments, a recording CLI and the device share it.
//!
//! Everything is sized at construction and nothing grows with time, so a meter left running for
//! days costs what it cost in its first second (T1-046, T1-047). No threads, no I/O but [`wav`]
//! on `std::io` traits.
//!
//! Levels are dBFS with a full-scale sine at 0 dBFS (its peak touches full scale; AES17), so
//! its RMS reads -3.01 dBFS. Silence reads [`FLOOR_DBFS`], never -inf or NaN.
//!
//! - [`Format`], [`decode`], [`decode_exact`]: PCM to f32, or to exact integers.
//! - [`meter`]: peak with a hold for a UI, 300 ms RMS, clipped samples, DC.
//! - [`spectrum`]: a windowed FFT in dBFS, display bands and the peak.
//! - [`tone`]: a sine test signal's analysis: frequency, level, THD, THD+N, SNR.
//! - [`compare`]: whether a capture carries a reference bit for bit.
//! - [`wav`]: RIFF WAVE read and write.

pub mod compare;
pub mod meter;
mod pcm;
pub mod spectrum;
pub mod tone;
pub mod wav;

pub use pcm::{Format, decode, decode_exact, encode_exact, encode_float};

/// The level silence reads, in dBFS: below anything a stream can carry (a 32-bit LSB is
/// -192.7 dBFS; a float stream's smallest normal is lower, and reads this floor).
pub const FLOOR_DBFS: f32 = -240.0;

/// An amplitude ratio (1 = full scale) in dB, floored at [`FLOOR_DBFS`].
pub fn amplitude_db(ratio: f64) -> f32 {
    if ratio > 0.0 {
        ((20.0 * ratio.log10()) as f32).max(FLOOR_DBFS)
    } else {
        FLOOR_DBFS
    }
}

/// A power ratio in dB, floored at [`FLOOR_DBFS`].
pub fn power_db(ratio: f64) -> f32 {
    if ratio > 0.0 {
        ((10.0 * ratio.log10()) as f32).max(FLOOR_DBFS)
    } else {
        FLOOR_DBFS
    }
}
