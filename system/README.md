# System

Blackbird OS system software in Rust: bus, orchestrator, UI runtime, SDK and base services.

## Layout

- `crates/`: the Rust workspace
  - Domain and primitives (platform-agnostic, tested on the host):
    - `blackbird-device`: the device description's domain model (display, controls and their roles)
    - `blackbird-ui-input`: the input domain (gestures, controls by role, semantic actions, thresholds)
    - `blackbird-ui-layout`: layout for large content (virtual lists and grids, anchoring, recycling, prefetch)
    - `blackbird-ui-media`: media primitives (variants, decode sizes and scheduling, caching tiers, audio buffering, plots)
    - `blackbird-ui-motion`: motion primitives (springs, easing, fling decay, interruptible animations, frame budget)
    - `blackbird-ui-forms`: fields and forms (text editing, field kinds, binding, validation)
    - `blackbird-services-audio-analysis`: audio measurements (meters, FFT spectrum, THD, THD+N, SNR, bit-exact comparison)
    - `blackbird-ui-splash`: the splash screen's rasterization, shared by `blackbird-apps-boot` and the composer
    - `blackbird-platform`: the platform contract between programs: every runtime path (the run directory and its markers), named once
  - Programs:
    - `blackbird-composer`: the composer, the compositor and interface runtime on Smithay (bring-up; RFC 0012)
    - `blackbird-apps-player`: the product host in Slint (bring-up: Now Playing)
    - `blackbird-apps-boot`: the boot UI (splash screen, the hand-over to the composer)
    - `blackbird-apps-statusled`: the status LED daemon
    - `blackbird-apps-testbed`: the UI primitives activity (SYS-071)
  - Build support: `opendap-workspace`, finding other repositories' checkouts from build scripts and turning Blackbird UI's published tokens (`export/css/tokens.css`) into Rust constants
- `schemas/`
- `docs/`

## Building

Development happens on the host; the virtual T1 only runs what is built ([development workflow](https://github.com/opendap-project/tools/blob/main/env/docs/guides/maintainers/development-workflow.md)).

```sh
cargo test --workspace                 # domain and core crates, natively
cargo bench -p blackbird-ui-layout -p blackbird-ui-input -p blackbird-ui-media -p blackbird-ui-motion   # UI primitive benchmarks (criterion, each crate's benches/)
cargo run -p blackbird-apps-player              # the product UI in a window on the host
odapkit sysroot install                   # once, and when the image's packages change (toolkit-image)
odapkit deploy                            # cross-build and run on the running virtual device (toolkit-image)
```

Checks: pre-commit runs rustfmt on the Rust files a commit changes; `pre-commit install --hook-type pre-push` adds clippy (`-D warnings`) and the workspace tests before each push. [`check.toml`](check.toml) lists the stages (fast, full), including clippy for the device target through `odapkit cross`.

Tests never inherit a hook's `GIT_*` variables: under pre-commit, git exports `GIT_DIR`, `GIT_INDEX_FILE` and others, and a test whose git inherits them acts on the repository being committed. The Rust tests remove them from every child process (`env_remove`), and `tests/e2e/run.py` after its imports. Each language's protection is in the Developer Toolkit's [rule](https://github.com/opendap-project/tools#tests-never-inherit-a-hooks-git-variables); `odapkit audit` flags a test file that runs git without it.

`blackbird-apps-player`, `blackbird-composer` and `blackbird-ui-splash` read Blackbird UI's published outputs (`export/`, `brand/svg/`), found through the workspace's `repos.tsv` or `ODAP_BLACKBIRD_DIR`; the colours and durations they draw with themselves come from `export/css/tokens.css`, never typed by hand. `odapkit deploy` takes its default crates and the units to restart from `[workspace.metadata.odap]` in `Cargo.toml`.

## License

This repository follows the [REUSE](https://reuse.software) layout: license texts live in `LICENSES/` and `REUSE.toml` maps each path to its license.

- Default: **GNU GPL v3.0 or later** (`GPL-3.0-or-later`)
- Markdown files: **Creative Commons Attribution-ShareAlike 4.0** (`CC-BY-SA-4.0`)
- `schemas/`: **Mozilla Public License 2.0** (`MPL-2.0`)
- `docs/`: **Creative Commons Attribution-ShareAlike 4.0** (`CC-BY-SA-4.0`)
- SDK crates, the libraries others build on: **Mozilla Public License 2.0** (`MPL-2.0`): `blackbird-device` (device model), `blackbird-platform` (platform contract), `blackbird-ui-input`, `blackbird-ui-layout`, `blackbird-ui-motion`, `blackbird-ui-forms`, `blackbird-ui-media` (UI primitives) and `blackbird-services-audio-analysis` (audio measurements). The `odap-*` programs, `blackbird-ui-splash` and `opendap-workspace` stay `GPL-3.0-or-later`.

Copyright © 2026 The OpenDAP Project.
