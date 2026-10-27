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

/// The spectrum's display bands, log-spaced from 20 Hz to 20 kHz: one per logical pixel of the
/// strip (instruments.rs), so a peak is drawn at its level.
pub const BANDS: usize = (crate::instruments::STRIP_W / 2) as usize;
/// The spectrogram's rows: one per pixel of the strip's height (instruments.rs).
pub const ROWS: usize = crate::instruments::STRIP_H as usize;
pub const BAND_LO_HZ: f32 = 20.0;
pub const BAND_HI_HZ: f32 = 20_000.0;
const PUBLISH_EVERY: Duration = Duration::from_millis(100);
const TONE_EVERY: Duration = Duration::from_secs(1);
const SPECTRUM_SIZE: usize = 8192;
const TONE_SAMPLES: usize = 32_768;
const RING_S: f64 = 0.25;
/// Sound buffered before the host starts playing, and again after it ran dry.
const PRIME_S: f64 = 0.06;
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

/// A bounded queue of interleaved samples between a listener and a host stream.
struct Ring {
    q: VecDeque<f32>,
    cap: usize,
    prime: usize,
    primed: bool,
}

impl Ring {
    fn new(rate: u32, channels: usize) -> Self {
        let per_s = f64::from(rate) * channels as f64;
        Ring {
            q: VecDeque::with_capacity((per_s * RING_S) as usize),
            cap: (per_s * RING_S) as usize,
            prime: (per_s * PRIME_S) as usize,
            primed: false,
        }
    }

    fn push(&mut self, samples: &[f32]) {
        self.q.extend(samples);
        let over = self.q.len().saturating_sub(self.cap);
        self.q.drain(..over);
    }

