//! When the composer restarts the product host after it exits, apart from the thread that runs it
//! (linux/mod.rs), so the backoff is tested on any host. Until the orchestrator owns activities
//! (ADR 0002), the composer supervises the product host.
//!
//! The first restart is quick; repeated failures wait longer, doubling up to `MAX`. A host that
//! stayed up `HEALTHY` counts as recovered, and its next restart is quick again.

use std::time::Duration;

pub const MIN: Duration = Duration::from_millis(250);
pub const MAX: Duration = Duration::from_secs(8);
pub const HEALTHY: Duration = Duration::from_secs(30);

/// The wait before the next restart.
#[derive(Debug)]
pub struct Backoff {
    next: Duration,
}

impl Default for Backoff {
    fn default() -> Self {
        Self { next: MIN }
    }
}

impl Backoff {
    /// The host exited after running `ran`: how long to wait before starting it again.
    pub fn after_exit(&mut self, ran: Duration) -> Duration {
        if ran >= HEALTHY {
            self.next = MIN;
        }
        let wait = self.next;
        self.next = (self.next * 2).min(MAX);
        wait
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_failures_wait_longer_up_to_the_most() {
        let mut b = Backoff::default();
        let waits: Vec<_> = (0..8).map(|_| b.after_exit(Duration::ZERO)).collect();
        let ms: Vec<_> = waits.iter().map(|w| w.as_millis()).collect();
        assert_eq!(ms, [250, 500, 1000, 2000, 4000, 8000, 8000, 8000]);
    }

    #[test]
    fn a_host_that_stayed_up_restarts_quickly_again() {
        let mut b = Backoff::default();
        for _ in 0..5 {
            b.after_exit(Duration::from_secs(1));
        }
        assert_eq!(b.after_exit(HEALTHY), MIN);
        assert_eq!(b.after_exit(Duration::from_secs(1)), MIN * 2);
        assert_eq!(b.after_exit(HEALTHY - Duration::from_millis(1)), MIN * 4);
    }
}
