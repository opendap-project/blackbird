// The design tokens, fonts and icons come from Blackbird UI's published outputs (its export/),
// found through ODAP_BLACKBIRD_UI_DIR or the workspace's repos.tsv, never from its sources.
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let ui =
        opendap_workspace::repo_dir("blackbird-ui", &manifest).unwrap_or_else(|e| panic!("{e}"));
    let export = ui.join("export");
    for sub in ["slint", "fonts", "icons"] {
        if !export.join(sub).is_dir() {
            panic!(
                "{} not found: run blackbird-ui's generators (tokens/generate.py, icons/icons.py)",
                export.join(sub).display()
            );
        }
    }
    println!(
        "cargo:rerun-if-env-changed={}",
        opendap_workspace::env_var("blackbird-ui")
    );
    println!("cargo:rerun-if-changed={}", export.display());

    // Debug builds carry the element tree's debug info, so the interface tests (ADR 0014, L2) can
    // find elements by their accessible names; release builds, the ones that ship, do not.
    let debug = std::env::var("PROFILE").is_ok_and(|p| p == "debug");
    let config = slint_build::CompilerConfiguration::new()
        .with_library_paths([("blackbird".to_string(), export)].into())
        .with_debug_info(debug);
    slint_build::compile_with_config("ui/testbed.slint", config).unwrap();
}
