//! The kernel's power-supply class (`/sys/class/power_supply`), the generic interface every Linux fuel gauge
//! and charger driver implements: the battery's level and charging state, and whether external power is in.
//! A device without a battery, or a host that is not Linux, reports none (hardware support spec §6).

use std::{fs, path::Path};

/// Where the kernel lists the power supplies.
pub const SYSFS: &str = "/sys/class/power_supply";

/// A battery's level in percent (at most 100) and whether it is charging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Battery {
    pub level: u8,
    pub charging: bool,
}

/// What the power supplies report.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Supplies {
    /// The first battery with a readable level.
    pub battery: Option<Battery>,
    /// Mains, USB or USB PD power is connected.
    pub external: bool,
}

fn read(p: &Path) -> Option<String> {
    fs::read_to_string(p).ok().map(|s| s.trim().to_string())
}

/// The supplies listed under `root` (a `power_supply` class directory).
pub fn read_from(root: &Path) -> Supplies {
    let mut out = Supplies::default();
    let mut entries: Vec<_> = fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .collect();
    entries.sort();
    for p in entries {
        match read(&p.join("type")).as_deref() {
            Some("Battery") if out.battery.is_none() => {
                if let Some(level) = read(&p.join("capacity")).and_then(|c| c.parse::<u8>().ok()) {
                    let charging = read(&p.join("status")).as_deref() == Some("Charging");
                    out.battery = Some(Battery {
                        level: level.min(100),
                        charging,
                    });
                }
            }
            Some("Mains" | "USB" | "USB_PD") => {
                out.external |= read(&p.join("online")).as_deref() == Some("1")
            }
            _ => {}
        }
    }
    out
}

/// The device's power supplies.
pub fn read_supplies() -> Supplies {
    read_from(Path::new(SYSFS))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn supply(root: &Path, name: &str, files: &[(&str, &str)]) {
        let d = root.join(name);
        fs::create_dir_all(&d).unwrap();
        for (f, v) in files {
            fs::write(d.join(f), format!("{v}\n")).unwrap();
        }
    }

    fn root(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("opendap-ps-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn battery_and_external_power() {
        let r = root("both");
        supply(
            &r,
            "battery",
            &[
                ("type", "Battery"),
                ("capacity", "57"),
                ("status", "Charging"),
            ],
        );
        supply(&r, "usb", &[("type", "USB"), ("online", "1")]);
        assert_eq!(
            read_from(&r),
            Supplies {
                battery: Some(Battery {
                    level: 57,
                    charging: true
                }),
                external: true
            }
        );
    }

    #[test]
    fn levels_above_100_are_clamped_and_unreadable_batteries_skipped() {
        let r = root("clamp");
        supply(&r, "a-broken", &[("type", "Battery"), ("capacity", "n/a")]);
        supply(
            &r,
            "b-battery",
            &[("type", "Battery"), ("capacity", "103"), ("status", "Full")],
        );
        assert_eq!(
            read_from(&r).battery,
            Some(Battery {
                level: 100,
                charging: false
            })
        );
    }

    #[test]
    fn no_class_directory_means_nothing() {
        assert_eq!(
            read_from(Path::new("/nonexistent/power_supply")),
            Supplies::default()
        );
    }
}
