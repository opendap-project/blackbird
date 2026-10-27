//! The device's load and its processes (stats.rs), read while the system group or the Tasks
//! page shows.

use super::super::*;

type Last = Arc<std::sync::Mutex<Option<stats::Frame>>>;
type Kept = Arc<std::sync::Mutex<history::History>>;

pub(crate) fn system(ui: &View, checkout: &std::path::Path) {
    let watch: Rc<std::cell::RefCell<Option<stats::Watch>>> = Rc::default();
    // The last frame, for the Tasks list to sort and filter again without waiting for the next.
    let last: Arc<std::sync::Mutex<Option<stats::Frame>>> = Arc::default();
    let history: Arc<std::sync::Mutex<history::History>> = Arc::default();
    let again = {
        let (last, weak, history) = (last.clone(), ui.as_weak(), history.clone());
        move || {
            let frame = last.lock().unwrap_or_else(|p| p.into_inner()).clone();
            if let (Some(ui), Some(f)) = (weak.upgrade(), frame) {
                show_tasks(&ui, &f, &history.lock().unwrap_or_else(|p| p.into_inner()));
            }
        }
    };
    tasks(ui, Rc::new(again), last.clone(), checkout.to_path_buf());
    watch_load(ui, checkout.to_path_buf(), watch, last, history);
}

/// The Tasks list's sorting, choosing and actions.
fn tasks(ui: &View, again: Rc<dyn Fn()>, last: Last, checkout: PathBuf) {
    {
        let again = again.clone();
        let weak = ui.as_weak();
        ui.on_task_sort_by(move |key| {
            let Some(ui) = weak.upgrade() else { return };
            // The same column again turns the order; a new one starts from its natural end
            // (busiest, largest first; names and PIDs from the start).
            if ui.get_task_sort() == key {
                ui.set_task_descending(!ui.get_task_descending());
            } else {
                ui.set_task_descending(matches!(key.as_str(), "cpu" | "mem"));
                ui.set_task_sort(key);
            }
            again();
        });
    }
    {
        let again = again.clone();
        ui.on_task_show_kernel(move |_| again());
    }
    {
        // The pointer left the list: it takes its sorted order again.
        let again = again.clone();
        ui.on_task_hover_changed(move || again());
    }
    {
        let again = again.clone();
        let weak = ui.as_weak();
        ui.on_task_choose(move |pid| {
            if let Some(ui) = weak.upgrade() {
                ui.set_task_pid(pid);
            }
            again();
        });
    }
    {
        let last = last.clone();
        let weak = ui.as_weak();

        ui.on_task_act(move |what| {
            let Some(ui) = weak.upgrade() else { return };
            let action = match what.as_str() {
                "end" => stats::Action::End,
                "kill" => stats::Action::Kill,
                "pause" => stats::Action::Pause,
                "resume" => stats::Action::Resume,
                "lower" => stats::Action::Lower,
                "raise" => stats::Action::Raise,
                _ => return,
            };
            let pid = ui.get_task_pid();
            let chosen = last
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .as_ref()
                .and_then(|f| f.processes.iter().find(|p| p.pid as i32 == pid).cloned());
            let Some(p) = chosen else { return };
            let (checkout, weak) = (checkout.clone(), ui.as_weak());
            // Over SSH: off the window's thread; the next frame shows the result.
            std::thread::spawn(move || {
                let r = stats::act(&checkout, action, &p);
                let _ = slint::invoke_from_event_loop(move || {
                    if let (Some(ui), Err(e)) = (weak.upgrade(), r) {
                        ui.set_note(format!("{}: {e}", p.name).into());
                    }
                });
            });
        });
    }
}

/// The load, the energy and the processes, as each frame arrives.
fn watch_load(
    ui: &View,
    checkout: PathBuf,
    watch: Rc<std::cell::RefCell<Option<stats::Watch>>>,
    last: Last,
    history: Kept,
) {
    let weak = ui.as_weak();
    ui.on_sys_watch(move |on| {
        if !on {
            watch.borrow_mut().take();
            return;
        }
        let (weak, last, history) = (weak.clone(), last.clone(), history.clone());
        *watch.borrow_mut() = Some(stats::Watch::start(checkout.clone(), move |frame| {
            *last.lock().unwrap_or_else(|p| p.into_inner()) = Some(frame.clone());
            history
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .record(&frame);
            let history = history.clone();
            let weak = weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                let Some(ui) = weak.upgrade() else { return };
                let l = instruments::load(&frame);
                ui.set_sys_known(true);
                ui.set_sys_cpu(l.cpu.into());
                ui.set_sys_cpu_busy(l.cpu_busy);
                ui.set_sys_mem(l.mem.into());
                ui.set_sys_mem_busy(l.mem_busy);
                ui.set_sys_load(l.load.into());
                ui.set_sys_uptime(l.uptime.into());
                if let Some(p) = &frame.power {
                    let e = instruments::energy(p);
                    let summary = format!("{}, {}", e.level, e.power);
                    probes_update(&ui, |v| v.battery = summary);
                    ui.set_energy_known(true);
                    ui.set_energy_level(e.level.into());
                    ui.set_energy_power(e.power.into());
                    ui.set_energy_status(e.status.into());
                    ui.set_energy_voltage(e.voltage.into());
                    ui.set_energy_current(e.current.into());
                    ui.set_energy_temperature(e.temperature.into());
                    ui.set_energy_time(e.time.into());
                    ui.set_energy_charger(e.charger.into());
                    let h = history.lock().unwrap_or_else(|p| p.into_inner());
                    let green = [48, 164, 108];
                    ui.set_graph_battery(slint::Image::from_rgba8(instruments::graph(
                        &h.battery.values(),
                        100.0,
                        green,
                    )));
                    // Power drawn shows as positive; the scale follows the largest seen.
                    let drawn: Vec<f32> = h.power.values().iter().map(|w| w.abs()).collect();
                    let top = drawn.iter().copied().fold(1.0f32, f32::max);
                    ui.set_graph_power(slint::Image::from_rgba8(instruments::graph(
                        &drawn,
                        top * 1.25,
                        [255, 159, 10],
                    )));
                }
                show_tasks(
                    &ui,
                    &frame,
                    &history.lock().unwrap_or_else(|p| p.into_inner()),
                );
            });
        }));
    });
}
