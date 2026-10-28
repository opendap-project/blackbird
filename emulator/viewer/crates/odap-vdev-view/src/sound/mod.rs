//! The device's sound in the simulator window (ADR 0003): QEMU's D-Bus audio gives the PCM the
//! guest plays, in the guest's own format, and asks for what it records.
//!
//! - Output: played on the host's default output, and measured (meters, a spectrum, a sine's
//!   THD+N) on the exact stream, before anything the host does to it. When the host takes the
//!   guest's rate and channels as they are, it plays them unchanged; otherwise a linear
//!   resampler adapts them, for listening only (the reading says so).
//! - Input: the guest records one of the host's inputs (any of them, the system's default
//!   first), a test tone (997 Hz, AES17's, at -6 dBFS) or silence; what it receives, quantized to
//!   its format, is measured the same way.
//! - The host's streams live on a thread of their own (cpal's are not `Send` everywhere); the
//!   listeners hand samples to them through rings of 250 ms, so a stall drops sound instead of
//!   growing a queue (T1-046).
//! - Readings are published at most every 100 ms, and only while samples arrive with analysis
//!   on (the window turns it on only while its Instruments page shows): an idle device, or a
//!   window looking elsewhere, costs nothing.
//! - The analysis runs on a thread of its own: QEMU waits for the input's `Read` on its audio
//!   path, so a listener only copies the samples to a bounded channel and answers; when the
//!   analysis falls behind, samples for it are dropped, never the guest's sound.

use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use opendap_audio_analysis::meter::{Level, Meter};
use opendap_audio_analysis::spectrum::{self, Spectrum};
use opendap_audio_analysis::{Format, decode, encode_exact, encode_float, tone};
use opendap_qemu_display::Session;
use opendap_qemu_display::audio::{self, Listening, Sink, Source};

mod analysis;
mod host;
mod listeners;
mod pcm;
mod tap;

use analysis::*;
use host::*;
use listeners::*;
use pcm::*;
use tap::*;

/// The spectrum's display bands, log-spaced from 20 Hz to 20 kHz: one per logical pixel of the
/// strip (instruments.rs), so a peak is drawn at its level.
pub const BANDS: usize = (crate::instruments::STRIP_W / 2) as usize;
/// The spectrogram's rows: one per pixel of the strip's height (instruments.rs).
pub const ROWS: usize = crate::instruments::STRIP_H as usize;
pub const BAND_LO_HZ: f32 = 20.0;
pub const BAND_HI_HZ: f32 = 20_000.0;
pub const TEST_TONE_HZ: f64 = 997.0;
pub const TEST_TONE_DBFS: f64 = -6.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Direction {
    Output,
    Input,
}

/// What the guest records.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum InSource {
    /// One of the host's inputs, by its id; None for the system's default.
    Device(Option<String>),
    Tone,
    Silence,
}

/// One of the host's inputs.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct InputDevice {
    pub id: String,
    pub name: String,
}

/// The host's inputs, and the name of the system's default one.
pub fn input_devices() -> (Vec<InputDevice>, Option<String>) {
    let host = cpal::default_host();
    let name = |d: &cpal::Device| d.description().map(|x| x.name().to_string()).ok();
    let default = host.default_input_device().as_ref().and_then(name);
    let list = host
        .input_devices()
        .map(|it| {
            it.filter_map(|d| {
                Some(InputDevice {
                    id: d.id().ok()?.to_string(),
                    name: name(&d)?,
                })
            })
            .collect()
        })
        .unwrap_or_default();
    (list, default)
}

/// A direction's state, for the window.
#[derive(Clone, Debug, Default)]
pub struct Reading {
    /// A stream runs.
    pub active: bool,
    /// The stream's format, for a person.
    pub format: String,
    pub levels: Vec<Level>,
    /// dBFS per display band ([`BANDS`]).
    pub bands: Vec<f32>,
    /// dBFS per spectrogram row ([`ROWS`], low to high), for the waterfall.
    pub rows: Vec<f32>,
    pub peak_hz: f32,
    pub peak_dbfs: f32,
    /// A sine's analysis, when the stream carries one.
    pub tone: Option<tone::Report>,
    /// What the host does differently (resampling, no device), or an error.
    pub note: String,
}

fn idle(note: &str) -> Reading {
    Reading {
        note: note.to_string(),
        ..Default::default()
    }
}

fn format_of(f: audio::Format) -> Format {
    Format {
        rate: f.rate,
        channels: f.channels,
        bits: f.bits,
        signed: f.signed,
        float: f.float,
        big_endian: f.big_endian,
    }
}

type OnReading = Arc<dyn Fn(Direction, Reading) + Send + Sync>;

/// How a recording ended: its format and seconds, or why it failed.
pub type Recorded = Result<(Format, f64), String>;

