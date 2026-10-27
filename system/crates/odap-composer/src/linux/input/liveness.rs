//! Whether the activity responds (SYS-072): a ping after a key, and how long it may go unanswered.

use super::*;

/// How long an activity may leave a ping unanswered before it counts as not responding.
pub(super) const UNRESPONSIVE: std::time::Duration = std::time::Duration::from_millis(500);

/// Whether the activity responds: a ping (xdg_wm_base) sent after a key it was given and left
/// unanswered past `UNRESPONSIVE` means it is busy with that key (a hang), and the next key is
/// dropped rather than queued, so nothing acts seconds later by surprise
/// (SYS-072). The ping goes after the key, so the activity
/// meets the key first. Nothing is sent at rest: no keys, no pings.
pub(super) fn activity_responds(composer: &Composer) -> bool {
    composer
        .ping_since
        .is_none_or(|since| since.elapsed() < UNRESPONSIVE)
}

/// Pings the activity a moment after a key (unless a ping is out): a client answers pings while it
/// reads its events, before its handlers run, so a ping sent with the key would be answered before
/// the handler that hangs; 100 ms later, a responsive activity answers within milliseconds and a
/// hung one does not. The timer exists only after input: nothing runs at rest.
pub(super) fn ping_activity(composer: &mut Composer) {
    if composer.ping_since.is_some() || composer.ping_scheduled {
        return;
    }
    composer.ping_scheduled = true;
    let _ = composer.handle.insert_source(
        calloop::timer::Timer::from_duration(std::time::Duration::from_millis(100)),
        |_, _, composer| {
            composer.ping_scheduled = false;
            if composer.ping_since.is_none() {
                let client = composer
                    .space
                    .elements()
                    .last()
                    .and_then(|w| w.toplevel().map(|t| t.client()));
                if let Some(client) = client
                    && client.send_ping(SERIAL_COUNTER.next_serial()).is_ok()
                {
                    composer.ping_since = Some(std::time::Instant::now());
                    composer.pinged = Some(client);
                }
            }
            calloop::timer::TimeoutAction::Drop
        },
    );
}
