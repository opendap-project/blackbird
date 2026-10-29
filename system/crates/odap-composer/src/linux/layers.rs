//! The system layers (spec §2, §7): Slint components the composer draws itself, above every client;
//! what they show today (the status bar, the power menu, toasts) are OS Shell drafts (RFC 0012),
//! with Slint's software renderer into a buffer the compositor places on top. While a layer is up,
//! touch and the pointer go to it only: no client sees them.
//!
//! Today: the power menu, beside `main`, and the debug UI mode (SYS-070): the blueprint's
//! references over the real interface, while its flag is on. That layer takes no input.

use std::{cell::RefCell, rc::Rc, time::Instant};

use blackbird_ui_input::gesture::Level;
use slint::{
    ComponentHandle, LogicalPosition, PhysicalSize,
    platform::{
        Platform, PointerEventButton, WindowAdapter, WindowEvent,
        software_renderer::{MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType},
    },
};
use smithay::{
    backend::{
        allocator::Fourcc,
        renderer::{
            element::{
                Kind,
                memory::{MemoryRenderBuffer, MemoryRenderBufferRenderElement},
            },
            gles::GlesRenderer,
        },
    },
    utils::{Logical, Point, Transform},
};
use tracing::warn;

use crate::power_menu::{self, Activation, Request};

slint::include_modules!();

/// Slint's platform for the composer: one window, the layers, drawn on demand.
struct ComposerPlatform {
    window: Rc<MinimalSoftwareWindow>,
    start: Instant,
}

impl Platform for ComposerPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.window.clone())
    }

    fn duration_since_start(&self) -> std::time::Duration {
        self.start.elapsed()
    }
}

/// The debug UI mode's runtime switch; the environment's default is `BLACKBIRD_DEBUG_UI`.
const DEBUG_SWITCH: &str = blackbird_platform::run::DEBUG_UI;

/// Whether the debug UI mode is on: the switch file, or the environment.
pub fn debug_ui() -> bool {
    // The switch file overrides the environment's default: `1` on, `0` off (a simulator's
    // switch can turn off what the board's support package turned on).
    match std::fs::read_to_string(DEBUG_SWITCH) {
        Ok(v) => v.trim() != "0",
        Err(_) => std::env::var("BLACKBIRD_DEBUG_UI").is_ok_and(|v| v == "1"),
    }
}

pub struct SystemLayers {
    window: Rc<MinimalSoftwareWindow>,
    ui: Layers,
    width: usize,
    height: usize,
    pixels: Vec<PremultipliedRgbaColor>,
    buffer: Option<MemoryRenderBuffer>,
    requests: Rc<RefCell<Vec<Request>>>,
    controls: Rc<slint::VecModel<ControlMark>>,
    contact: Option<blackbird_ui_input::gesture::Contact>,
    /// When the composer started, for the contacts' times.
    started: Instant,
    /// Logical pixels per design pixel (the hardware's exact scale over the applied one).
    unit: f32,
    /// Counts the gestures shown, so a callout's timer clears only its own.
    gesture_seq: u64,
}

impl SystemLayers {
    /// Logical pixels per design pixel.
    pub fn unit(&self) -> f32 {
        self.unit
    }

