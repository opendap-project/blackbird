// The design tokens, fonts and icons come from Blackbird UI's published outputs (its export/),
// found through ODAP_BLACKBIRD_UI_DIR or the workspace's repos.tsv, never from its sources.
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let ui = opendap_workspace::published(
        "blackbird-ui",
        &manifest,
        &["export/slint", "export/fonts", "export/icons"],
        "run blackbird-ui's generators (tokens/generate.py, icons/icons.py)",
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let config = slint_build::CompilerConfiguration::new()
        .with_library_paths([("blackbird".to_string(), ui.join("export"))].into());
    slint_build::compile_with_config("ui/product.slint", config).unwrap();
}
