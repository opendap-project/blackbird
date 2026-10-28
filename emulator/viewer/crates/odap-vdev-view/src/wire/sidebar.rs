//! The sidebar's actions on the device, on the worker; the window shows the state it publishes.

use super::super::*;

/// Starts the worker; the caller keeps it for as long as the window runs.
pub(crate) fn sidebar(ui: &View, checkout: &std::path::Path, name: &str) -> device::Worker {
    let worker = {
        let weak = ui.as_weak();
        device::Worker::start(checkout.to_path_buf(), move |st| {
            let weak = weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = weak.upgrade() {
                    show(&ui, &st);
                }
            });
        })
    };
    actions(ui, worker.jobs.clone(), name.to_string());
    switches(ui, worker.jobs.clone());
    worker
}

/// Save PNG, reboot, reboot into debug, power off.
fn actions(ui: &View, jobs: mpsc::Sender<device::Job>, device_name: String) {
    let weak = ui.as_weak();
    ui.on_action(move |what| {
        let Some(ui) = weak.upgrade() else { return };
        let job = match what.as_str() {
            // Save PNG: the display as the host sees it, to the Desktop, with no part of the OS involved.
            "screenshot" => {
                // In flight from the click: a second click never saves a second picture.
                if ui.get_saving() {
                    return;
                }
                ui.set_saving(true);
                let shots = std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_default()
                    .join("Desktop");
                device::Job::SavePng(shots.join(format!("{device_name} {}.png", stamp())))
            }
            "reboot" => device::Job::Power(PowerAction::Reboot {
                mode: BootMode::Normal,
            }),
            "reboot-debug" => device::Job::Power(PowerAction::Reboot {
                mode: BootMode::Debug,
            }),
            "poweroff" => device::Job::Power(PowerAction::Poweroff),
            _ => return,
        };
        let _ = jobs.send(job);
    });
}

/// The battery, the light, the debug UI mode and the attachments.
fn switches(ui: &View, jobs: mpsc::Sender<device::Job>) {
    {
        let jobs = jobs.clone();
        ui.on_battery_set(move |level| {
            let _ = jobs.send(device::Job::Battery(level.round().clamp(0.0, 100.0) as u8));
        });
    }
    {
        let jobs = jobs.clone();
        ui.on_light_set(move |position| {
            let _ = jobs.send(device::Job::Light(slider_to_lux(position)));
        });
    }
    let weak = ui.as_weak();
    let debug_jobs = jobs.clone();
    let debug_weak = ui.as_weak();
    ui.on_set_debug_ui(move |on| {
        if let Some(ui) = debug_weak.upgrade() {
            // In flight from the click, as the other switches.
            match (ui.get_debug_ui(), on) {
                (0, true) => ui.set_debug_ui(1),
                (2, false) => ui.set_debug_ui(3),
                _ => return,
            }
        }
        let _ = debug_jobs.send(device::Job::DebugUi(on));
    });
    ui.on_attach(move |what, on| {
        let Some(ui) = weak.upgrade() else { return };
        let (a, get, set): (Attachment, Getter, Setter) = match what.as_str() {
            "microsd" => (Attachment::Microsd, View::get_sd, View::set_sd),
            "usb-storage" => (
                Attachment::UsbStorage,
                View::get_usb_disk,
                View::set_usb_disk,
            ),
            "usb-audio" => (Attachment::UsbAudio, View::get_usb_dac, View::set_usb_dac),
            "charger" => (Attachment::Charger, View::get_charger, View::set_charger),
            _ => return,
        };
        // In flight from the click, even while the worker finishes another change: the
        // worker's next state confirms it, or puts it back if it does not apply.
        match (get(&ui), on) {
            (0, true) => set(&ui, 1),
            (2, false) => set(&ui, 3),
            _ => return,
        }
        let _ = jobs.send(device::Job::Attach(a, on));
    });
}
