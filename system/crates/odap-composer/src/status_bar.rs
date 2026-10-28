//! What the status bar shows and says for each item (SYS-038), apart from how it reads the state
//! (linux/status.rs) and how it draws it (ui/layers.slint), so the choices are tested on any host.
//!
//! Items, in visual and reading order: at the start the time and the microSD card (the volume item
//! joins the end of this group once the audio engine provides the level and its safe or warning
//! state, SYS-012); at the end Bluetooth, Wi-Fi and the battery.

/// The low battery, in percent: the status LED's threshold (odap-statusled), Android's default
/// (familiar, early enough to act; SYS-076). At it or less, discharging, the battery's
/// icon turns amber (and linux/status.rs warns once).
pub const LOW_PERCENT: u8 = 15;
/// The critically low battery, in percent: the status LED's second threshold, Android's default
/// (SYS-076).
pub const CRITICAL_PERCENT: u8 = 5;
/// The warning to show when the charge goes from `before` to `after`, discharging: once when it
/// crosses each threshold, never again until it rises above it.
pub fn warning(before: Option<u8>, after: Option<u8>, charging: bool) -> Option<String> {
    let (Some(before), Some(after)) = (before, after) else {
        return None;
    };
    if charging {
        return None;
    }
    if before > CRITICAL_PERCENT && after <= CRITICAL_PERCENT {
        Some(format!("Battery critically low: {after} %"))
    } else if before > LOW_PERCENT && after <= LOW_PERCENT {
        Some(format!("Battery low: {after} %"))
    } else {
        None
    }
}

/// From `since_epoch` to just after the next minute starts (20 ms in, so the clock reads the new
/// minute): the clock's one wake a minute, never a poll.
pub fn until_next_minute(since_epoch: std::time::Duration) -> std::time::Duration {
    let into = since_epoch.as_millis() % 60_000;
    std::time::Duration::from_millis((60_000 - into) as u64 + 20)
}

/// A radio as the bar shows it: one the hardware lacks is not shown (SYS-040), one that is off is
/// dimmed (the design system's Disabled opacity), one that is on is drawn as every other item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Radio {
    Absent,
    Off,
    On,
}

impl Radio {
    /// From the platform's reading: `None` when there is no such radio, else whether it is on.
    pub fn from_reading(reading: Option<bool>) -> Self {
        match reading {
            None => Self::Absent,
            Some(false) => Self::Off,
            Some(true) => Self::On,
        }
    }

    pub fn shown(self) -> bool {
        self != Self::Absent
    }

    pub fn dimmed(self) -> bool {
        self == Self::Off
    }
}

/// The battery glyph's level: 0 (empty) to 4 (full), the nearest of the status set's five
/// (`status-battery-0` to `-4`: 0, 25, 50, 75 and 100 %). Charging has a glyph of its own.
pub fn battery_glyph(percent: u8) -> u8 {
    // Rounded to the nearest quarter: 13 % is a bar, 12 % none.
    ((u16::from(percent.min(100)) + 12) / 25) as u8
}

/// Whether the battery shows as low (amber): at `LOW_PERCENT` or less, and not charging.
pub fn battery_low(percent: u8, charging: bool) -> bool {
    !charging && percent <= LOW_PERCENT
}

/// One accessible label per item, stating its state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Labels {
    pub time: String,
    pub microsd: String,
    pub bluetooth: String,
    pub wifi: String,
    pub battery: String,
}

