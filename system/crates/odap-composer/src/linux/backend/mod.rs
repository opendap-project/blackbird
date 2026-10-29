//! Display output: DRM/KMS with GBM buffers and the GLES renderer, on a libseat session.
//!
//! Whatever the hardware offers is used: the first connected connector, its preferred mode and
//! the CRTC and planes it can drive. Nothing about a board is assumed (hardware support spec §9).
//! A frame is drawn only after a client commits, and at most once per refresh (spec §5).

use std::{os::fd::AsFd, path::PathBuf, time::Duration};

use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay::{
    backend::{
        allocator::{
            Fourcc,
            gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
        },
        drm::{
            DrmDevice, DrmDeviceFd, DrmEvent, DrmNode,
            compositor::{DrmCompositor, FrameFlags},
            exporter::gbm::{GbmFramebufferExporter, NodeFilter},
        },
        egl::{EGLContext, EGLDisplay},
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        renderer::{Color32F, ImportDma, ImportMemWl, gles::GlesRenderer},
        session::{Event as SessionEvent, Session, libseat::LibSeatSession},
        udev,
    },
    output::{Mode as OutputMode, Output, PhysicalProperties, Scale, Subpixel},
    reexports::{
        drm::control::{Device as _, ModeTypeFlags},
        input::Libinput,
        rustix::fs::OFlags,
    },
    utils::{DeviceFd, Transform},
};
use tracing::{info, warn};

use smithay::backend::renderer::element::{
    AsRenderElements, memory::MemoryRenderBufferRenderElement, surface::WaylandSurfaceRenderElement,
};
use smithay::utils::Scale as UtilScale;

use super::{splash::Splash, state::Composer};

mod init;
mod render;

use crate::tokens::{BG, unit};
pub use init::init;
pub use render::render_if_idle;

type Compositor =
    DrmCompositor<GbmAllocator<DrmDeviceFd>, GbmFramebufferExporter<DrmDeviceFd>, (), DrmDeviceFd>;

/// The design system's background (`semantic.bg`, graphite 950), shown where no client draws.
const CLEAR: Color32F = Color32F::new(unit(BG[0]), unit(BG[1]), unit(BG[2]), 1.0);

smithay::backend::renderer::element::render_elements! {
    pub ComposerElement<=GlesRenderer>;
    Window=WaylandSurfaceRenderElement<GlesRenderer>,
    Memory=MemoryRenderBufferRenderElement<GlesRenderer>,
}

pub struct Backend {
    splash: Splash,
    splash_timer: bool,
    touches_timer: bool,
    session: LibSeatSession,
    libinput: Libinput,
    renderer: GlesRenderer,
    compositor: Compositor,
    drm: DrmDevice,
    start: std::time::Instant,
    frame_pending: bool,
    redraw: bool,
    /// The display's power: off after a short press of `main` (controls.rs).
    display_off: bool,
    /// A screenshot is asked for (Main + Volume −): taken with the next frame.
    screenshot: bool,
}

impl Backend {
    pub fn schedule(&mut self) {
        self.redraw = true;
    }

    pub fn request_screenshot(&mut self) {
        self.screenshot = true;
        self.redraw = true;
    }

    pub fn display_on(&self) -> bool {
        !self.display_off
    }

    /// The display off (DPMS off, every plane disabled; clients get no frame callbacks, so they
    /// stop drawing) or on again with a full frame.
    pub fn set_display(&mut self, on: bool) {
        if on == !self.display_off {
            return;
        }
        if on {
            self.display_off = false;
            if let Err(err) = self.compositor.reset_state() {
                warn!(?err, "display on");
            }
            self.redraw = true;
        } else {
            if let Err(err) = self.compositor.clear() {
                warn!(?err, "display off");
            }
            self.display_off = true;
            self.frame_pending = false;
        }
    }

    /// Lets the display go to a boot UI that is armed to cover this composer's restart (blackbird-apps-boot
    /// cover): drops DRM master with the last frame still on screen, and waits until the boot UI's
    /// splash screen has replaced it, so the display shows no blank frame. Without an armed boot UI
    /// it returns at once.
    pub fn hand_over(&self) {
        use blackbird_ui_splash::{COVER_ARMED_PATH, COVER_SHOWN_PATH};
        use smithay::reexports::drm::Device as _;
        if !std::path::Path::new(COVER_ARMED_PATH).exists() {
            return;
        }
        if let Err(err) = self.drm.device_fd().release_master_lock() {
            warn!(%err, "cannot let the display go");
            return;
        }
        let deadline = std::time::Instant::now() + HAND_OVER_TIMEOUT;
        while !std::path::Path::new(COVER_SHOWN_PATH).exists()
            && std::time::Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

/// How long a toast stays (Android's short toast).
const TOAST: Duration = Duration::from_secs(2);

/// The longest a stopping composer waits for the boot UI to take the display.
const HAND_OVER_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("session: {0}")]
    Session(String),
    #[error("no display device on this seat")]
    NoGpu,
    #[error("display device: {0}")]
    Drm(String),
    #[error("no connected display")]
    NoConnector,
    #[error("renderer: {0}")]
    Render(String),
}

fn frame_done(composer: &mut Composer) {
    if let (Some(backend), Some(output)) = (composer.backend.as_mut(), composer.output.as_ref()) {
        if let Err(err) = backend.compositor.frame_submitted() {
            warn!(?err, "frame submitted");
        }
        backend.frame_pending = false;
        send_frames(&composer.space, output, backend.start.elapsed());
    }
    render_if_idle(composer);
}

/// One frame callback per refresh, to the visible surfaces only (spec §5).
fn send_frames(
    space: &smithay::desktop::Space<smithay::desktop::Window>,
    output: &Output,
    time: Duration,
) {
    for window in space.elements() {
        window.send_frame(output, time, Some(Duration::ZERO), |_, _| {
            Some(output.clone())
        });
    }
}
