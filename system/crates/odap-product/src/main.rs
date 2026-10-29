//! `blackbird-apps-player`: the product host (ADR 0012). Bring-up: the Now Playing place.
//!
//! What it shows comes through ports, so the same screen runs on the device, on the virtual T1
//! and on a developer host:
//! - the track comes from a fixture until the audio engine and library services exist;
//! - the status bar is not the product's: the composer draws it, as a system layer above the
//!   top 28 px of every activity (composer and UI runtime spec §2).

use slint::ComponentHandle;

slint::include_modules!();

/// The track to show, until `/usr/lib/blackbird/library` and the audio engine provide it over the bus.
fn fixture_track() -> TrackView {
    TrackView {
        title: "You Only Live Once".into(),
        artist: "The Strokes".into(),
        album: "First Impressions of Earth".into(),
        format: "FLAC 16/44.1 · 4.4 mm".into(),
        path: "Direct".into(),
        position: "1:19".into(),
        remaining: "−1:50".into(),
        progress: 0.42,
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let product = Product::new()?;
    product.set_track(fixture_track());

    let weak = product.as_weak();
    product.on_toggle_play(move || {
        if let Some(p) = weak.upgrade() {
            p.set_playing(!p.get_playing());
        }
    });

    product.run()
}

#[cfg(test)]
mod tests {
    /// The status bar is the composer's system layer (composer and UI runtime spec §2): the
    /// product draws none of its items, so the top of the screen shows one bar.
    #[test]
    fn the_product_draws_no_status_bar() {
        let ui = include_str!("../ui/product.slint");
        assert!(
            !ui.contains("icons/16/status-"),
            "the status set is the composer's"
        );
        assert!(!ui.contains("StatusBar"), "no status bar of its own");
    }
}