    /// For a display of `size` physical pixels at `scale` physical pixels per logical pixel, with
    /// `main` centred `main_y` logical pixels from the top (the device description).
    /// `controls`: each physical control's role, edge, centre and length (logical pixels); `unit`:
    /// logical pixels per design pixel.
    pub fn new(
        size: (u32, u32),
        scale: f32,
        main_y: f32,
        controls: Vec<(String, String, f32, f32)>,
        unit: f32,
    ) -> Option<Self> {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        let platform = ComposerPlatform {
            window: window.clone(),
            start: Instant::now(),
        };
        if let Err(err) = slint::platform::set_platform(Box::new(platform)) {
            warn!(%err, "system layers");
            return None;
        }
        window.dispatch_event(WindowEvent::ScaleFactorChanged {
            scale_factor: scale,
        });
        window.set_size(PhysicalSize::new(size.0, size.1));
        let ui = match Layers::new() {
            Ok(ui) => ui,
            Err(err) => {
                warn!(%err, "system layers");
                return None;
            }
        };
        ui.set_main_y(main_y);
        ui.set_unit(unit);
        let marks = Rc::new(slint::VecModel::from(
            controls
                .into_iter()
                .map(|(role, edge, centre, span)| ControlMark {
                    role: role.into(),
                    edge: edge.into(),
                    centre,
                    span,
                    down: false,
                })
                .collect::<Vec<_>>(),
        ));
        ui.set_controls(slint::ModelRc::from(marks.clone()));
        ui.set_debug(debug_ui());
        let requests = Rc::new(RefCell::new(Vec::new()));
        let sink = requests.clone();
        ui.on_power_choice(move |what| {
            sink.borrow_mut().push(power_menu::choice(what.as_str()));
        });
        if let Err(err) = ui.show() {
            warn!(%err, "system layers");
            return None;
        }
        let (width, height) = (size.0 as usize, size.1 as usize);
        Some(Self {
            window,
            ui,
            width,
            height,
            pixels: vec![PremultipliedRgbaColor::default(); width * height],
            buffer: None,
            requests,
            controls: marks,
            contact: None,
            started: Instant::now(),
            unit,
            gesture_seq: 0,
        })
    }

    /// Whether the status bar shows (its 28 px are then the system's, not the activity's).
    pub fn status_shown(&self) -> bool {
        self.ui.get_status_shown()
    }

    pub fn power_open(&self) -> bool {
        self.ui.get_power_open()
    }

    /// Whether any layer is on screen.
    fn visible(&self) -> bool {
        self.power_open()
            || self.ui.get_status_shown()
            || !self.ui.get_toast().is_empty()
            || self.ui.get_debug()
    }

    /// The status bar's state (status.rs).
    pub fn set_status(&mut self, s: &super::status::Status) {
        use crate::status_bar::{Radio, battery_glyph, battery_low, labels};
        let (bluetooth, wifi) = (
            Radio::from_reading(s.bluetooth),
            Radio::from_reading(s.wifi),
        );
        let l = labels(&s.time, s.microsd, bluetooth, wifi, s.battery, s.charging);
        self.ui.set_status_shown(true);
        self.ui.set_status_time(s.time.as_str().into());
        self.ui.set_status_microsd(s.microsd);
        self.ui.set_status_battery(s.battery.map_or(-1, i32::from));
        self.ui
            .set_status_battery_glyph(s.battery.map_or(0, |p| i32::from(battery_glyph(p))));
        self.ui
            .set_status_battery_low(s.battery.is_some_and(|p| battery_low(p, s.charging)));
        self.ui.set_status_charging(s.charging);
        self.ui.set_status_bluetooth_shown(bluetooth.shown());
        self.ui.set_status_bluetooth_dimmed(bluetooth.dimmed());
        self.ui.set_status_wifi_shown(wifi.shown());
        self.ui.set_status_wifi_dimmed(wifi.dimmed());
        self.ui.set_status_labels(StatusLabels {
            time: l.time.into(),
            microsd: l.microsd.into(),
            bluetooth: l.bluetooth.into(),
            wifi: l.wifi.into(),
            battery: l.battery.into(),
        });
        self.window.request_redraw();
    }

    /// Reads the debug UI mode's flag again; true when it changed.
    pub fn refresh_debug(&mut self) -> bool {
        let on = debug_ui();
        if on == self.ui.get_debug() {
            return false;
        }
        self.ui.set_debug(on);
        self.window.request_redraw();
        if !self.visible() {
            self.buffer = None;
        }
        true
    }

    /// A physical control went down or up (every role, system combinations included).
    pub fn control(&mut self, role: &str, down: bool) {
        use slint::Model;
        for i in 0..self.controls.row_count() {
            let mut m = self.controls.row_data(i).unwrap();
            if m.role == role && m.down != down {
                m.down = down;
                self.controls.set_row_data(i, m);
                self.window.request_redraw();
            }
        }
    }

