//! QEMU's audio listeners: the guest's output arrives, its input is answered.

use super::*;

pub(super) struct OutListener(pub(super) Arc<Shared>);

/// QEMU waits for the window's answer to every input read (ADR 0003), so the thread that answers
/// runs at the highest QoS: at a lower one, a loaded host left it seconds without the CPU, QEMU
/// stalled and the guest's playback and capture failed. Once per thread.
pub(super) fn urgent_thread() {
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

pub(super) struct InListener(pub(super) Arc<Shared>);

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
        let (frames, mut samples) = from_source(inp, &f, asked.min(due as usize));
        inp.given += frames as u64;
        count(inp, size, frames * f.bytes_per_frame());
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

/// Up to `frames` frames from the input's source (an input gives what it has), as samples.
fn from_source(inp: &mut In, f: &Format, frames: usize) -> (usize, Vec<f32>) {
    let ch = usize::from(f.channels);
    match inp.source {
        InSource::Device(_) => {
            let r = inp.microphone.as_ref().map(|ring| lock(ring));
            let frames = frames.min(r.as_ref().map_or(frames, |r| r.q.len() / ch));
            let mut samples = vec![0.0f32; frames * ch];
            if let Some(mut r) = r {
                for s in samples.iter_mut() {
                    *s = r.q.pop_front().unwrap_or(0.0);
                }
            }
            (frames, samples)
        }
        InSource::Tone => {
            let mut samples = vec![0.0f32; frames * ch];
            let amp = 10f64.powf(TEST_TONE_DBFS / 20.0);
            let w = 2.0 * std::f64::consts::PI * TEST_TONE_HZ / f64::from(f.rate);
            for frame in samples.chunks_exact_mut(ch) {
                frame.fill((amp * inp.phase.sin()) as f32);
                inp.phase = (inp.phase + w) % (2.0 * std::f64::consts::PI);
            }
            (frames, samples)
        }
        InSource::Silence => (frames, vec![0.0f32; frames * ch]),
    }
}

/// The reads' tally, traced once a second (ODAP_VIEW_TRACE).
fn count(inp: &mut In, asked: usize, given: usize) {
    let t = &mut inp.tally;
    t.calls += 1;
    t.asked += asked as u64;
    t.given += given as u64;
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
}
