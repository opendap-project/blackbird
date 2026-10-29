//! The device's sound in the window (virtual-devices ADR 0003): played on this computer and
//! measured on the Instruments page; the window's choices kept in view.json.

use super::super::*;

/// A change to the window's choices, saved at once.
pub(crate) type Save = Rc<dyn Fn(&dyn Fn(&mut Prefs))>;

/// The device's sound through the display's session; none when the device was started without
/// its D-Bus audio.
pub(crate) fn sound(
    ui: &View,
    display: &Arc<blackbird_emulator_qemu_display::Display>,
    input_direct: bool,
) -> Option<Arc<sound::Sound>> {
    let weak = ui.as_weak();
    // The spectrograms scroll on the listeners' threads; the window gets each new picture.
    let waterfalls: Arc<std::sync::Mutex<[instruments::Waterfall; 2]>> = Arc::default();
    let attached = sound::Sound::attach(
        display.session(),
        input_direct,
        move |direction, reading| {
            let waterfall = {
                let mut w = waterfalls.lock().unwrap_or_else(|p| p.into_inner());
                let w = &mut w[usize::from(direction == sound::Direction::Input)];
                if reading.active {
                    w.push(&reading.rows);
                }
                w.image()
            };
            let tint = match direction {
                sound::Direction::Output => [59, 130, 196],
                sound::Direction::Input => [255, 159, 10],
            };
            let bars = instruments::bars(&reading.levels);
            let strip = instruments::strip(if reading.active { &reading.bands } else { &[] }, tint);
            let text = instruments::readout(&reading);
            let weak = weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                let Some(ui) = weak.upgrade() else { return };
                let meters = ModelRc::from(Rc::new(VecModel::from(bars)));
                let image = slint::Image::from_rgba8(strip);
                match direction {
                    sound::Direction::Output => {
                        let f = if reading.active {
                            reading.format.clone()
                        } else {
                            "nothing playing".into()
                        };
                        probes_update(&ui, |p| p.out = f);
                        ui.set_out_active(reading.active);
                        ui.set_out_format(reading.format.into());
                        ui.set_out_meters(meters);
                        ui.set_out_spectrum(image);
                        ui.set_out_waterfall(slint::Image::from_rgba8(waterfall));
                        ui.set_out_readout(text.into());
                        ui.set_out_note(reading.note.into());
                    }
                    sound::Direction::Input => {
                        let f = if reading.active {
                            reading.format.clone()
                        } else {
                            "not recording".into()
                        };
                        probes_update(&ui, |p| p.input = f);
                        ui.set_in_active(reading.active);
                        ui.set_in_format(reading.format.into());
                        ui.set_in_meters(meters);
                        ui.set_in_spectrum(image);
                        ui.set_in_waterfall(slint::Image::from_rgba8(waterfall));
                        ui.set_in_readout(text.into());
                        ui.set_in_note(reading.note.into());
                    }
                }
            });
        },
    );
    match attached {
        Ok(s) => Some(Arc::new(s)),
        Err(e) => {
            // A device started without the D-Bus audio (QEMU's own window): no sound here.
            eprintln!("blackbird-emulator-viewer: sound: {e}");
            ui.set_out_note("No sound: the device was started without its D-Bus audio".into());
            None
        }
    }
}

/// The window's choices from last time (view.json), applied, and how a change is kept.
pub(crate) fn prefs(ui: &View, frame: &std::path::Path) -> (Rc<std::cell::RefCell<Prefs>>, Save) {
    let prefs = Rc::new(std::cell::RefCell::new(Prefs::load(frame)));
    let save: Save = {
        let prefs = prefs.clone();
        let frame = frame.to_path_buf();
        Rc::new(move |change: &dyn Fn(&mut Prefs)| {
            change(&mut prefs.borrow_mut());
            prefs.borrow().save(&frame);
        })
    };
    {
        let p = prefs.borrow().clone();
        ui.set_tab(p.tab.clamp(0, 2));
        ui.set_out_analyze(p.out_analyze);
        ui.set_out_play(p.out_play.unwrap_or(true));
        ui.set_in_analyze(p.in_analyze);
        ui.set_out_view(p.out_view.clamp(0, 1));
        ui.set_in_view(p.in_view.clamp(0, 1));
    }
    {
        let save = save.clone();
        ui.on_sound_view(move |which, view| {
            let out = which == "out";
            save(&|p| {
                if out {
                    p.out_view = view
                } else {
                    p.in_view = view
                }
            });
        });
    }
    (prefs, save)
}

