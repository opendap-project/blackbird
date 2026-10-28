//! Whether a capture carries a reference bit for bit.

use super::*;

pub(crate) fn compare_files(args: &[String]) -> Result<(), String> {
    let p = positional(args);
    let (Some(rp), Some(cp)) = (p.first(), p.get(1)) else {
        return Err("compare needs REFERENCE.wav CAPTURE.wav".into());
    };
    let (rf, rd) = load(rp)?;
    let (cf, cd) = load(cp)?;
    let mut fmt_note = String::new();
    if (rf.rate, rf.channels, rf.bits, rf.float) != (cf.rate, cf.channels, cf.bits, cf.float) {
        fmt_note = format!(
            "formats differ: {} against {}",
            rf.describe(),
            cf.describe()
        );
    }
    let (mut r, mut c) = (Vec::new(), Vec::new());
    decode_exact(&rf, &rd, &mut r).ok_or("the reference is float: bit-exact compares integers")?;
    decode_exact(&cf, &cd, &mut c).ok_or("the capture is float: bit-exact compares integers")?;
    let ch = usize::from(rf.channels.max(1));
    let frames = r.len() / ch;
    let mut result = if fmt_note.is_empty() {
        compare::bit_exact(&r, &c, ch, frames)
    } else {
        None
    };
    // The capture may begin with sound that is not the reference's (another stream's tail held
    // back by the sound path): find the reference's own start in the capture, compare from there,
    // and report what came before it.
    let mut leading = 0usize;
    if result.is_none() && fmt_note.is_empty() {
        (result, leading) = realigned(&rf, &rd, &cd, &r, &c);
    }
    let report = serde_json::json!({
        "reference": rp, "capture": cp,
        "format": rf.describe(),
        "exact": result.is_some_and(|x| x.exact),
        "aligned": result.is_some(),
        "note": fmt_note,
        "offset_frames": result.map(|x| x.offset),
        "capture_start_frames": result.map(|x| x.capture_start),
        "compared_frames": result.map(|x| x.compared),
        "reference_frames": frames,
        "first_mismatch_frame": result.and_then(|x| x.first_mismatch),
        "max_lsb": result.map(|x| x.max_lsb),
        "leading_frames": leading,
    });
    if flag(args, "--json") {
        println!("{report:#}");
    } else {
        print_text(result, frames, &rf, leading, &fmt_note);
    }
    if result.is_some_and(|x| x.exact) && leading == 0 {
        Ok(())
    } else if result.is_some_and(|x| x.exact) {
        Err("bit-exact, after sound that is not the reference's".into())
    } else {
        Err("not bit-exact".into())
    }
}

/// The reference's own start found in the capture, compared from there: the comparison, and
/// how many frames of other sound came before it.
fn realigned(
    rf: &Format,
    rd: &[u8],
    cd: &[u8],
    r: &[i64],
    c: &[i64],
) -> (Option<compare::Comparison>, usize) {
    let ch = usize::from(rf.channels.max(1));
    let bpf = rf.bytes_per_frame();
    let first = rd
        .chunks_exact(bpf)
        .position(|f| f.iter().any(|&b| b != 0))
        .unwrap_or(0);
    let probe = &rd[first * bpf..((first + 256) * bpf).min(rd.len())];
    let found = cd
        .windows(probe.len().max(1))
        .enumerate()
        .find(|(i, w)| i % bpf == 0 && *w == probe)
        .map(|(i, _)| i / bpf);
    let Some(k) = found else { return (None, 0) };
    let result = compare::bit_exact(&r[first * ch..], &c[k * ch..], ch, 0).map(|mut x| {
        x.offset += first;
        x.capture_start = k;
        x
    });
    (result, k.saturating_sub(first))
}

/// The comparison as lines for a person.
fn print_text(
    result: Option<compare::Comparison>,
    frames: usize,
    rf: &Format,
    leading: usize,
    fmt_note: &str,
) {
    match result {
        Some(x) if x.exact => {
            println!(
                "bit-exact: {} of the reference's {} frames ({}), from reference frame {}",
                x.compared,
                frames,
                rf.describe(),
                x.offset
            );
            if leading > 0 {
                println!(
                    "but {leading} frames ({:.1} ms) of other sound come before it (another stream's tail?)",
                    leading as f64 * 1000.0 / f64::from(rf.rate)
                );
            }
        }
        Some(x) => println!(
            "NOT bit-exact: first difference at frame {} of {} compared, up to {} LSB",
            x.first_mismatch.unwrap_or(0),
            x.compared,
            x.max_lsb
        ),
        None if !fmt_note.is_empty() => println!("NOT bit-exact: {fmt_note}"),
        None => println!(
            "NOT bit-exact: the capture's first sound is nowhere in the reference (changed from its start: a gain, a resampler, dither)"
        ),
    }
}