    /// Fills `out`, with silence while priming or once dry.
    fn pull(&mut self, out: &mut [f32]) {
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
struct Resampler {
    step: f64,
    pos: f64,
    last: Vec<f32>,
    in_ch: usize,
    out_ch: usize,
}

impl Resampler {
    fn new(in_rate: u32, in_ch: usize, out_rate: u32, out_ch: usize) -> Self {
        Resampler {
            step: f64::from(in_rate) / f64::from(out_rate),
            pos: 0.0,
            last: vec![0.0; in_ch],
            in_ch,
            out_ch,
        }
    }

    fn identity(&self) -> bool {
        self.step == 1.0 && self.in_ch == self.out_ch
    }

    fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
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

/// A direction's measurements, sized for its stream.
struct Analysis {
    channels: usize,
    meter: Meter,
    spectrum: Spectrum,
    mono: Vec<f32>,
    tone_ring: VecDeque<f32>,
    tone: Option<tone::Report>,
    last_publish: Instant,
    last_tone: Instant,
}

impl Analysis {
    fn new(f: &Format) -> Self {
        let past = Instant::now() - TONE_EVERY;
        Analysis {
            channels: usize::from(f.channels),
            meter: Meter::new(usize::from(f.channels), f.rate),
            spectrum: Spectrum::new(f.rate, SPECTRUM_SIZE),
            mono: Vec::new(),
            tone_ring: VecDeque::with_capacity(TONE_SAMPLES),
            tone: None,
            last_publish: past,
            last_tone: past,
        }
    }

    fn feed(&mut self, interleaved: &[f32]) {
        self.meter.feed(interleaved);
        self.mono.clear();
        spectrum::mono(interleaved, self.channels, &mut self.mono);
        self.spectrum.feed(&self.mono);
        self.tone_ring.extend(&self.mono);
        let over = self.tone_ring.len().saturating_sub(TONE_SAMPLES);
        self.tone_ring.drain(..over);
    }

    /// A reading, when one is due.
    fn due(&mut self, format: &Format, note: &str) -> Option<Reading> {
        if self.last_publish.elapsed() < PUBLISH_EVERY {
            return None;
        }
        self.last_publish = Instant::now();
        if self.last_tone.elapsed() >= TONE_EVERY && self.tone_ring.len() == TONE_SAMPLES {
            self.last_tone = Instant::now();
            let (a, b) = self.tone_ring.as_slices();
            let samples: Vec<f32> = a.iter().chain(b).copied().collect();
            self.tone = tone::analyze(&samples, format.rate).filter(|r| r.level_dbfs > -80.0);
        }
        self.spectrum.compute();
        let (peak_hz, peak_dbfs) = self.spectrum.peak();
        Some(Reading {
            active: true,
            format: format.describe(),
            levels: self.meter.levels(),
            bands: self.spectrum.bands(BANDS, BAND_LO_HZ, BAND_HI_HZ),
            rows: self.spectrum.bands(ROWS, BAND_LO_HZ, BAND_HI_HZ),
            peak_hz,
            peak_dbfs,
            tone: self.tone,
            note: note.to_string(),
        })
    }
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

/// What the host's audio thread is asked.
enum Cmd {
    /// Open the default output for `rate` × `channels`; the answer is the rate and channels it
    /// plays at, and a note when they differ.
    Output {
        rate: u32,
        channels: u16,
        ring: Arc<Mutex<Ring>>,
        reply: mpsc::Sender<Result<(u32, usize), String>>,
    },
    CloseOutput,
    /// Open an input (by id; the default without one); its samples go to `ring` at `rate` ×
    /// `channels`.
    Microphone {
        device: Option<String>,
        rate: u32,
        channels: usize,
        ring: Arc<Mutex<Ring>>,
        reply: mpsc::Sender<Result<(), String>>,
    },
    CloseMicrophone,
}

fn host_thread(rx: mpsc::Receiver<Cmd>) {
    let host = cpal::default_host();
    let (mut output, mut microphone): (Option<cpal::Stream>, Option<cpal::Stream>) = (None, None);
    for cmd in rx {
        match cmd {
            Cmd::Output {
                rate,
                channels,
                ring,
                reply,
            } => {
                output = None;
                let Some(device) = host.default_output_device() else {
                    let _ = reply.send(Err("no output on this computer".into()));
                    continue;
                };
                let play = |config: cpal::StreamConfig| {
                    let ring = ring.clone();
                    device.build_output_stream::<f32, _, _>(
                        config,
                        move |data, _| ring.lock().unwrap_or_else(|p| p.into_inner()).pull(data),
                        |e| eprintln!("odap-vdev-view: sound out: {e}"),
                        None,
                    )
                };
                // The guest's own rate and channels first; the device's default otherwise.
                let exact = cpal::StreamConfig {
                    channels,
                    sample_rate: rate,
                    buffer_size: cpal::BufferSize::Default,
                };
                let opened = play(exact).map(|s| (s, exact)).or_else(|_| {
                    let config = device
                        .default_output_config()
                        .map_err(|e| e.to_string())?
                        .config();
                    play(config).map(|s| (s, config)).map_err(|e| e.to_string())
                });
                match opened {
                    Ok((stream, config)) => {
                        let _ = stream.play();
                        output = Some(stream);
                        let _ = reply.send(Ok((config.sample_rate, usize::from(config.channels))));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(e));
                    }
                }
            }
            Cmd::CloseOutput => output = None,
            Cmd::Microphone {
                device,
                rate,
                channels,
                ring,
                reply,
            } => {
                microphone = None;
                let opened = (|| -> Result<cpal::Stream, String> {
                    let device = match &device {
                        Some(id) => host
                            .input_devices()
                            .map_err(|e| e.to_string())?
                            .find(|d| d.id().is_ok_and(|i| i.to_string() == *id))
                            .ok_or("that input is gone")?,
                        None => host
                            .default_input_device()
                            .ok_or("no input on this computer")?,
                    };
                    let config = device
                        .default_input_config()
                        .map_err(|e| e.to_string())?
                        .config();
                    let mut resampler = Resampler::new(
                        config.sample_rate,
                        usize::from(config.channels),
                        rate,
                        channels,
                    );
                    let mut converted = Vec::new();
                    let stream = device
                        .build_input_stream::<f32, _, _>(
                            config,
                            move |data, _| {
                                converted.clear();
                                resampler.process(data, &mut converted);
                                ring.lock()
                                    .unwrap_or_else(|p| p.into_inner())
                                    .push(&converted);
                            },
                            |e| eprintln!("odap-vdev-view: sound in: {e}"),
                            None,
                        )
                        .map_err(|e| e.to_string())?;
                    stream.play().map_err(|e| e.to_string())?;
                    Ok(stream)
                })();
                match opened {
                    Ok(s) => {
                        microphone = Some(s);
                        let _ = reply.send(Ok(()));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(e));
                    }
                }
            }
            Cmd::CloseMicrophone => microphone = None,
        }
    }
    drop((output, microphone));
}

type OnReading = Arc<dyn Fn(Direction, Reading) + Send + Sync>;

/// How a recording ended: its format and seconds, or why it failed.
pub type Recorded = Result<(Format, f64), String>;

/// What the listeners hand the analysis thread.
enum Feed {
    /// Samples as decoded (what the guest played, or received), with the direction's note.
    Samples(Direction, Format, String, Vec<f32>),
    /// Measuring stopped: its state goes.
    Reset(Direction),
}

/// Chunks the analysis may fall behind by before samples for it are dropped (a second or so).
const FEED_DEPTH: usize = 128;

/// The analysis thread: each direction's measurements, published as they fall due.
fn analysis_thread(rx: mpsc::Receiver<Feed>, on_reading: OnReading) {
    let mut state: [Option<(Format, Analysis)>; 2] = [None, None];
    let slot = |d: Direction| usize::from(d == Direction::Input);
    for feed in rx {
        match feed {
            Feed::Reset(d) => state[slot(d)] = None,
            Feed::Samples(d, f, note, samples) => {
                let s = &mut state[slot(d)];
                if s.as_ref().is_none_or(|(g, _)| *g != f) {
                    *s = Some((f, Analysis::new(&f)));
                }
                let (f, a) = s.as_mut().expect("just set");
                a.feed(&samples);
                if let Some(r) = a.due(f, &note) {
                    on_reading(d, r);
                }
            }
        }
    }
}

/// A recording of one direction's exact bytes (the control socket's `record`), from its first
/// sound for a number of bytes; it ends on its own, or with the stream.
struct Tap {
    file: std::io::BufWriter<std::fs::File>,
    format: Option<Format>,
    seconds: f64,
    written: usize,
    wanted: usize,
    done: Option<mpsc::Sender<Recorded>>,
}

impl Tap {
    /// Takes the stream's bytes; true once it has what it wanted (or failed).
    fn take(&mut self, f: &Format, data: &[u8]) -> bool {
        if self.format.is_some_and(|g| g != *f) {
            return self.finish(Err(format!(
                "the guest changed its format to {}",
                f.describe()
            )));
        }
        if self.format.is_none() {
            // From the first sound: leading digital silence is not recorded.
            if data.iter().all(|&b| b == 0) {
                return false;
            }
            self.format = Some(*f);
            self.wanted = (self.seconds * f64::from(f.rate)) as usize * f.bytes_per_frame();
            if let Err(e) = opendap_audio_analysis::wav::write_header(&mut self.file, f, 0) {
                return self.finish(Err(e.to_string()));
            }
        }
        let take = data.len().min(self.wanted - self.written);
        if let Err(e) = std::io::Write::write_all(&mut self.file, &data[..take]) {
            return self.finish(Err(e.to_string()));
        }
        self.written += take;
        self.written >= self.wanted && self.finish(Ok(()))
    }

    /// Closes the file with its final sizes and says how it went.
    fn finish(&mut self, r: Result<(), String>) -> bool {
        let r = r.and_then(|()| {
            let f = self.format.ok_or("nothing played")?;
            std::io::Write::flush(&mut self.file).map_err(|e| e.to_string())?;
            opendap_audio_analysis::wav::finalize(self.file.get_mut(), &f, self.written as u32)
                .map_err(|e| e.to_string())?;
            Ok((
                f,
                self.written as f64 / f.bytes_per_frame() as f64 / f64::from(f.rate),
            ))
        });
        if let Some(done) = self.done.take() {
            let _ = done.send(r);
        }
        true
    }
}

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

struct OutListener(Arc<Shared>);

/// QEMU waits for the window's answer to every input read (ADR 0003), so the thread that answers
/// runs at the highest QoS: at a lower one, a loaded host left it seconds without the CPU, QEMU
/// stalled and the guest's playback and capture failed. Once per thread.
fn urgent_thread() {
    thread_local!(static DONE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) });
    DONE.with(|done| {
        if !done.replace(true) {
            #[cfg(target_os = "macos")]
            // SAFETY: sets the calling thread's own QoS class; no pointers are involved.
            unsafe {
                libc::pthread_set_qos_class_self_np(
                    libc::qos_class_t::QOS_CLASS_USER_INTERACTIVE,
                    0,
                );
            }
        }
    });
}