    /// A contact for the debug UI mode, at `at` (logical pixels): its crosshair and region while
    /// it is down; when it lifts, the gesture it was, to show for a while (the caller clears it).
    pub fn debug_contact(&mut self, at: Point<f64, Logical>, event: Contact) -> Option<u64> {
        if !self.ui.get_debug() {
            return None;
        }
        let at = (at.x as f32, at.y as f32);
        let size = (
            self.width as f32 / self.window.scale_factor(),
            self.height as f32 / self.window.scale_factor(),
        );
        let now = self.started.elapsed().as_millis() as u64;
        // The screen frame: the composer recognizes at the screen level (input events §3.2.1).
        let surface = blackbird_ui_input::gesture::Frame::screen(size, self.unit);
        let mut gesture = None;
        match event {
            Contact::Down => {
                self.contact = Some(blackbird_ui_input::gesture::Contact::down(at, now, surface));
                self.ui.set_touch_zone(zone(at, surface).into());
                self.ui.set_touching(true);
            }
            // A pan or a drag is named live, phase by phase.
            Contact::Moved => {
                gesture = self
                    .contact
                    .as_mut()
                    .and_then(|c| c.moved(at, now))
                    .map(|g| g.describe_at(Level::Screen));
            }
            Contact::Up => {
                if let Some(mut c) = self.contact.take() {
                    // The last of what it was (a swipe over the pan it ends).
                    gesture = c.up(at, now).last().map(|g| g.describe_at(Level::Screen));
                }
                self.ui.set_touching(false);
            }
            Contact::Cancel => {
                gesture = self
                    .contact
                    .take()
                    .and_then(|mut c| c.cancel())
                    .map(|g| g.describe_at(Level::Screen));
                self.ui.set_touching(false);
            }
        }
        self.ui.set_touch_x(at.0);
        self.ui.set_touch_y(at.1);
        self.window.request_redraw();
        let ended = matches!(event, Contact::Up | Contact::Cancel);
        if let Some(text) = gesture {
            self.ui.set_gesture(text.as_str().into());
            self.gesture_seq += 1;
        }
        // The toast stays while the contact lasts (a long press held, a pan or a drag going on),
        // and its timeout starts when the contact ends.
        (ended && self.ui.get_gesture() != "").then_some(self.gesture_seq)
    }

    /// A long press, if the contact is still down within the slop: named while the finger is
    /// still on the screen, until it lifts.
    pub fn debug_long_press(&mut self) {
        let now = self.started.elapsed().as_millis() as u64;
        let Some(g) = self.contact.as_mut().and_then(|c| c.long_due(now)) else {
            return;
        };
        self.ui.set_gesture(g.describe_at(Level::Screen).into());
        self.window.request_redraw();
        // Held: it leaves when the finger lifts (`debug_contact`).
        self.gesture_seq += 1;
    }

    /// The gesture's callout leaves, unless a newer gesture took its place.
    pub fn clear_gesture(&mut self, seq: u64) {
        if seq == self.gesture_seq && self.contact.is_none() {
            self.ui.set_gesture("".into());
            self.window.request_redraw();
        }
    }

    /// A short message that leaves by itself; the composer clears it with `hide_toast`.
    pub fn show_toast(&mut self, text: &str) {
        self.ui.set_toast(text.into());
        self.window.request_redraw();
    }

    pub fn hide_toast(&mut self) {
        self.ui.set_toast("".into());
        self.window.request_redraw();
        if !self.visible() {
            self.buffer = None;
        }
    }

    pub fn open_power(&mut self, can_recovery: bool) {
        self.ui.set_can_recovery(can_recovery);
        self.ui.set_point_at_main(false);
        // Focus on entry (design system §8.4): the first choice.
        self.ui.set_focused_choice(0);
        self.ui.set_power_open(true);
        self.window.request_redraw();
    }

    pub fn close(&mut self) {
        self.ui.set_power_open(false);
        self.window.request_redraw();
        if !self.visible() {
            self.buffer = None;
        }
    }

    /// Volume + (`-1`, up) or − (`1`, down): the focus moves (crate::power_menu).
    pub fn move_focus(&mut self, step: i32) {
        let n = power_menu::choices(self.ui.get_can_recovery());
        let next = power_menu::move_focus(self.ui.get_focused_choice(), step, n);
        self.ui.set_focused_choice(next);
        self.window.request_redraw();
    }

