# Rust crate architecture

- **Status:** Accepted ([RFC 0002](https://github.com/opendap-project/rfcs/blob/main/text/0002-rust-crate-architecture.md))
- **Date:** 2026-10-22
- **Applies to:** every OpenDAP Rust repository: system (services, SDK), the `opendap` CLI and its plugins (host), OpenDAP Desktop (host), the toolkit repositories that ship Rust (toolkit-verify), firmware where Rust is used.

The goal is reuse and portability: a capability is written once and reaches every surface (CLI, OpenDAP Desktop, device services, agents), and core code runs on every supported host and on the device.

## 1. Crates first

- Every capability is a **library crate**. A binary crate only parses input, wires dependencies and renders output; it holds no logic another surface could need.
- A crate has one purpose, a small public API and documentation on every public item.
- Prefer several focused crates over one large one; split when two consumers need different parts.

## 2. Layers

Dependencies point inward only. An inner layer never depends on an outer one.

| Layer | Contains | May depend on | Example |
|---|---|---|---|
| **Domain model** | Types and rules of the domain: devices, targets, modes, tracks, capabilities, errors as data. No I/O, no async runtime, no platform code | std, `serde`, small pure crates | `opendap-host-model` |
| **Application core** | Use cases expressed against **ports** (traits): discover devices, run a command, transfer a file, flash an image, build an image | domain model | `opendap-host-core` |
| **Adapters** | Implementations of the ports for one technology: SSH, USB/MaskROM (`rockusb`), QEMU/QMP, filesystem, network, Varlink bus | core, domain model, the technology's crates | `opendap-host-transports-ssh`, `opendap-host-transports-emulator` |
| **Interfaces** | Surfaces for people and programs: CLI, desktop UI, device services, MCP server | everything above | `opendap-host-cli` (the `opendap` binary), OpenDAP Desktop, device services |

Shared conventions of one interface kind (for example the CLI's output modes and error envelope) live in their own crate in the interface layer (`opendap-host-command`), not in the core.

## 3. Domain model

- One domain model per bounded context (host tooling, device system, audio engine, library), shared by every surface of that context. The CLI and OpenDAP Desktop use the same model and the same core; neither re-implements a use case.
- Types that cross the device–host boundary come from the bus schemas (system SDK, generated from the Rust types), so both sides compile against one definition.
- Errors are data: stable codes, typed fields, no stringly-typed errors in libraries.

## 4. Portability

- Domain and core crates are platform-agnostic: no POSIX-only or macOS-only assumptions, no hard-coded paths, no shelling out. Platform specifics live in adapters, selected by target or Cargo feature.
- This keeps Windows support possible for OpenDAP Desktop (DESK-005) and lets device and host share crates.
- `no_std` where it costs little (domain types used by firmware).
- Async: core APIs do not tie callers to one runtime unless the crate's purpose requires it.

## 5. Conventions

- Edition 2024; one Cargo workspace per repository; shared `[workspace.package]` metadata.
- Naming: `<root>-<path joined by ->` ([RFC 0017](https://github.com/opendap-project/rfcs/blob/main/text/0017-naming-and-topology.md) §3), applied to each root at its cut: the host crates are `opendap-host-*` (`opendap-host-model`, `opendap-host-core`, `opendap-host-transports-ssh`); crates of roots not yet cut keep `opendap-<area>[-<part>]`.
- Errors: `thiserror` in libraries, `anyhow` only in binaries.
- Features: minimal defaults; optional adapters behind features.
- No `unsafe` without a comment that justifies it and a test that exercises it.
- Size: a file stays within 600 lines of code (tests excluded), a function within 80 lines and a struct within 15 fields; going over one is a split, not a style note.
- Semantic versioning for every library; breaking changes are listed in the changelog.
- **Licences:** libraries others build on are MPL-2.0; binaries and services are GPL-3.0-or-later; firmware and templates MIT (workspace licensing rule).
- Tests: unit tests in the domain and core against fake adapters; contract tests that every adapter of a port passes; end-to-end tests against the virtual T1.

## 6. Reuse across repositories

- Host-side crates (`opendap-host-model`, `opendap-host-core`, the adapters) live in the [`host`](https://github.com/opendap-project/host) root, in `crates/` (`crates/model`, `crates/core`, `crates/platform`, `crates/transports/*`, `crates/command`), with the `opendap` binary in `cli/`; OpenDAP Desktop and the emulator's viewer consume them by version (the viewer by git revision).
- Device-side crates live in system and are the SDK.
- A crate needed on both sides moves to the side that owns its domain and is consumed by version from the other.
