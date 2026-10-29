//! An OS Shell draft hosted by the composer (RFC 0012). The status bar's state (SYS-038) and the
//! battery's warnings, read from the kernel's own classes (what each item shows and when the
//! battery warns: status_bar.rs):
//! the clock, the microSD card (a block device of type SD, never the eMMC), Bluetooth and Wi-Fi
//! (rfkill and the network devices), the battery and the charger (power_supply). Kernel uevents
//! wake it, and the clock's minute; nothing polls.

use std::time::Duration;

use smithay::reexports::calloop::{
    Interest, Mode, PostAction,
    generic::Generic,
    timer::{TimeoutAction, Timer},
};
use tracing::{info, warn};

use super::state::Composer;

use crate::status_bar::{until_next_minute, warning};

/// How long a battery warning shows.
const WARNING_FOR: Duration = Duration::from_secs(4);

/// The state the bar shows, as read.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Status {
    pub time: String,
    pub microsd: bool,
    pub battery: Option<u8>,
    pub charging: bool,
    /// None: no such radio; Some(on).
    pub bluetooth: Option<bool>,
    pub wifi: Option<bool>,
}

fn clock() -> String {
    // SAFETY: time(2) and localtime_r(3) with valid pointers to stack values.
    unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&now, &mut tm).is_null() {
            return String::new();
        }
        format!("{:02}:{:02}", tm.tm_hour, tm.tm_min)
    }
}

pub fn now() -> Status {
    let mut s = Status {
        time: clock(),
        ..Default::default()
    };
    let battery = blackbird_platform::power_supply::read_supplies().battery;
    s.battery = battery.map(|b| b.level);
    s.charging = battery.is_some_and(|b| b.charging);
    let sys = blackbird_platform::sysfs::Sysfs::default();
    s.microsd = sys.microsd();
    s.bluetooth = sys.radio("bluetooth");
    s.wifi = sys
        .radio("wlan")
        .or_else(|| sys.wireless_interface().then_some(true));
    s
}

/// Reads the state again and shows it; warns when the battery crosses a threshold.
pub fn refresh(composer: &mut Composer) {
    let status = now();
    let before = composer.system.status.battery;
    let note = warning(before, status.battery, status.charging);
    if status != composer.system.status {
        composer.system.status = status.clone();
        if let Some(layers) = composer.system.layers.as_mut() {
            layers.set_status(&status);
        }
    }
    if let Some(text) = note {
        info!(%text, "battery");
        if let Some(layers) = composer.system.layers.as_mut() {
            layers.show_toast(&text);
        }
        let _ =
            composer
                .handle
                .insert_source(Timer::from_duration(WARNING_FOR), |_, _, composer| {
                    if let Some(l) = composer.system.layers.as_mut() {
                        l.hide_toast();
                    }
                    redraw(composer);
                    TimeoutAction::Drop
                });
    }
    redraw(composer);
}

fn redraw(composer: &mut Composer) {
    if let Some(b) = composer.backend.as_mut() {
        b.schedule();
    }
    super::backend::render_if_idle(composer);
}

/// Follows the kernel's uevents and the clock's minutes, and the debug UI mode's flag, so the
/// mode switches in place whenever it is set (an x-ray of whatever is on screen: nothing
/// restarts).
pub fn start(composer: &mut Composer) {
    refresh(composer);
    watch_debug(composer);
    match blackbird_platform::kernel::uevents() {
        Ok(fd) => {
            let source = Generic::new(fd, Interest::READ, Mode::Level);
            let inserted = composer.handle.insert_source(source, |_, fd, composer| {
                blackbird_platform::kernel::drain(&**fd);
                refresh(composer);
                Ok(PostAction::Continue)
            });
            if let Err(e) = inserted {
                warn!(%e, "status: uevents");
            }
        }
        Err(e) => warn!(%e, "status: uevents"),
    }
    // The clock: woken at the start of each minute.
    let _ =
        composer
            .handle
            .insert_source(Timer::from_duration(to_next_minute()), |_, _, composer| {
                refresh(composer);
                TimeoutAction::ToDuration(to_next_minute())
            });
}

fn watch_debug(composer: &mut Composer) {
    // The debug UI switch and the show-touches switch (blackbird-platform run).
    let Ok(fd) = blackbird_platform::kernel::inotify(&[blackbird_platform::run::COMPOSER_DIR])
    else {
        return;
    };
    blackbird_platform::kernel::watch(&fd, blackbird_platform::run::DEBUG_DIR);
    let inserted = composer.handle.insert_source(
        Generic::new(fd, Interest::READ, Mode::Level),
        |_, fd, composer| {
            blackbird_platform::kernel::drain(&**fd);
            if composer
                .system
                .layers
                .as_mut()
                .is_some_and(|l| l.refresh_debug())
            {
                redraw(composer);
            }
            Ok(PostAction::Continue)
        },
    );
    if let Err(e) = inserted {
        warn!(%e, "status: debug flag");
    }
}

fn to_next_minute() -> Duration {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    until_next_minute(now)
}
