//! Media geometry over generated pictures and boxes (ADR 0014, level L1): the frosted backdrop
//! of any picture at any factor and radius, and content of any shape in a box of any other. A
//! failure is shrunk to the smallest case that breaks one.

use blackbird_ui_media::blur::frosted;
use blackbird_ui_media::fit::{Fit, Rect, decode_size, place};
use proptest::prelude::*;

const EPS: f32 = 0.05;

fn picture() -> impl Strategy<Value = (Vec<u8>, usize, usize)> {
    (1usize..40, 1usize..40).prop_flat_map(|(w, h)| {
        (
            prop::collection::vec(any::<u8>(), w * h * 4),
            Just(w),
            Just(h),
        )
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 1500, ..ProptestConfig::default() })]

    /// A blur only mixes what is there: the small picture has the size asked, every channel stays
    /// between the darkest and the brightest of that channel in the source, and a flat picture
    /// stays exactly flat.
    #[test]
    fn frosting_only_mixes_what_is_there((rgba, w, h) in picture(), factor in 1usize..9, radius in 0usize..40) {
        let (out, sw, sh) = frosted(&rgba, w, h, factor, radius);
        prop_assert_eq!((sw, sh), (w.div_ceil(factor), h.div_ceil(factor)));
        prop_assert_eq!(out.len(), sw * sh * 4);
        for c in 0..4 {
            let src = rgba.iter().skip(c).step_by(4);
            let (lo, hi) = src.fold((255u8, 0u8), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
            prop_assert!(out.iter().skip(c).step_by(4).all(|v| *v >= lo && *v <= hi), "channel {c} within {lo}..{hi}");
        }
        let px = [rgba[0], rgba[1], rgba[2], rgba[3]];
        let flat: Vec<u8> = px.iter().copied().cycle().take(w * h * 4).collect();
        let (f, _, _) = frosted(&flat, w, h, factor, radius);
        prop_assert!(f.chunks(4).all(|p| p == px), "flat stays flat");
    }
}

fn bounds() -> impl Strategy<Value = Rect> {
    (
        -100.0f32..100.0,
        -100.0f32..100.0,
        1.0f32..1000.0,
        1.0f32..1000.0,
    )
        .prop_map(|(x, y, w, h)| Rect { x, y, w, h })
}

fn within(inner: Rect, outer: Rect) -> bool {
    inner.x >= outer.x - EPS
        && inner.y >= outer.y - EPS
        && inner.x + inner.w <= outer.x + outer.w + EPS
        && inner.y + inner.h <= outer.y + outer.h + EPS
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 5000, ..ProptestConfig::default() })]

    /// Content in a box: contain and scale-down land inside the box with the source's shape (and
    /// contain touches two sides; scale-down never enlarges); cover fills the box with a centred
    /// part of the source of the box's shape; none shows the source at its size, clipped; fill is
    /// the box. The part shown is always inside the source.
    #[test]
    fn content_fits_its_box(src in (1.0f32..4000.0, 1.0f32..4000.0), b in bounds()) {
        let full = Rect { x: 0.0, y: 0.0, w: src.0, h: src.1 };
        let ratio = src.0 / src.1;
        for fit in [Fit::Contain, Fit::Cover, Fit::Fill, Fit::None, Fit::ScaleDown] {
            let (dest, part) = place(src, b, fit);
            prop_assert!(within(part, full), "{fit:?}: part {part:?} inside the source");
            match fit {
                Fit::Contain | Fit::ScaleDown => {
                    prop_assert!(within(dest, b), "{fit:?}: inside the box");
                    prop_assert!((dest.w / dest.h - ratio).abs() / ratio < 1e-3, "{fit:?}: the source's shape");
                    if fit == Fit::Contain { prop_assert!((dest.w - b.w).abs() < EPS || (dest.h - b.h).abs() < EPS, "touches two sides"); }
                    if fit == Fit::ScaleDown { prop_assert!(dest.w <= src.0 + EPS); }
                }
                Fit::Cover => {
                    prop_assert_eq!(dest, b);
                    prop_assert!((part.w / part.h - b.w / b.h).abs() / (b.w / b.h) < 1e-3, "the box's shape");
                    prop_assert!(((part.x + part.w / 2.0) - src.0 / 2.0).abs() < EPS && ((part.y + part.h / 2.0) - src.1 / 2.0).abs() < EPS, "centred");
                }
                Fit::Fill => prop_assert_eq!(dest, b),
                Fit::None => {
                    prop_assert!(within(dest, b));
                    prop_assert!((dest.w - part.w).abs() < EPS && (dest.h - part.h).abs() < EPS, "one source pixel, one box pixel");
                }
            }
        }
    }

    /// The size to decode at: never more than the source, never zero, and never less than what
    /// lands on screen (in physical pixels) when the source has that much.
    #[test]
    fn decode_never_exceeds_the_source(src in (1u32..6000, 1u32..6000), b in (1.0f32..800.0, 1.0f32..800.0), scale in 1.0f32..3.0) {
        for fit in [Fit::Contain, Fit::Cover, Fit::Fill, Fit::None, Fit::ScaleDown] {
            let (w, h) = decode_size(src, b, scale, fit);
            prop_assert!(w >= 1 && h >= 1 && w <= src.0 && h <= src.1, "{fit:?}: {w}x{h} of {src:?}");
            if fit == Fit::Contain {
                let (dest, _) = place((src.0 as f32, src.1 as f32), Rect { x: 0.0, y: 0.0, w: b.0 * scale, h: b.1 * scale }, fit);
                prop_assert!(w as f32 + 1.0 >= dest.w.min(src.0 as f32), "{w} covers {}", dest.w);
            }
        }
    }
}