    /// Action: the screen's primary action, or else the focused element (T1 controls spec §1.3;
    /// crate::power_menu).
    pub fn activate_focus(&mut self) {
        let n = power_menu::choices(self.ui.get_can_recovery());
        match power_menu::activate(self.ui.get_focused_choice(), n) {
            Some(Activation::PointAtMain) => {
                self.ui.set_point_at_main(true);
                self.window.request_redraw();
            }
            Some(Activation::Request(r)) => self.requests.borrow_mut().push(r),
            None => {}
        }
    }

    /// Whether a layer takes input now; then no client gets it.
    pub fn takes_input(&self) -> bool {
        self.power_open()
    }

    /// A contact at `at` (logical pixels): down, moved, or up.
    pub fn contact(&mut self, at: Point<f64, Logical>, event: Contact) {
        let position = LogicalPosition::new(at.x as f32, at.y as f32);
        let event = match event {
            Contact::Down => WindowEvent::PointerPressed {
                position,
                button: PointerEventButton::Left,
            },
            Contact::Moved => WindowEvent::PointerMoved { position },
            Contact::Up => WindowEvent::PointerReleased {
                position,
                button: PointerEventButton::Left,
            },
            // Slint has no pointer cancel: the pointer leaves, so nothing it pressed acts.
            Contact::Cancel => WindowEvent::PointerExited,
        };
        let released = matches!(event, WindowEvent::PointerReleased { .. });
        self.window.dispatch_event(event);
        // A finger leaves the screen: nothing stays hovered.
        if released {
            self.window.dispatch_event(WindowEvent::PointerExited);
        }
    }

    pub fn take_requests(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.requests.borrow_mut())
    }

    /// Whether Slint has animations running (frames must keep coming).
    pub fn animating(&self) -> bool {
        self.visible() && self.window.has_active_animations()
    }

    /// The layers' element, above every client, while a layer is up.
    pub fn element(
        &mut self,
        renderer: &mut GlesRenderer,
    ) -> Option<MemoryRenderBufferRenderElement<GlesRenderer>> {
        if !self.visible() {
            return None;
        }
        slint::platform::update_timers_and_animations();
        let (width, height) = (self.width, self.height);
        let pixels = &mut self.pixels;
        let drawn = self.window.draw_if_needed(|r| {
            pixels.fill(PremultipliedRgbaColor::default());
            r.render(pixels, width);
        });
        if drawn || self.buffer.is_none() {
            // Slint's premultiplied RGBA to the compositor's ARGB8888 (B, G, R, A bytes).
            let mut bytes = Vec::with_capacity(width * height * 4);
            for p in &self.pixels {
                bytes.extend_from_slice(&[p.blue, p.green, p.red, p.alpha]);
            }
            self.buffer = Some(MemoryRenderBuffer::from_slice(
                &bytes,
                Fourcc::Argb8888,
                (width as i32, height as i32),
                1,
                Transform::Normal,
                None,
            ));
        }
        let buffer = self.buffer.as_ref()?;
        MemoryRenderBufferRenderElement::from_buffer(
            renderer,
            (0.0, 0.0),
            buffer,
            None,
            None,
            None,
            Kind::Unspecified,
        )
        .map_err(|err| warn!(?err, "system layers"))
        .ok()
    }
}

/// The region a contact starts in, for the debug UI mode to light it (`blackbird-ui-input`'s
/// regions: corners of their own where two edge zones meet).
fn zone(at: (f32, f32), surface: blackbird_ui_input::gesture::Frame) -> &'static str {
    use blackbird_ui_input::gesture::{Corner, Edge, Region};
    match surface.region(at) {
        Region::Content => "",
        Region::Edge(Edge::Leading) => "edge-leading",
        Region::Edge(Edge::Trailing) => "edge-trailing",
        Region::Edge(Edge::Top) => "edge-top",
        Region::Edge(Edge::Bottom) => "edge-bottom",
        Region::Corner(Corner::TopLeading) => "corner-top-leading",
        Region::Corner(Corner::TopTrailing) => "corner-top-trailing",
        Region::Corner(Corner::BottomLeading) => "corner-bottom-leading",
        Region::Corner(Corner::BottomTrailing) => "corner-bottom-trailing",
    }
}

#[derive(Clone, Copy)]
pub enum Contact {
    Down,
    Moved,
    Up,
    /// The system took the contact (a system view opened, the display went off).
    Cancel,
}
