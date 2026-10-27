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

mod analyze;
mod cli;
mod exact;
mod picture;
mod record;

use analyze::*;
use cli::*;
use exact::*;
use picture::*;
use record::*;

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
