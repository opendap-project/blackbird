//! Recordings of one direction's exact bytes, for the control socket's `record`.

use super::*;

/// A recording of one direction's exact bytes (the control socket's `record`), from its first
/// sound for a number of bytes; it ends on its own, or with the stream.
pub(super) struct Tap {
    pub(super) file: std::io::BufWriter<std::fs::File>,
    pub(super) format: Option<Format>,
    pub(super) seconds: f64,
    pub(super) written: usize,
    pub(super) wanted: usize,
    pub(super) done: Option<mpsc::Sender<Recorded>>,
}

impl Tap {
    /// Takes the stream's bytes; true once it has what it wanted (or failed).
    pub(super) fn take(&mut self, f: &Format, data: &[u8]) -> bool {
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
            if let Err(e) =
                blackbird_services_audio_analysis::wav::write_header(&mut self.file, f, 0)
            {
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
    pub(super) fn finish(&mut self, r: Result<(), String>) -> bool {
        let r = r.and_then(|()| {
            let f = self.format.ok_or("nothing played")?;
            std::io::Write::flush(&mut self.file).map_err(|e| e.to_string())?;
            blackbird_services_audio_analysis::wav::finalize(
                self.file.get_mut(),
                &f,
                self.written as u32,
            )
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