/// The sound's switches: measuring, playing on this computer, and the input's source.
pub(crate) fn controls(
    ui: &View,
    sound: Option<Arc<sound::Sound>>,
    prefs: Rc<std::cell::RefCell<Prefs>>,
    save: Save,
) {
    let Some(sound) = sound else {
        ui.on_tab_changed(move |tab| save(&|p| p.tab = tab));
        return;
    };
    let p = prefs.borrow().clone();
    // Measuring starts once the Instruments page shows (the window's `changed` handlers).
    let shown = ui.get_sidebar() && ui.get_tab() == 1;
    sound.set_analyze(sound::Direction::Output, p.out_analyze && shown);
    sound.set_analyze(sound::Direction::Input, p.in_analyze && shown);
    if p.out_play == Some(false) {
        let s = sound.clone();
        std::thread::spawn(move || s.set_play(false));
    }
    let (sa, sb, sc) = (sound.clone(), sound.clone(), sound);
    {
        let save = save.clone();
        ui.on_sound_analyze(move |which, on| {
            sa.set_analyze(
                if which == "out" {
                    sound::Direction::Output
                } else {
                    sound::Direction::Input
                },
                on,
            );
        });
        ui.on_sound_measure_wish(move |which, on| {
            let out = which == "out";
            save(&|p| {
                if out {
                    p.out_analyze = on
                } else {
                    p.in_analyze = on
                }
            });
        });
    }
    {
        let save = save.clone();
        // A switch's change can wait on the host's audio: off the window's thread.
        ui.on_sound_play(move |on| {
            let sb = sb.clone();
            std::thread::spawn(move || sb.set_play(on));
            save(&|p| p.out_play = Some(on));
        });
    }
    sources(ui, sc, prefs, save);
}

/// The sources the list offers, by index: the default input, each of the host's inputs, the
/// test tone, silence. Read again whenever the Instruments page opens (a microphone plugged in
/// since shows up). The choice is kept by its key, so it survives a list that changed.
fn sources(ui: &View, sc: Arc<sound::Sound>, prefs: Rc<std::cell::RefCell<Prefs>>, save: Save) {
    let sources: Rc<std::cell::RefCell<Vec<sound::InSource>>> = Rc::default();
    let refresh = {
        let sources = sources.clone();
        let prefs = prefs.clone();
        let weak = ui.as_weak();
        move || {
            let (devices, default) = sound::input_devices();
            let Some(ui) = weak.upgrade() else { return };
            let mut list = vec![sound::InSource::Device(None)];
            let mut names: Vec<slint::SharedString> = vec![
                default
                    .map_or("This computer's default input".into(), |d| {
                        format!("Default input ({d})")
                    })
                    .into(),
            ];
            for d in devices {
                names.push(d.name.into());
                list.push(sound::InSource::Device(Some(d.id)));
            }
            list.extend([sound::InSource::Tone, sound::InSource::Silence]);
            names.extend(["Test tone 997 Hz".into(), "Silence".into()]);
            let key = prefs
                .borrow()
                .in_source
                .clone()
                .unwrap_or_else(|| "default".into());
            let index = list.iter().position(|x| source_key(x) == key).unwrap_or(0);
            *sources.borrow_mut() = list;
            ui.set_in_sources(ModelRc::from(Rc::new(VecModel::from(names))));
            ui.set_in_source(index as i32);
        }
    };
    refresh();
    // The kept source, applied once the list is known.
    if let Some(source) = sources.borrow().get(ui.get_in_source() as usize).cloned() {
        let sc = sc.clone();
        std::thread::spawn(move || sc.set_source(source));
    }
    {
        let refresh = refresh.clone();
        let save = save.clone();
        ui.on_tab_changed(move |tab| {
            save(&|p| p.tab = tab);
            if tab == 1 {
                refresh();
            }
        });
    }
    ui.on_sound_source(move |i| {
        let sc = sc.clone();
        let Some(source) = sources.borrow().get(i as usize).cloned() else {
            return;
        };
        let key = source_key(&source);
        save(&|p| p.in_source = Some(key.clone()));
        std::thread::spawn(move || sc.set_source(source));
    });
}
