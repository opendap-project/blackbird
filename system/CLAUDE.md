# System

Blackbird OS system software in Rust: bus, orchestrator, UI runtime, SDK and base services.

## Working here

- Rust style is the standard rustfmt style; clippy runs with `-D warnings` (pre-push hook and `check.toml`). Build with `CARGO_BUILD_JOBS=4` on the shared machine.
- Requirements are edited in `docs/requirements/requirements.toml`; `docs/requirements/README.md` is rendered from it with `odapkit req render` and never edited by hand.
