//! The design tokens the compositor draws with itself (the clear colour, the touch indicator), from
//! Blackbird UI's published tokens (`export/css/tokens.css`), read by build.rs as the layers' Slint
//! tokens are: never typed by hand.

include!(concat!(env!("OUT_DIR"), "/tokens.rs"));

/// An 8-bit sRGB channel as the renderer's 0 to 1.
pub const fn unit(c: u8) -> f32 {
    c as f32 / 255.0
}

#[cfg(test)]
mod tests {
    use super::*;

    // The values these tokens had when the composer typed them by hand.
    #[test]
    fn every_token_keeps_its_value() {
        assert_eq!(BG, [0x11, 0x13, 0x15]);
        assert_eq!(COLOR_GRAPHITE_950, [0x11, 0x13, 0x15]);
        assert_eq!(MOTION_DURATION_FAST_MS, 120);
        assert_eq!(MOTION_DURATION_STANDARD_MS, 200);
    }

    // The clear colour was written as 0.0667, 0.0745, 0.0824: the same 8-bit colour.
    #[test]
    fn the_clear_colour_is_the_one_typed_before() {
        for (c, old) in BG.into_iter().zip([0.0667f32, 0.0745, 0.0824]) {
            assert!((unit(c) - old).abs() < 5e-5, "{c} against {old}");
            assert_eq!((old * 255.0).round() as u8, c);
        }
    }
}
