// The splash screen's lockup and the tokens it draws with are Blackbird UI's published outputs (its
// brand file and export/css/tokens.css), found through ODAP_BLACKBIRD_DIR or the workspace's
// repos.tsv, never its sources.
use std::path::PathBuf;

const LOCKUP: &str = "brand/svg/opendap-vertical-light.svg";
const TOKENS: &str = "export/css/tokens.css";

/// The tokens src/lib.rs takes from export/css/tokens.css: colours as 8-bit sRGB, durations in ms.
const COLOURS: &[&str] = &["bg"];
const DURATIONS: &[&str] = &["motion-duration-slow"];

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let ui = opendap_workspace::published(
        "blackbird-ui",
        &manifest,
        &[LOCKUP, TOKENS],
        "run blackbird-ui's generators (tokens/generate.py)",
    )
    .unwrap_or_else(|e| panic!("{e}"));
    println!(
        "cargo:rustc-env=OPENDAP_LOCKUP_SVG={}",
        ui.join(LOCKUP).canonicalize().unwrap().display()
    );
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    opendap_workspace::tokens::write_rust_constants(
        &ui.join(TOKENS),
        COLOURS,
        DURATIONS,
        &out.join("tokens.rs"),
    )
    .unwrap_or_else(|e| panic!("{e}"));
}
