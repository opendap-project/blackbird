//! Recording the guest's sound: through the simulator window when it runs, or from QEMU's
//! D-Bus audio directly.

use super::*;

pub(crate) enum Msg {
    Init(Format),
    Data(Vec<u8>),
}

pub(crate) struct Rec(pub(crate) Mutex<mpsc::Sender<Msg>>);

impl Sink for Rec {
    fn init(&self, _: u64, f: audio::Format) {
        let f = Format {
            rate: f.rate,
            channels: f.channels,
            bits: f.bits,
            signed: f.signed,
            float: f.float,
            big_endian: f.big_endian,
        };
        let _ = self
            .0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .send(Msg::Init(f));
    }
    fn fini(&self, _: u64) {}
    fn enabled(&self, _: u64, _: bool) {}
    fn write(&self, _: u64, data: &[u8]) {
        let _ = self
            .0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .send(Msg::Data(data.to_vec()));
    }
}

pub(crate) fn record(args: &[String]) -> Result<(), String> {
    let qmp = opt(args, "--qmp").ok_or("record needs --qmp SOCKET")?;
    let out = PathBuf::from(opt(args, "--out").ok_or("record needs --out FILE.wav")?);
    let seconds: f64 = opt(args, "--seconds")
        .map_or(Ok(5.0), |s| s.parse())
        .map_err(|_| "--seconds: a number")?;
    let wait: f64 = opt(args, "--wait")
        .map_or(Ok(30.0), |s| s.parse())
        .map_err(|_| "--wait: a number")?;
    let direction = opt(args, "--direction").unwrap_or_else(|| "out".into());
    let window = Path::new(&qmp).with_file_name("view.sock");
    if let Ok(stream) = std::os::unix::net::UnixStream::connect(&window) {
        return record_through_window(stream, args, &direction, &out, seconds, wait);
    }
    if direction != "out" {
        return Err(
            "recording the input needs the simulator window (it supplies the input)".into(),
        );
    }
    record_from_qemu(&qmp, args, &out, seconds, wait)
}

/// The guest's output straight from QEMU's D-Bus audio (no window to ask).
fn record_from_qemu(
    qmp: &str,
    args: &[String],
    out: &Path,
    seconds: f64,
    wait: f64,
) -> Result<(), String> {
    let (tx, rx) = mpsc::channel();
    let session =
        blackbird_emulator_qemu_display::connect(Path::new(&qmp)).map_err(|e| e.to_string())?;
    let _listening = session
        .audio_out(Arc::new(Rec(Mutex::new(tx))))
        .map_err(|e| format!("{e} (is the device started with the D-Bus audio?)"))?;
    let mut child = match opt(args, "--run") {
        Some(cmd) => Some(
            Command::new("sh")
                .args(["-c", &cmd])
                .spawn()
                .map_err(|e| format!("--run: {e}"))?,
        ),
        None => None,
    };
    let (file, format, written) = capture(&rx, out, seconds, wait)?;
    if let Some(c) = child.as_mut() {
        let _ = c.wait();
    }
    let (Some(w), Some(f)) = (file, format) else {
        return Err("nothing recorded".into());
    };
    let mut f_out = w.into_inner().map_err(|e| e.to_string())?;
    wav::finalize(&mut f_out, &f, written as u32).map_err(|e| e.to_string())?;
    let secs = written as f64 / f.bytes_per_frame() as f64 / f64::from(f.rate);
    println!("{} ({}, {secs:.2} s)", out.display(), f.describe());
    Ok(())
}

/// What the guest plays, from its first sound, written to `out` for `seconds`; ends when the
/// guest stops playing or changes its format. The file (still open), its format and the bytes.
type Captured = (Option<BufWriter<File>>, Option<Format>, usize);

fn capture(
    rx: &mpsc::Receiver<Msg>,
    out: &Path,
    seconds: f64,
    wait: f64,
) -> Result<Captured, String> {
    let deadline = Instant::now() + Duration::from_secs_f64(wait);
    // QEMU announces the stream when the listener registers, and again when the guest reopens it.
    let mut format: Option<Format> = None;
    let mut file: Option<BufWriter<File>> = None;
    let (mut written, mut wanted) = (0usize, 0usize);
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let msg = match rx.recv_timeout(if file.is_some() {
            Duration::from_secs(5)
        } else {
            left
        }) {
            Ok(m) => m,
            Err(_) if file.is_none() => return Err(format!("nothing played in {wait} s")),
            Err(_) => break, // the guest stopped playing: what it played is kept
        };
        match msg {
            Msg::Init(f) => {
                if file.is_some() && format != Some(f) {
                    eprintln!(
                        "blackbird-emulator-audio: the guest changed its format ({}): stopped there",
                        f.describe()
                    );
                    break;
                }
                format = Some(f);
            }
            Msg::Data(d) => {
                let Some(f) = format else { continue };
                if file.is_none() {
                    // From the first sound: leading digital silence is not recorded.
                    if d.iter().all(|&b| b == 0) {
                        continue;
                    }
                    let mut w = BufWriter::new(
                        File::create(out).map_err(|e| format!("{}: {e}", out.display()))?,
                    );
                    wav::write_header(&mut w, &f, 0).map_err(|e| e.to_string())?;
                    file = Some(w);
                    wanted = (seconds * f64::from(f.rate)) as usize * f.bytes_per_frame();
                }
                let take = d.len().min(wanted - written);
                file.as_mut()
                    .unwrap()
                    .write_all(&d[..take])
                    .map_err(|e| e.to_string())?;
                written += take;
                if written >= wanted {
                    break;
                }
            }
        }
    }
    Ok((file, format, written))
}

/// `record` asked of the simulator window over its control socket.
pub(crate) fn record_through_window(
    stream: std::os::unix::net::UnixStream,
    args: &[String],
    direction: &str,
    out: &Path,
    seconds: f64,
    wait: f64,
) -> Result<(), String> {
    use std::io::{BufRead, BufReader};
    let out = std::path::absolute(out).map_err(|e| e.to_string())?;
    let req =
        serde_json::json!({ "record": direction, "seconds": seconds, "wait": wait, "path": out });
    let mut w = &stream;
    writeln!(w, "{req}").map_err(|e| e.to_string())?;
    let mut lines = BufReader::new(&stream).lines();
    let mut next = || -> Result<serde_json::Value, String> {
        let line = lines
            .next()
            .ok_or("the window closed the connection")?
            .map_err(|e| e.to_string())?;
        serde_json::from_str(&line).map_err(|e| e.to_string())
    };
    let first = next()?;
    if let Some(e) = first["error"].as_str() {
        return Err(e.to_string());
    }
    let mut child = match opt(args, "--run") {
        Some(cmd) => Some(
            Command::new("sh")
                .args(["-c", &cmd])
                .spawn()
                .map_err(|e| format!("--run: {e}"))?,
        ),
        None => None,
    };
    let last = next()?;
    if let Some(c) = child.as_mut() {
        let _ = c.wait();
    }
    if let Some(e) = last["error"].as_str() {
        return Err(e.to_string());
    }
    println!(
        "{} ({}, {:.2} s)",
        out.display(),
        last["format"].as_str().unwrap_or("?"),
        last["seconds"].as_f64().unwrap_or(0.0)
    );
    Ok(())
}
