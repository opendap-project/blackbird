// The system layers' design tokens and fonts come from Blackbird UI's published outputs (its
// export/), found through ODAP_BLACKBIRD_UI_DIR or the workspace's repos.tsv, never its sources.
// Glyphs are embedded for the software renderer: the composer needs no font files at run time.
// The tokens the compositor draws with itself come from the same export, as Rust constants.
use std::path::PathBuf;

/// The tokens src/tokens.rs takes from export/css/tokens.css: colours as 8-bit sRGB, durations in ms.
const COLOURS: &[&str] = &["bg", "color-graphite-950"];
const DURATIONS: &[&str] = &["motion-duration-fast", "motion-duration-standard"];

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let ui = opendap_workspace::published(
        "blackbird-ui",
        &manifest,
        &["export"],
        "run blackbird-ui's generators (tokens/generate.py, icons/icons.py)",
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    opendap_workspace::tokens::write_rust_constants(
        &ui.join("export/css/tokens.css"),
        COLOURS,
        DURATIONS,
        &out.join("tokens.rs"),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let config = slint_build::CompilerConfiguration::new()
        .with_library_paths([("blackbird".to_string(), ui.join("export"))].into())
        .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer);
    slint_build::compile_with_config("ui/layers.slint", config).unwrap();
}