#[derive(Default)]
struct Out {
    /// Counts openings of the host's output: an answer belongs to the latest only.
    opening: u64,
    tap: Option<Tap>,
    format: Option<Format>,
    analyze: bool,
    play: bool,
    host: Option<(Arc<Mutex<Ring>>, Resampler)>,
    decoded: Vec<f32>,
    converted: Vec<f32>,
    note: String,
}

struct In {
    /// Counts openings of the host's input: an answer belongs to the latest only.
    opening: u64,
    tally: ReadTally,
    tap: Option<Tap>,
    format: Option<Format>,
    analyze: bool,
    source: InSource,
    enabled: bool,
    microphone: Option<Arc<Mutex<Ring>>>,
    phase: f64,
    /// When the guest started recording, and the frames given since: a source runs in real
    /// time, so it gives only what is due (QEMU asks as often as it likes).
    started: Option<Instant>,
    given: u64,
    note: String,
}

struct Shared {
    out: Mutex<Out>,
    inp: Mutex<In>,
    cmd: Mutex<mpsc::Sender<Cmd>>,
    feed: mpsc::SyncSender<Feed>,
    on_reading: OnReading,
    // Whether the input goes through QEMU's mixing engine, which doubles a mono capture (stock
    // QEMU 11.1); false with the patched QEMU that runs the input without it (ADR 0003).
    mono_doubled: bool,
}

/// ODAP_VIEW_TRACE: every stream event, and the input's reads once a second, on stderr
/// (window.log), with the time since the window started: what QEMU did when sound stops.
fn trace(what: std::fmt::Arguments) {
    static ON: std::sync::OnceLock<Option<Instant>> = std::sync::OnceLock::new();
    if let Some(t0) = ON.get_or_init(|| std::env::var_os("ODAP_VIEW_TRACE").map(|_| Instant::now()))
    {
        eprintln!(
            "odap-vdev-view: sound {:.3} {what}",
            t0.elapsed().as_secs_f64()
        );
    }
}

/// The input's reads over the last second, for the trace.
#[derive(Default)]
struct ReadTally {
    since: Option<Instant>,
    calls: u64,
    asked: u64,
    given: u64,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

impl Shared {
    fn send(&self, cmd: Cmd) {
        let _ = lock(&self.cmd).send(cmd);
    }

    /// The host's output for the output's format, when it plays.
    /// The host's output for the output's format, when it plays. The host's audio may take a
    /// while to answer: nothing is locked meanwhile, so the listeners never wait for it; a newer
    /// opening supersedes an older one's answer.
    fn open_output(&self) {
        let (f, opening) = {
            let mut out = lock(&self.out);
            out.host = None;
            out.opening += 1;
            (out.format.filter(|_| out.play), out.opening)
        };
        let Some(f) = f else {
            self.send(Cmd::CloseOutput);
            return;
        };
        let ring = Arc::new(Mutex::new(Ring::new(f.rate, usize::from(f.channels))));
        let (tx, rx) = mpsc::channel();
        self.send(Cmd::Output {
            rate: f.rate,
            channels: u16::from(f.channels),
            ring: ring.clone(),
            reply: tx,
        });
        let answer = rx.recv_timeout(Duration::from_secs(3));
        let mut out = lock(&self.out);
        if out.opening != opening {
            return;
        }
        match answer {
            Ok(Ok((rate, channels))) => {
                let r = Resampler::new(f.rate, usize::from(f.channels), rate, channels);
                out.note = if r.identity() {
                    String::new()
                } else {
                    format!(
                        "This computer plays it at {} Hz, {channels} ch (resampled for listening only)",
                        rate
                    )
                };
                // The ring's size follows what it carries: the host's rate and channels.
                *lock(&ring) = Ring::new(rate, channels);
                out.host = Some((ring, r));
            }
            Ok(Err(e)) => out.note = format!("Not played on this computer: {e}"),
            Err(_) => out.note = "Not played on this computer: its audio did not answer".into(),
        }
    }

