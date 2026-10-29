//! The device's controls from the window: its keys, the mouse on them and the host's keyboard.

use super::super::*;

/// A control pressed or released: sent to the device, and shown pressed on the window.
pub(crate) type Press = Rc<dyn Fn(&str, bool)>;

/// The controls, in order, on a thread of their own: the window never waits for QMP. The
/// channel takes the window's presses and a keyboard's keys (← →), in order with the buttons.
pub(crate) fn sender(
    qmp: &std::path::Path,
    qcode_of: &[(String, String)],
) -> mpsc::Sender<InputEvent> {
    let (key_tx, key_rx) = mpsc::channel::<InputEvent>();
    {
        let live = LiveKeys::new(
            qmp,
            qcode_of
                .iter()
                // The model's controls name roles of the shared vocabulary (RFC 0020); a name that
                // is not one cannot be sent, and says so.
                .filter_map(|(r, q)| match r.parse::<Role>() {
                    Ok(role) => Some((role, q.clone())),
                    Err(e) => {
                        eprintln!("blackbird-emulator-viewer: {e}");
                        None
                    }
                })
                .collect(),
        );
        std::thread::spawn(move || {
            // ODAP_VIEW_TRACE=1: every control event, as sent, on stderr (window.log).
            let trace = std::env::var_os("ODAP_VIEW_TRACE").is_some();
            for event in key_rx {
                if trace {
                    eprintln!("blackbird-emulator-viewer: {event:?}");
                }
                if let Err(e) = live.send(&event) {
                    eprintln!("blackbird-emulator-viewer: {}", e.message);
                }
            }
        });
    }
    key_tx
}

/// What a press does: sent (unless the control is already so) and drawn.
pub(crate) fn press(
    ui: &View,
    key_tx: mpsc::Sender<InputEvent>,
    keys_model: Rc<VecModel<Key>>,
) -> Press {
    let weak = ui.as_weak();
    Rc::new(move |role: &str, down: bool| {
        // Already so (a key the OS repeats, the mouse and a key on the same control): nothing.
        let same = (0..keys_model.row_count())
            .filter_map(|i| keys_model.row_data(i))
            .any(|k| k.role == role && k.pressed == down);
        if same {
            return;
        }
        let Ok(role) = role.parse::<Role>() else {
            return;
        };
        let _ = key_tx.send(if down {
            InputEvent::KeyDown { role }
        } else {
            InputEvent::KeyUp { role }
        });
        let role = role.as_str();
        if let Some(ui) = weak.upgrade() {
            match role {
                "action" => ui.set_action_down(down),
                "main" => ui.set_main_down(down),
                "volume-up" => ui.set_volume_up_down(down),
                "volume-down" => ui.set_volume_down_down(down),
                _ => {}
            }
        }
        for i in 0..keys_model.row_count() {
            let mut k = keys_model.row_data(i).unwrap();
            if k.role == role {
                k.pressed = down;
                keys_model.set_row_data(i, k);
            }
        }
    })
}

/// The window's own key shapes and the S combo (Main + Volume −).
pub(crate) fn mouse(ui: &View, press: &Press) {
    {
        let press = press.clone();
        let trace = std::env::var_os("ODAP_VIEW_TRACE").is_some();
        ui.on_key(move |role, down| {
            if trace {
                eprintln!("blackbird-emulator-viewer: key {role} {down}");
            }
            press(role.as_str(), down)
        });
    }
    {
        let press = press.clone();
        let trace = std::env::var_os("ODAP_VIEW_TRACE").is_some();
        ui.on_combo(move |down| {
            if trace {
                eprintln!("blackbird-emulator-viewer: combo {down}");
            }
            press("main", down);
            press("volume-down", down);
        });
    }
}

/// The host keys the faceplate gives (Space, ↑, ↓, P): their role, by their qcode.
pub(crate) fn host_keys(
    ui: &View,
    doc: &FrameDoc,
    press: &Press,
    keyboard_tx: mpsc::Sender<InputEvent>,
) {
    let host_keys: Vec<(String, String)> = doc
        .host_keys
        .iter()
        .map(|h| (h.key.clone(), h.role.clone()))
        .collect();
    let press = press.clone();
    let trace = std::env::var_os("ODAP_VIEW_TRACE").is_some();
    ui.on_host_key(move |text, down| {
        if trace {
            eprintln!(
                "blackbird-emulator-viewer: host key {:?} {down}",
                text.as_str()
            );
        }
        // ↑ and ↓ are the buttons by their place (Volume + above, Volume − below), as the
        // faceplate's host keys say. ← and → are no button (the buttons stand vertically):
        // they reach the device as a keyboard's arrows, so a focused slider takes them
        // (→ more, ← less). Enter is Action, as Space is: the key that takes the default
        // action or the focused element; which of the two Action does is the device's call.
        match text.as_str() {
            "\n" | "\r" => return press("action", down),
            "\u{F702}" | "\u{F703}" => {
                let key = if text.as_str() == "\u{F702}" {
                    "left"
                } else {
                    "right"
                };
                let _ = keyboard_tx.send(InputEvent::Keyboard {
                    key: key.into(),
                    down,
                });
                return;
            }
            _ => {}
        }
        let qcode = match text.as_str() {
            " " => "spc",
            "\u{F700}" => "up",
            "\u{F701}" => "down",
            t => t,
        };
        if let Some((_, role)) = host_keys.iter().find(|(q, _)| q == qcode) {
            press(role, down);
        }
    });
}