impl Sink for OutListener {
    fn init(&self, stream: u64, f: audio::Format) {
        trace(format_args!("out {stream:x} init {f:?}"));
        let mut out = lock(&self.0.out);
        let f = format_of(f);
        out.format = Some(f);
    }

    fn fini(&self, stream: u64) {
        trace(format_args!("out {stream:x} fini"));
        let mut out = lock(&self.0.out);
        out.format = None;
        out.host = None;
        self.0.send(Cmd::CloseOutput);
        (self.0.on_reading)(Direction::Output, idle(""));
    }

    fn enabled(&self, stream: u64, on: bool) {
        trace(format_args!("out {stream:x} enabled {on}"));
        if on {
            self.0.open_output();
        } else {
            let mut out = lock(&self.0.out);
            out.host = None;
            self.0.send(Cmd::CloseOutput);
            let note = out.note.clone();
            drop(out);
            (self.0.on_reading)(Direction::Output, idle(&note));
        }
    }

    fn write(&self, _stream: u64, data: &[u8]) {
        urgent_thread();
        let mut guard = lock(&self.0.out);
        let out = &mut *guard;
        let Some(f) = out.format else { return };
        if out.tap.as_mut().is_some_and(|t| t.take(&f, data)) {
            out.tap = None;
        }
        out.decoded.clear();
        decode(&f, data, &mut out.decoded);
        if let Some((ring, resampler)) = out.host.as_mut() {
            out.converted.clear();
            resampler.process(&out.decoded, &mut out.converted);
            lock(ring).push(&out.converted);
        }
        if out.analyze {
            // Never waits: a full channel drops these samples for the analysis only.
            let _ = self.0.feed.try_send(Feed::Samples(
                Direction::Output,
                f,
                out.note.clone(),
                out.decoded.clone(),
            ));
        }
    }
}