    /// The host's input, the same way: never waited for under the lock QEMU's reads take.
    fn open_microphone(&self) {
        let (request, opening) = {
            let mut inp = lock(&self.inp);
            inp.microphone = None;
            inp.opening += 1;
            let request = match (inp.source.clone(), inp.format.filter(|_| inp.enabled)) {
                (InSource::Device(device), Some(f)) => Some((device, f)),
                _ => None,
            };
            (request, inp.opening)
        };
        let Some((device, f)) = request else {
            self.send(Cmd::CloseMicrophone);
            return;
        };
        let ring = Arc::new(Mutex::new(Ring::new(f.rate, usize::from(f.channels))));
        let (tx, rx) = mpsc::channel();
        self.send(Cmd::Microphone {
            device,
            rate: f.rate,
            channels: usize::from(f.channels),
            ring: ring.clone(),
            reply: tx,
        });
        let answer = rx.recv_timeout(Duration::from_secs(5));
        let mut inp = lock(&self.inp);
        if inp.opening != opening {
            return;
        }
        match answer {
            Ok(Ok(())) => {
                inp.note.clear();
                inp.microphone = Some(ring);
            }
            Ok(Err(e)) => inp.note = format!("Input: {e}"),
            Err(_) => {
                inp.note = "Input: no answer (the system may be asking for permission)".into()
            }
        }
    }
}

/// The device's sound, attached to its display session.
pub struct Sound {
    shared: Arc<Shared>,
    _out: Listening,
    _in: Listening,
}

impl Sound {
    /// Listens to the guest's output and answers its input. `input_direct`: QEMU runs the input
    /// without its mixing engine (the patched QEMU). `on_reading` runs on a listener's thread.
    pub fn attach(
        session: &Session,
        input_direct: bool,
        on_reading: impl Fn(Direction, Reading) + Send + Sync + 'static,
    ) -> io::Result<Sound> {
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("sound".into())
            .spawn(move || host_thread(rx))?;
        let on_reading: OnReading = Arc::new(on_reading);
        let (feed, feed_rx) = mpsc::sync_channel(FEED_DEPTH);
        let readings = on_reading.clone();
        std::thread::Builder::new()
            .name("sound-analysis".into())
            .spawn(move || analysis_thread(feed_rx, readings))?;
        let shared = Arc::new(Shared {
            out: Mutex::new(Out {
                play: true,
                ..Default::default()
            }),
            inp: Mutex::new(In {
                opening: 0,
                tally: ReadTally::default(),
                tap: None,
                format: None,
                analyze: false,
                source: InSource::Device(None),
                enabled: false,
                microphone: None,
                phase: 0.0,
                started: None,
                given: 0,
                note: String::new(),
            }),
            cmd: Mutex::new(tx),
            feed,
            on_reading,
            mono_doubled: !input_direct,
        });
        let out = session.audio_out(Arc::new(OutListener(shared.clone())))?;
        let inp = session.audio_in(Arc::new(InListener(shared.clone())))?;
        Ok(Sound {
            shared,
            _out: out,
            _in: inp,
        })
    }

    /// Measure a direction, or stop measuring it.
    pub fn set_analyze(&self, direction: Direction, on: bool) {
        let idle_note = match direction {
            Direction::Output => {
                let mut out = lock(&self.shared.out);
                out.analyze = on;
                (!on).then(|| out.note.clone())
            }
            Direction::Input => {
                let mut inp = lock(&self.shared.inp);
                inp.analyze = on;
                (!on).then(|| inp.note.clone())
            }
        };
        if let Some(note) = idle_note {
            // From the window's thread: it may wait for room, the listeners never do.
            let _ = self.shared.feed.send(Feed::Reset(direction));
            (self.shared.on_reading)(direction, idle(&note));
        }
    }

    /// Records `direction`'s exact bytes (what the guest played, or what it was given to
    /// record) to a WAV file at `path`, from its first sound for `seconds`; the answer comes on
    /// the returned channel when it is done, or when it fails. One recording per direction.
    pub fn record(
        &self,
        direction: Direction,
        path: &std::path::Path,
        seconds: f64,
    ) -> Result<mpsc::Receiver<Recorded>, String> {
        let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let (tx, rx) = mpsc::channel();
        let tap = Tap {
            file: std::io::BufWriter::new(file),
            format: None,
            seconds,
            written: 0,
            wanted: 0,
            done: Some(tx),
        };
        let slot = match direction {
            Direction::Output => &mut lock(&self.shared.out).tap,
            Direction::Input => &mut lock(&self.shared.inp).tap,
        };
        if slot.is_some() {
            return Err("a recording of that direction is already running".into());
        }
        *slot = Some(tap);
        Ok(rx)
    }

    /// Ends a direction's recording where it is (its stream stopped, or the caller gave up).
    pub fn stop_recording(&self, direction: Direction) {
        let tap = match direction {
            Direction::Output => lock(&self.shared.out).tap.take(),
            Direction::Input => lock(&self.shared.inp).tap.take(),
        };
        if let Some(mut t) = tap {
            t.finish(Ok(()));
        }
    }

    /// Play the output on this computer, or not.
    pub fn set_play(&self, on: bool) {
        {
            let mut out = lock(&self.shared.out);
            out.play = on;
            if !on {
                out.note.clear();
            }
        }
        self.shared.open_output();
    }

    /// What the guest records.
    pub fn set_source(&self, source: InSource) {
        {
            let mut inp = lock(&self.shared.inp);
            if !matches!(source, InSource::Device(_)) {
                inp.note.clear();
            }
            inp.source = source;
        }
        self.shared.open_microphone();
    }
}
