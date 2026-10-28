//! The command line: usage, options and positionals, and how a failure is reported.

use super::*;

pub(crate) const USAGE: &str = "usage:
  odap-vdev-audio record --qmp SOCKET [--direction out|in] [--seconds S] [--wait S] [--run CMD] --out FILE.wav
  odap-vdev-audio analyze FILE.wav [--from S] [--to S] [--bands N] [--blocks S] [--json]
  odap-vdev-audio compare REFERENCE.wav CAPTURE.wav [--json]
  odap-vdev-audio spectrum FILE.wav --out FILE.png [--from S] [--to S]
  odap-vdev-audio spectrogram FILE.wav --out FILE.png [--from S] [--to S] [--channel N]

  record    the guest's output, exactly as the guest played it (or, --direction in, what it was
            given to record), from its first sound for --seconds (default 5); --run starts CMD
            (sh -c) once listening, to make the guest play; gives up after --wait seconds
            (default 30) of nothing. With the simulator window open it asks the window (its
            view.sock, beside the QMP socket): QEMU's D-Bus display serves one client, and a
            second would displace the window. Without it, it attaches to QEMU itself (output
            only). QMP serves one client per socket: use odap-vdev's (status: qmp).
  analyze   format; per channel the sample peak, RMS, DC and samples at full scale; the loudest
            frequency; a sine's frequency, level, THD, THD+N (20 Hz-20 kHz) and SNR when the file
            carries one (on --from..--to, default its middle); --bands N log bands from 20 Hz to
            20 kHz (dBFS, the highest bin in each); --blocks S analyses the whole file S seconds at
            a time and reports the worst block (a dropout, a click or a phase jump in a continuous
            sine shows as one block's THD+N far above the others) and the longest run of exact
            zeros.
  compare   whether CAPTURE carries REFERENCE bit for bit (past leading silence and a late start):
            the alignment, the frames compared, the first difference and its size in LSB. Sound
            before the reference begins (another stream's tail the sound path held back) is found
            and reported as leading frames; the comparison then starts at the reference's own
            start, and the exit is non-zero.
  spectrum  the averaged spectrum as a PNG (1200 x 480: 20 Hz to Nyquist on a log axis, 0 to
            -160 dBFS, lines every decade and every 20 dB), for a person or an agent to look at.
  spectrogram  time across (1600 px for the whole file or --from..--to), 20 Hz to Nyquist up a log
            axis (600 px, lines at 100 Hz, 1 kHz, 10 kHz), level as colour (0 dBFS bright yellow
            to -160 dBFS black, inferno). Each column is the loudest of every 4096-point
            transform in its span, so a click of a millisecond in ten minutes still shows: a
            dropout or a click is a vertical line, a spur a horizontal one. Mono mix, or
            --channel N (from 1).";

pub(crate) fn fail(msg: impl std::fmt::Display) -> ExitCode {
    eprintln!("odap-vdev-audio: {msg}");
    ExitCode::FAILURE
}

/// `--name value` from the arguments.
pub(crate) fn opt(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

pub(crate) fn flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

/// The positional arguments (those not an option or its value).
pub(crate) fn positional(args: &[String]) -> Vec<String> {
    let valued = [
        "--qmp",
        "--direction",
        "--seconds",
        "--wait",
        "--run",
        "--out",
        "--from",
        "--to",
        "--bands",
        "--blocks",
        "--channel",
    ];
    let mut out = Vec::new();
    let mut skip = false;
    for a in args {
        if skip {
            skip = false;
        } else if valued.contains(&a.as_str()) {
            skip = true;
        } else if !a.starts_with("--") {
            out.push(a.clone());
        }
    }
    out
}