struct InListener(Arc<Shared>);

/// f32 samples as bytes in `f` (integers rounded and clamped to the format's range).
fn encode(f: &Format, samples: &[f32], out: &mut Vec<u8>) {
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

impl Source for InListener {
    fn init(&self, stream: u64, f: audio::Format) {
        trace(format_args!("in {stream:x} init {f:?}"));
        let mut inp = lock(&self.0.inp);
        let f = format_of(f);
        inp.format = Some(f);
    }

    fn fini(&self, stream: u64) {
        trace(format_args!("in {stream:x} fini"));
        let mut inp = lock(&self.0.inp);
        inp.format = None;
        inp.enabled = false;
        inp.microphone = None;
        self.0.send(Cmd::CloseMicrophone);
        (self.0.on_reading)(Direction::Input, idle(""));
    }

    fn enabled(&self, stream: u64, on: bool) {
        trace(format_args!("in {stream:x} enabled {on}"));
        {
            let mut inp = lock(&self.0.inp);
            inp.enabled = on;
            inp.started = on.then(Instant::now);
            inp.given = 0;
        }
        self.0.open_microphone();
        if !on {
            let note = lock(&self.0.inp).note.clone();
            (self.0.on_reading)(Direction::Input, idle(&note));
        }
    }

    fn read(&self, _stream: u64, size: usize) -> Vec<u8> {
        urgent_thread();
        let mut guard = lock(&self.0.inp);
        let inp = &mut *guard;
        let Some(f) = inp.format else {
            return vec![0; size];
        };
        let ch = usize::from(f.channels);
        let asked = size / f.bytes_per_frame().max(1);
        // What is due by the clock since recording started; an input gives what it has.
        let elapsed = inp.started.map_or(0, |t| {
            (t.elapsed().as_secs_f64() * f64::from(f.rate)) as u64
        });
        let due = elapsed.saturating_sub(inp.given);
        let mut frames = asked.min(due as usize);
        let mut samples;
        match inp.source {
            InSource::Device(_) => {
                let r = inp.microphone.as_ref().map(|ring| lock(ring));
                frames = frames.min(r.as_ref().map_or(frames, |r| r.q.len() / ch));
                samples = vec![0.0f32; frames * ch];
                if let Some(mut r) = r {
                    for s in samples.iter_mut() {
                        *s = r.q.pop_front().unwrap_or(0.0);
                    }
                }
            }
            InSource::Tone => {
                samples = vec![0.0f32; frames * ch];
                let amp = 10f64.powf(TEST_TONE_DBFS / 20.0);
                let w = 2.0 * std::f64::consts::PI * TEST_TONE_HZ / f64::from(f.rate);
                for frame in samples.chunks_exact_mut(ch) {
                    frame.fill((amp * inp.phase.sin()) as f32);
                    inp.phase = (inp.phase + w) % (2.0 * std::f64::consts::PI);
                }
            }
            InSource::Silence => samples = vec![0.0f32; frames * ch],
        }
        inp.given += frames as u64;
        let t = &mut inp.tally;
        t.calls += 1;
        t.asked += size as u64;
        t.given += (frames * f.bytes_per_frame()) as u64;
        if t.since
            .is_none_or(|s| s.elapsed() >= Duration::from_secs(1))
        {
            trace(format_args!(
                "in reads {} asked {} given {} (since start {} frames)",
                t.calls, t.asked, t.given, inp.given
            ));
            *t = ReadTally {
                since: Some(Instant::now()),
                ..Default::default()
            };
        }
        // Through QEMU's mixing engine (stock QEMU 11.1, whose D-Bus input spins without it) a
        // mono sample is duplicated into both channels and summed back: a mono capture arrives
        // at twice its value, so a mono source gives half and the guest records the level it was
        // given, to within one LSB. The patched QEMU runs the input without the engine and
        // carries it bit for bit (ADR 0003).
        let mono_gain = if ch == 1 && self.0.mono_doubled {
            0.5
        } else {
            1.0
        };
        if ch == 1 {
            samples.iter_mut().for_each(|s| *s *= mono_gain);
        }
        let mut bytes = Vec::with_capacity(size);
        encode(&f, &samples, &mut bytes);
        if inp.tap.as_mut().is_some_and(|t| t.take(&f, &bytes)) {
            inp.tap = None;
        }
        if inp.analyze {
            // What the guest receives, quantized to its format, is what is measured; handed to
            // the analysis thread without waiting (QEMU waits for this answer).
            samples.clear();
            decode(&f, &bytes, &mut samples);
            samples.iter_mut().for_each(|s| *s /= mono_gain);
            let _ = self.0.feed.try_send(Feed::Samples(
                Direction::Input,
                f,
                inp.note.clone(),
                samples,
            ));
        }
        bytes
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_resampler_keeps_identity_exact_and_converts_otherwise() {
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
    fn the_ring_primes_and_stays_bounded() {
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
    fn encoding_rounds_and_clamps() {
        let f = Format::int(48_000, 1, 16);
        let mut b = Vec::new();
        encode(&f, &[0.5, -1.0, 1.0], &mut b);
        assert_eq!(b, [0x00, 0x40, 0x00, 0x80, 0xff, 0x7f]);
    }
}
