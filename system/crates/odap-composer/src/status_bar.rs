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
