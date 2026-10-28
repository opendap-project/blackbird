//! The measurements on the exact streams, on a thread of their own.

use super::*;

pub(super) const PUBLISH_EVERY: Duration = Duration::from_millis(100);
pub(super) const TONE_EVERY: Duration = Duration::from_secs(1);
pub(super) const SPECTRUM_SIZE: usize = 8192;
pub(super) const TONE_SAMPLES: usize = 32_768;
/// A direction's measurements, sized for its stream.
pub(super) struct Analysis {
    pub(super) channels: usize,
    pub(super) meter: Meter,
    pub(super) spectrum: Spectrum,
    pub(super) mono: Vec<f32>,
    pub(super) tone_ring: VecDeque<f32>,
    pub(super) tone: Option<tone::Report>,
    pub(super) last_publish: Instant,
    pub(super) last_tone: Instant,
}

impl Analysis {
    pub(super) fn new(f: &Format) -> Self {
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

    pub(super) fn feed(&mut self, interleaved: &[f32]) {
        self.meter.feed(interleaved);
        self.mono.clear();
        spectrum::mono(interleaved, self.channels, &mut self.mono);
        self.spectrum.feed(&self.mono);
        self.tone_ring.extend(&self.mono);
        let over = self.tone_ring.len().saturating_sub(TONE_SAMPLES);
        self.tone_ring.drain(..over);
    }

    /// A reading, when one is due.
    pub(super) fn due(&mut self, format: &Format, note: &str) -> Option<Reading> {
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

/// What the listeners hand the analysis thread.
pub(super) enum Feed {
    /// Samples as decoded (what the guest played, or received), with the direction's note.
    Samples(Direction, Format, String, Vec<f32>),
    /// Measuring stopped: its state goes.
    Reset(Direction),
}

/// Chunks the analysis may fall behind by before samples for it are dropped (a second or so).
pub(super) const FEED_DEPTH: usize = 128;

/// The analysis thread: each direction's measurements, published as they fall due.
pub(super) fn analysis_thread(rx: mpsc::Receiver<Feed>, on_reading: OnReading) {
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
