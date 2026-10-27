# ADR 0008: Rust dependency set

- **Status:** Accepted
- **Date:** 2026-10-22

## Context

Every OpenDAP Rust repository needs one dependency set, chosen in 34 areas by backing, maintenance, adoption, standards conformance, licence, portability, footprint and API stability. Seven choices needed a decision (O1–O7).

## Decision

- **The table below is the default dependency set** for every OpenDAP Rust repository (RFC 0002). Adding a crate outside it needs a short justification against the same criteria.
- **O1, SSH:** the CLI's SSH adapter uses `openssh` (the system's OpenSSH). If OpenDAP Desktop ships on Windows, a second adapter on `russh` implements the same port; the core does not change (Rust crate architecture §4).
- **O2, Smithay:** the compositor is a binary, not a published library, so it may depend on Smithay pinned to a git revision until a release carries the input-method hooks; the pin is reviewed at every Smithay release.
- **O3, CBOR:** not used. JSON for control and configuration, `zerocopy` frames for the data plane.
- **O4, HTTP:** one stack only, `reqwest` with `rustls` (its blocking API where a tool is synchronous); no `ureq`.
- **O5, SQLite:** bundled (`rusqlite` with the bundled feature) on host and device, so FTS5 and the SQLite version are fixed and reproducible.
- **O6, messages:** **Project Fluent** is the single message format for the system, templates, capabilities and the desktop app. Slint strings are fed from Fluent by OpenDAP's binding, so there are never two translation formats.
- **O7, EQ:** OpenDAP's own RBJ biquads in the audio engine (small, audio-critical, bit-exact control, co-designed with the DAC volume path).
- **Logging:** `tracing` to journald on the device; no OpenTelemetry anywhere in the platform.
- **QMP:** OpenDAP's own thin client over `serde_json`; no maintained crate exists.

### Default dependency set

| Area | Crate(s) | Where | Licence |
|---|---|---|---|
| Argument parsing, completions, man | `clap` (derive), `clap_complete`, `clap_mangen` | toolkit | MIT OR Apache-2.0 |
| Colour, progress | `anstream`/`anstyle`, `indicatif` | toolkit | MIT OR Apache-2.0; MIT |
| Errors | `thiserror` (libraries), `anyhow` (binaries) | all | MIT OR Apache-2.0 |
| Logging | `tracing`, `tracing-subscriber` (`env-filter`, `json`), `tracing-journald` (device) | all | MIT |
| Serialisation, schema | `serde`, `serde_json`, `toml`, `toml_edit`, `schemars` | all | MIT OR Apache-2.0; MIT |
| Data-plane frames | `zerocopy` | system | BSD-2-Clause OR Apache-2.0 OR MIT |
| Async runtime | `tokio` (≥ 1.54.1, minimal features); `calloop` in the compositor | all | MIT |
| System calls, signals, mmap | `rustix`, `signal-hook` / `tokio::signal`, `memmap2` (≥ 0.9.11) | all | Apache-2.0 OR MIT family |
| Directories, config | `etcetera` (XDG strategy) + our own layering | toolkit | MIT OR Apache-2.0 |
| HTTP | `reqwest` (`rustls`, no default features) | toolkit, `dap-fetch` | MIT OR Apache-2.0 |
| TLS, crypto | `rustls` (≥ 0.23.46) + `aws-lc-rs`, `rustls-platform-verifier`, `ed25519-dalek`, `minisign-verify`, `sha2` | all | permissive |
| TUF | `tough` | device, toolkit | MIT OR Apache-2.0 |
| Compression | `zstd`, `flate2`, `liblzma` (if xz is needed) | toolkit, device | BSD-3-Clause; MIT OR Apache-2.0 |
| Archives, flashing images | `tar` (≥ 0.4.46), `bmap-parser` | toolkit | MIT OR Apache-2.0 |
| SSH | `openssh` (+ `ssh-key` for pairing keys) | toolkit (Unix hosts) | MIT OR Apache-2.0 |
| USB, MaskROM | `nusb`, `rockusb` | toolkit | Apache-2.0 OR MIT |
| QMP | our own thin client (`serde_json` + `tokio`) | toolkit | — |
| Serial | `serialport` | toolkit | MPL-2.0 |
| Varlink | `zlink` | system, toolkit | MIT |
| D-Bus | `zbus` | system | MIT |
| systemd | `sd-notify`, `tracing-journald`; units over `zbus` | system | MIT OR Apache-2.0; MIT |
| Wayland compositor | `smithay` (pin; see O2 below) | system | MIT |
| UI | `slint` (+ `accesskit` through it) | system | GPL-3.0-only |
| Images | `image` (`png`, `jpeg`, `webp` only) | system | MIT OR Apache-2.0 |
| Audio output | `alsa` (exclusive), `pipewire` (shared), `audio_thread_priority` | system | Apache-2.0 OR MIT; MIT; MPL-2.0 |
| Decoding | `symphonia` (PCM codecs) + our own DSF/DFF reader and DSD converter | system | MPL-2.0 |
| Resampling | `rubato` | system | MIT OR Apache-2.0 |
| EQ, spectra | our own RBJ biquads; `realfft` | system | — ; MIT |
| Tags | `lofty` + `id3` (DSF/DFF) | system | MIT OR Apache-2.0; MIT |
| Library database, search | `rusqlite` + SQLite FTS5 | system | MIT (SQLite: public domain) |
| Keyboard dictionaries | `fst` | system | Unlicense OR MIT |
| File watching | `notify` + `notify-debouncer-full` | system, toolkit | CC0-1.0; MIT OR Apache-2.0 |
| i18n | `icu`, `fluent-bundle` | system | Unicode-3.0; Apache-2.0 OR MIT |
| Keyring | `keyring` (opt-in feature) | toolkit | MIT OR Apache-2.0 |
| MCP | `rmcp` (stdio) | toolkit | Apache-2.0 |
| Time, IDs | `jiff`, `uuid` | all | Unlicense OR MIT; Apache-2.0 OR MIT |
| mDNS (host) | `mdns-sd` | toolkit | Apache-2.0 OR MIT |
| Hardware | `gpiocdev`, `i2cdev`, `evdev` | system | Apache-2.0 OR MIT |
| Testing | `insta`, `proptest`, `assert_cmd` + `predicates`, `tempfile` | all (dev) | Apache-2.0; MIT OR Apache-2.0 |
| Benchmarking | `criterion` | all (dev) | Apache-2.0 OR MIT |
| FFI | `bindgen`, `cc`, `pkg-config` (only for C libraries without bindings) | as needed | BSD-3-Clause; MIT OR Apache-2.0 |

## Consequences

- One dependency set to audit for licences and RustSec advisories, checked by the weekly security check.
- The Windows path for OpenDAP Desktop stays open without affecting the CLI.
