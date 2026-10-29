//! The touch indicator's marks (linux/touches.rs), as numbers apart from how they are drawn, so
//! they are tested on any host: the two shapes' pixels and how they move.

use std::time::Duration;

use crate::tokens;

/// The ring's white band and its dark outer edge (for light content), in logical pixels.
pub const RING_WIDTH: f64 = 2.0;
pub const EDGE_WIDTH: f64 = 1.0;

/// The two shapes, drawn at each diameter they are shown at (in physical pixels) and kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shape {
    Ring,
    Disc,
}

/// How far `elapsed` is into `total`, 0 to 1.
pub fn progress(elapsed: Duration, total: Duration) -> f64 {
    (elapsed.as_secs_f64() / total.as_secs_f64()).clamp(0.0, 1.0)
}

pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// `motion.easing.standard` (blackbird-ui-motion's, the design system's curve).
pub fn ease(t: f64) -> f64 {
    f64::from(blackbird_ui_motion::easing::Bezier::STANDARD.at(t as f32))
}

/// The shape's premultiplied (B, G, R, A) at distance `r` from the centre, for a texture `px`
/// physical pixels across at `scale`.
pub fn shade(shape: Shape, r: f64, px: i32, scale: f64) -> [f64; 4] {
    let outer = px as f64 / 2.0;
    let edge = EDGE_WIDTH * scale;
    match shape {
        // A white band with a dark outer edge, so it shows on light content too.
        Shape::Ring => {
            let white = band(r, outer - edge - RING_WIDTH * scale, outer - edge);
            let dark = band(r, outer - edge, outer) * 0.5 * (1.0 - white);
            // Graphite 950 for the edge, in the buffer's BGRA order.
            let c = |d: u8| white + dark * d as f64 / 255.0;
            let [red, green, blue] = tokens::COLOR_GRAPHITE_950;
            [c(blue), c(green), c(red), white + dark]
        }
        // A white disc inside the ring's edge.
        Shape::Disc => {
            let a = band(r, -1.0, outer - edge);
            [a, a, a, a]
        }
    }
}

/// Anti-aliased coverage of the annulus between `inner` and `outer` (one pixel of smoothing).
fn band(r: f64, inner: f64, outer: f64) -> f64 {
    ((r - inner + 0.5).clamp(0.0, 1.0)).min((outer - r + 0.5).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_easing_runs_from_0_to_1_and_front_loads() {
        assert!(ease(0.0).abs() < 1e-6);
        assert!((ease(1.0) - 1.0).abs() < 1e-6);
        assert!(ease(0.5) > 0.5);
    }

    #[test]
    fn progress_is_clamped_and_lerp_runs_between_its_ends() {
        let ms = Duration::from_millis;
        assert_eq!(progress(ms(50), ms(200)), 0.25);
        assert_eq!(progress(ms(400), ms(200)), 1.0);
        assert_eq!(lerp(32.0, 26.0, 0.5), 29.0);
    }

    #[test]
    fn the_ring_is_hollow_and_the_disc_is_full() {
        let (px, scale) = (32, 1.0);
        assert_eq!(shade(Shape::Ring, 0.0, px, scale)[3], 0.0);
        assert!(shade(Shape::Ring, 16.0 - EDGE_WIDTH - RING_WIDTH / 2.0, px, scale)[3] > 0.99);
        assert!(shade(Shape::Disc, 0.0, px, scale)[3] > 0.99);
        assert_eq!(shade(Shape::Disc, 17.0, px, scale)[3], 0.0);
    }
}
