//! The platform contract shared by Blackbird OS programs: the paths through which they hand state to each other.
//! Each path is named once here, with who writes it and who reads it; programs never spell a path of their own.

#[cfg(target_os = "linux")]
pub mod kernel;
pub mod power_supply;
pub mod sysfs;

/// The run directory: runtime state that lives until the next boot.
pub mod run {
    /// The run directory itself.
    pub const DIR: &str = "/run/blackbird";

    /// The device description the hardware hands over at boot (read through `opendap_device::locate`).
    pub const DEVICE_DESCRIPTION: &str = "/run/blackbird/device/description.toml";

    /// The boot UI's directory: the hand-over and the cover's state.
    pub const BOOT_DIR: &str = "/run/blackbird/boot";
    /// Written by `odap-boot handoff` to ask the boot UI for the display.
    pub const BOOT_HANDOFF: &str = "/run/blackbird/boot/handoff";
    /// Written by the boot UI once it has released the display.
    pub const BOOT_RELEASED: &str = "/run/blackbird/boot/released";
    /// The running boot UI's process id, so a hand-over knows whether to wait.
    pub const BOOT_PID: &str = "/run/blackbird/boot/pid";
    /// The boot UI covering a composer restart is armed: display open, splash ready, waiting for DRM master.
    pub const BOOT_ARMED: &str = "/run/blackbird/boot/armed";
    /// The boot UI's splash screen is on the display.
    pub const BOOT_ON_SCREEN: &str = "/run/blackbird/boot/on-screen";
    /// When the splash screen first showed, in CLOCK_MONOTONIC milliseconds (written by `odap-boot`).
    pub const SPLASH_SHOWN: &str = "/run/blackbird/boot/splash-shown";

    /// The composer's runtime directory.
    pub const COMPOSER_DIR: &str = "/run/blackbird/composer";
    /// Written by the composer at its first frame on screen: booting ends (read by the status LED).
    pub const COMPOSER_UI_READY: &str = "/run/blackbird/composer/ui-ready";
    /// The activity the composer starts in place of the product, kept across composer restarts.
    pub const ACTIVITY: &str = "/run/blackbird/composer/activity";
    /// The debug switches' directory (created at boot, `tmpfiles.d`).
    pub const DEBUG_DIR: &str = "/run/blackbird/debug";
    /// The debug UI mode's runtime switch (the environment's default is `BLACKBIRD_DEBUG_UI`).
    pub const DEBUG_UI: &str = "/run/blackbird/debug/ui";
    /// The show-touches runtime switch (the environment's default is `BLACKBIRD_SHOW_TOUCHES`).
    pub const SHOW_TOUCHES: &str = "/run/blackbird/composer/show-touches";

    /// Requests to the status LED, one file per requester.
    pub const LED_REQUESTS: &str = "/run/blackbird/statusled/requests";
    /// The power service's socket (blackbird-os `blackbird-power.socket`).
    pub const POWER_SOCKET: &str = "/run/blackbird/power/control.sock";

    /// Every path above, for checks.
    pub const ALL: &[&str] = &[
        DEVICE_DESCRIPTION,
        BOOT_HANDOFF,
        BOOT_RELEASED,
        BOOT_PID,
        BOOT_ARMED,
        BOOT_ON_SCREEN,
        SPLASH_SHOWN,
        COMPOSER_DIR,
        COMPOSER_UI_READY,
        ACTIVITY,
        DEBUG_UI,
        SHOW_TOUCHES,
        LED_REQUESTS,
        POWER_SOCKET,
    ];
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn every_path_is_inside_the_run_directory_and_named_once() {
        let mut seen = std::collections::HashSet::new();
        for p in run::ALL {
            assert!(
                p.starts_with(&format!("{}/", run::DIR)),
                "{p} is outside {}",
                run::DIR
            );
            assert!(seen.insert(*p), "{p} is named twice");
        }
    }

    #[test]
    fn composer_paths_live_in_the_composer_directory() {
        assert!(run::COMPOSER_UI_READY.starts_with(run::COMPOSER_DIR));
    }
}
