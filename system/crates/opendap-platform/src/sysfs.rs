//! What the status bar shows about the device's hardware, read from sysfs: the radios (rfkill),
//! a microSD card (the block devices) and a wireless interface (the network class). The root is
//! injectable, so each reader is tested on a fixture tree.

use std::{
    fs,
    path::{Path, PathBuf},
};

/// A sysfs tree: `/sys` on a device, a fixture in tests.
#[derive(Debug, Clone)]
pub struct Sysfs {
    root: PathBuf,
}

impl Default for Sysfs {
    fn default() -> Self {
        Sysfs::at("/sys")
    }
}

fn read(p: &Path) -> Option<String> {
    fs::read_to_string(p).ok().map(|s| s.trim().to_string())
}

impl Sysfs {
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Sysfs { root: root.into() }
    }

    fn entries(&self, class: &str) -> Vec<PathBuf> {
        let mut out: Vec<_> = fs::read_dir(self.root.join(class))
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .collect();
        out.sort();
        out
    }

    /// Whether a radio of `kind` (rfkill's `bluetooth`, `wlan`) exists, and if so whether one is
    /// on (neither soft- nor hard-blocked).
    pub fn radio(&self, kind: &str) -> Option<bool> {
        let mut found = None;
        for p in self.entries("class/rfkill") {
            if read(&p.join("type")).as_deref() == Some(kind) {
                let on = read(&p.join("soft")).as_deref() == Some("0")
                    && read(&p.join("hard")).as_deref() == Some("0");
                found = Some(found.unwrap_or(false) || on);
            }
        }
        found
    }

    /// An SD card is in a card slot (an `mmcblk` block device whose device type is `SD`).
    pub fn microsd(&self) -> bool {
        self.entries("block").iter().any(|b| {
            b.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("mmcblk"))
                && read(&b.join("device/type")).as_deref() == Some("SD")
        })
    }

    /// A network interface is wireless.
    pub fn wireless_interface(&self) -> bool {
        self.entries("class/net")
            .iter()
            .any(|n| n.join("wireless").exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(name: &str, files: &[(&str, &str)]) -> Sysfs {
        let root =
            std::env::temp_dir().join(format!("opendap-sysfs-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for (path, text) in files {
            let p = root.join(path);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, format!("{text}\n")).unwrap();
        }
        fs::create_dir_all(&root).unwrap();
        Sysfs::at(root)
    }

    #[test]
    fn a_radio_is_on_when_one_of_its_kind_is_unblocked() {
        let s = tree(
            "radio",
            &[
                ("class/rfkill/rfkill0/type", "bluetooth"),
                ("class/rfkill/rfkill0/soft", "1"),
                ("class/rfkill/rfkill0/hard", "0"),
                ("class/rfkill/rfkill1/type", "wlan"),
                ("class/rfkill/rfkill1/soft", "0"),
                ("class/rfkill/rfkill1/hard", "0"),
            ],
        );
        assert_eq!(s.radio("bluetooth"), Some(false));
        assert_eq!(s.radio("wlan"), Some(true));
        assert_eq!(s.radio("nfc"), None);
    }

    #[test]
    fn only_an_sd_card_in_an_mmc_slot_counts() {
        let s = tree(
            "sd",
            &[
                ("block/mmcblk0/device/type", "MMC"),
                ("block/mmcblk1/device/type", "SD"),
                ("block/sda/device/type", "SD"),
            ],
        );
        assert!(s.microsd());
        let emmc = tree("emmc", &[("block/mmcblk0/device/type", "MMC")]);
        assert!(!emmc.microsd());
    }

    #[test]
    fn a_wireless_interface_has_a_wireless_directory() {
        let s = tree(
            "net",
            &[
                ("class/net/wlan0/wireless/x", ""),
                ("class/net/eth0/type", "1"),
            ],
        );
        assert!(s.wireless_interface());
        assert!(!tree("wired", &[("class/net/eth0/type", "1")]).wireless_interface());
    }
}
