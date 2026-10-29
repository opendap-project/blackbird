//! Prints the guest's audio streams as they arrive: formats, bytes, peak per second.
//! Usage: audio_probe QMP_SOCKET [SECONDS [RAW_OUTPUT_FILE]]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use blackbird_emulator_qemu_display::audio::{Format, Sink, Source};

struct Probe(Mutex<(u64, Instant)>, Mutex<Option<std::fs::File>>);

impl Sink for Probe {
    fn init(&self, s: u64, f: Format) {
        println!("out init {s:x} {f:?}");
    }
    fn fini(&self, s: u64) {
        println!("out fini {s:x}");
    }
    fn enabled(&self, s: u64, on: bool) {
        println!("out enabled {s:x} {on}");
    }
    fn volume(&self, s: u64, mute: bool, v: &[u8]) {
        println!("out volume {s:x} mute={mute} {v:?}");
    }
    fn write(&self, s: u64, data: &[u8]) {
        if let Some(f) = self.1.lock().unwrap().as_mut() {
            use std::io::Write;
            f.write_all(data).unwrap();
        }
        let mut g = self.0.lock().unwrap();
        g.0 += data.len() as u64;
        if g.1.elapsed() > Duration::from_secs(1) {
            println!(
                "out {s:x}: {} bytes/s, first bytes {:?}",
                g.0,
                &data[..data.len().min(8)]
            );
            *g = (0, Instant::now());
        }
    }
}

/// Silence, paced by the clock (QEMU's D-Bus input does not pace its reads); each read logged.
struct Tone(Mutex<(Option<Instant>, u64, u32, usize)>);
impl Source for Tone {
    fn init(&self, s: u64, f: Format) {
        println!("in init {s:x} {f:?}");
        let mut g = self.0.lock().unwrap();
        g.2 = f.rate;
        g.3 = f.bytes_per_frame as usize;
    }
    fn fini(&self, s: u64) {
        println!("in fini {s:x}");
    }
    fn enabled(&self, s: u64, on: bool) {
        println!("in enabled {s:x} {on}");
        let mut g = self.0.lock().unwrap();
        g.0 = on.then(Instant::now);
        g.1 = 0;
    }
    fn read(&self, _s: u64, size: usize) -> Vec<u8> {
        let mut g = self.0.lock().unwrap();
        let elapsed =
            g.0.map_or(0, |t| (t.elapsed().as_secs_f64() * g.2 as f64) as u64);
        let frames = (elapsed.saturating_sub(g.1) as usize).min(size / g.3.max(1));
        g.1 += frames as u64;
        if std::env::var_os("PROBE_LOG").is_some() {
            println!(
                "read {size} -> {} at {:.3}",
                frames * g.3,
                g.0.map_or(0.0, |t| t.elapsed().as_secs_f64())
            );
        }
        vec![0; frames * g.3]
    }
}

fn main() {
    let mut a = std::env::args().skip(1);
    let qmp = a.next().expect("QMP socket");
    let secs: u64 = a.next().map(|s| s.parse().unwrap()).unwrap_or(10);
    let dump = a.next().map(|p| std::fs::File::create(p).unwrap());
    let session = blackbird_emulator_qemu_display::connect(qmp.as_ref()).expect("connect");
    let _o = session
        .audio_out(Arc::new(Probe(
            Mutex::new((0, Instant::now())),
            Mutex::new(dump),
        )))
        .expect("out listener");
    let _i = session
        .audio_in(Arc::new(Tone(Mutex::new((None, 0, 48000, 2)))))
        .expect("in listener");
    println!("listening");
    std::thread::sleep(Duration::from_secs(secs));
}