/// The labels for the state the bar shows. An item that is not shown gets an empty label.
pub fn labels(
    time: &str,
    microsd: bool,
    bluetooth: Radio,
    wifi: Radio,
    battery: Option<u8>,
    charging: bool,
) -> Labels {
    let radio = |name: &str, r: Radio| match r {
        Radio::Absent => String::new(),
        Radio::Off => format!("{name} off"),
        Radio::On => format!("{name} on"),
    };
    let battery = battery.map_or_else(String::new, |p| {
        let state = if charging {
            ", charging"
        } else if battery_low(p, charging) {
            ", low"
        } else {
            ""
        };
        format!("Battery {p} %{state}")
    });
    Labels {
        time: format!("Time {time}"),
        microsd: if microsd {
            "microSD card".into()
        } else {
            String::new()
        },
        bluetooth: radio("Bluetooth", bluetooth),
        wifi: radio("Wi-Fi", wifi),
        battery,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warns_once_per_threshold_while_discharging() {
        assert_eq!(
            warning(Some(16), Some(15), false).as_deref(),
            Some("Battery low: 15 %")
        );
        assert_eq!(warning(Some(15), Some(14), false), None, "once");
        assert_eq!(
            warning(Some(6), Some(5), false).as_deref(),
            Some("Battery critically low: 5 %")
        );
        assert_eq!(
            warning(Some(16), Some(15), true),
            None,
            "not while charging"
        );
        assert_eq!(
            warning(None, Some(10), false),
            None,
            "not at the first reading"
        );
    }

    #[test]
    fn the_clock_wakes_just_after_each_minute() {
        use std::time::Duration;
        let ms = |m| Duration::from_millis(m);
        assert_eq!(until_next_minute(ms(120_000)), ms(60_020));
        assert_eq!(until_next_minute(ms(120_001)), ms(60_019));
        assert_eq!(until_next_minute(ms(179_990)), ms(30));
    }

    #[test]
    fn a_radio_the_hardware_lacks_is_hidden_and_an_off_one_dimmed() {
        let absent = Radio::from_reading(None);
        let off = Radio::from_reading(Some(false));
        let on = Radio::from_reading(Some(true));
        assert_eq!((absent, off, on), (Radio::Absent, Radio::Off, Radio::On));
        assert!(!absent.shown());
        assert!(off.shown() && off.dimmed());
        assert!(on.shown() && !on.dimmed());
    }

    #[test]
    fn battery_glyph_is_the_nearest_of_five_levels() {
        let cases = [
            (0, 0),
            (12, 0),
            (13, 1),
            (37, 1),
            (38, 2),
            (50, 2),
            (62, 2),
            (63, 3),
            (87, 3),
            (88, 4),
            (100, 4),
        ];
        for (percent, glyph) in cases {
            assert_eq!(battery_glyph(percent), glyph, "{percent} %");
        }
        assert_eq!(battery_glyph(255), 4, "a reading past 100 stays full");
    }

    #[test]
    fn battery_is_low_at_the_warning_threshold_while_discharging() {
        assert!(battery_low(15, false));
        assert!(battery_low(10, false));
        assert!(!battery_low(16, false));
        assert!(!battery_low(10, true), "not while charging");
    }

    #[test]
    fn each_item_says_its_state() {
        let l = labels("10:42", true, Radio::Off, Radio::On, Some(50), true);
        assert_eq!(l.time, "Time 10:42");
        assert_eq!(l.microsd, "microSD card");
        assert_eq!(l.bluetooth, "Bluetooth off");
        assert_eq!(l.wifi, "Wi-Fi on");
        assert_eq!(l.battery, "Battery 50 %, charging");

        let l = labels("07:05", false, Radio::On, Radio::Off, Some(10), false);
        assert_eq!(l.microsd, "");
        assert_eq!(l.bluetooth, "Bluetooth on");
        assert_eq!(l.wifi, "Wi-Fi off");
        assert_eq!(l.battery, "Battery 10 %, low");

        let l = labels("07:05", false, Radio::Absent, Radio::Absent, None, false);
        assert_eq!(
            (l.bluetooth.as_str(), l.wifi.as_str(), l.battery.as_str()),
            ("", "", "")
        );
        assert_eq!(
            labels("07:05", false, Radio::On, Radio::On, Some(100), false).battery,
            "Battery 100 %"
        );
    }
}
