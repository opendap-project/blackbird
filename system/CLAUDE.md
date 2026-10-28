# System

Blackbird OS system software in Rust: bus, orchestrator, UI runtime, SDK and base services.

<!-- >>> odapkit sync: managed block from the workspace templates; edit them, not this block -->
Part of [OpenDAP](https://github.com/opendap-project). This repository is Open Source.

Contributor rules (English, confirmed information only, REUSE licensing, the `docs/` layout and its skills): the [contributor guide](https://github.com/opendap-project/.github/blob/main/CONTRIBUTING.md).
<!-- <<< odapkit sync -->

## Working here

- Rust style is the standard rustfmt style; clippy runs with `-D warnings` (pre-push hook and `check.toml`). Build with `CARGO_BUILD_JOBS=4` on the shared machine.
- Requirements are edited in `docs/requirements/requirements.toml`; `docs/requirements/README.md` is rendered from it with `odapkit req render` and never edited by hand.
