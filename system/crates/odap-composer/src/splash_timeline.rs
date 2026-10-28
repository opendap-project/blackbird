//! When the splash screen shows and leaves (composer and UI runtime spec §9), apart from how it is
//! drawn (linux/splash.rs), so its timeline is tested on any host: on screen at least
//! `MINIMUM_MS` from when the boot UI first showed it, and until the product host has drawn; then
//! a fade of `FADE_MS`; then gone for good.

use std::time::{Duration, Instant};

/// How long the splash screen must still show at `now_ms` (the monotonic clock), from the boot
/// UI's record of when it first showed it: counted from now without one, or with one from the
/// future.
pub fn minimum_left(shown_at: Option<&str>, now_ms: u64, minimum_ms: u64) -> Duration {
    let shown = shown_at
        .and_then(|t| t.trim().parse::<u64>().ok())
        .filter(|&t| t <= now_ms)
        .unwrap_or(now_ms);
    Duration::from_millis((shown + minimum_ms).saturating_sub(now_ms))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    /// On screen until `until`, and longer if the product host has not drawn yet.
    Showing {
        until: Instant,
        client_ready: bool,
    },
    Fading(Instant),
    Done,
}

/// The splash screen's timeline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timeline {
    phase: Phase,
    fade: Duration,
}

impl Timeline {
    pub fn new(until: Instant, fade: Duration) -> Self {
        Self {
            phase: Phase::Showing {
                until,
                client_ready: false,
            },
            fade,
        }
    }

    /// The product host drew its first frame.
    pub fn client_ready(&mut self) {
        if let Phase::Showing { client_ready, .. } = &mut self.phase {
            *client_ready = true;
        }
    }

    /// Whether the splash screen has left the display for good.
    pub fn done(&self) -> bool {
        matches!(self.phase, Phase::Done)
    }

    /// Whether frames must keep coming: only while fading. A static splash screen draws nothing.
    pub fn animating(&self) -> bool {
        matches!(self.phase, Phase::Fading(_))
    }

    /// When the composer must look again: the end of the minimum, once the product host is ready.
    pub fn wake_at(&self, now: Instant) -> Option<Instant> {
        match self.phase {
            Phase::Showing {
                until,
                client_ready: true,
            } if now < until => Some(until),
            _ => None,
        }
    }

    /// Its opacity in the frame drawn at `now`, or `None` once it has left; moves the timeline on.
    pub fn alpha(&mut self, now: Instant) -> Option<f32> {
        if let Phase::Showing {
            until,
            client_ready: true,
        } = self.phase
            && now >= until
        {
            self.phase = Phase::Fading(now);
        }
        match self.phase {
            Phase::Showing { .. } => Some(1.0),
            Phase::Fading(start) => {
                let t =
                    now.saturating_duration_since(start).as_secs_f32() / self.fade.as_secs_f32();
                if t >= 1.0 {
                    self.phase = Phase::Done;
                    return None;
                }
                Some(1.0 - t)
            }
            Phase::Done => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u64 = 5000;

    #[test]
    fn the_minimum_counts_from_when_the_boot_ui_showed_it() {
        let ms = Duration::from_millis;
        assert_eq!(minimum_left(Some("1000\n"), 3000, MIN), ms(3000));
        assert_eq!(minimum_left(Some("1000"), 7000, MIN), ms(0));
        assert_eq!(
            minimum_left(None, 3000, MIN),
            ms(5000),
            "from now without one"
        );
        assert_eq!(
            minimum_left(Some("9000"), 3000, MIN),
            ms(5000),
            "not from the future"
        );
        assert_eq!(minimum_left(Some("soon"), 3000, MIN), ms(5000));
    }

    #[test]
    fn it_stays_until_the_minimum_and_the_product_host_then_fades() {
        let t0 = Instant::now();
        let ms = Duration::from_millis;
        let mut s = Timeline::new(t0 + ms(1000), ms(320));
        // Past the minimum but the product host has not drawn: it stays, and asks no wake.
        assert_eq!(s.alpha(t0 + ms(2000)), Some(1.0));
        assert_eq!(s.wake_at(t0), None);
        s.client_ready();
        assert_eq!(s.wake_at(t0 + ms(500)), Some(t0 + ms(1000)));
        // The fade starts at the first frame after both, and runs FADE.
        assert_eq!(s.alpha(t0 + ms(2000)), Some(1.0));
        assert!(s.animating());
        assert_eq!(s.alpha(t0 + ms(2160)), Some(0.5));
        assert_eq!(s.alpha(t0 + ms(2320)), None);
        assert!(s.done() && !s.animating());
        assert_eq!(s.alpha(t0 + ms(9000)), None);
    }

    #[test]
    fn a_ready_host_before_the_minimum_waits_for_it() {
        let t0 = Instant::now();
        let ms = Duration::from_millis;
        let mut s = Timeline::new(t0 + ms(1000), ms(320));
        s.client_ready();
        assert_eq!(s.alpha(t0 + ms(999)), Some(1.0));
        assert!(!s.animating());
        assert_eq!(s.alpha(t0 + ms(1000)), Some(1.0));
        assert!(s.animating());
    }
}
