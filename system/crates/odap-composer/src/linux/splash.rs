//! The splash screen (blackbird-ui-splash; composer and UI runtime spec §9): the composer's first frames show
//! the same splash screen the boot UI (`blackbird-apps-boot`) put on the display while the OS loaded, keep it at least
//! `MINIMUM_MS` from when it first showed, then fade it out once the product host has drawn
//! (crate::splash_timeline decides when).
//! Nothing else is drawn at boot (OS-035).

use std::time::{Duration, Instant};

use blackbird_ui_splash::{FADE_MS, MINIMUM_MS, SHOWN_AT_PATH};
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
    reexports::rustix::time::{ClockId, clock_gettime},
    utils::{Size, Transform},
};
use tracing::warn;

use crate::splash_timeline::{Timeline, minimum_left};

pub struct Splash {
    timeline: Timeline,
    buffer: MemoryRenderBuffer,
}

fn monotonic_ms() -> u64 {
    let t = clock_gettime(ClockId::Monotonic);
    t.tv_sec as u64 * 1000 + t.tv_nsec as u64 / 1_000_000
}

impl Splash {
    pub fn new(display: Size<i32, smithay::utils::Physical>) -> Self {
        let (w, h) = (display.w.max(1) as u32, display.h.max(1) as u32);
        let pixels = blackbird_ui_splash::screen_xrgb(w, h, w as usize * 4);
        // B, G, R, 0xff bytes: ARGB8888 on little-endian hosts, opaque.
        let buffer = MemoryRenderBuffer::from_slice(
            &pixels,
            Fourcc::Argb8888,
            (w as i32, h as i32),
            1,
            Transform::Normal,
            None,
        );
        // The minimum counts from when blackbird-apps-boot first showed it, or from now without it.
        let shown = std::fs::read_to_string(SHOWN_AT_PATH).ok();
        let left = minimum_left(shown.as_deref(), monotonic_ms(), MINIMUM_MS);
        Self {
            timeline: Timeline::new(Instant::now() + left, Duration::from_millis(FADE_MS)),
            buffer,
        }
    }

    /// The product host drew its first frame.
    pub fn client_ready(&mut self) {
        self.timeline.client_ready();
    }

    /// Whether the splash screen has left the display for good.
    pub fn done(&self) -> bool {
        self.timeline.done()
    }

    /// Whether frames must keep coming: only while fading.
    pub fn animating(&self) -> bool {
        self.timeline.animating()
    }

    /// When the composer must look again: the end of the minimum, once the product host is ready.
    pub fn wake_at(&self) -> Option<Instant> {
        self.timeline.wake_at(Instant::now())
    }

    /// The splash screen's element above the client, if it is still on screen.
    pub fn element(
        &mut self,
        renderer: &mut GlesRenderer,
    ) -> Option<MemoryRenderBufferRenderElement<GlesRenderer>> {
        let alpha = self.timeline.alpha(Instant::now())?;
        MemoryRenderBufferRenderElement::from_buffer(
            renderer,
            (0.0, 0.0),
            &self.buffer,
            Some(alpha),
            None,
            None,
            Kind::Unspecified,
        )
        .map_err(|err| warn!(?err, "splash screen"))
        .ok()
    }
}
