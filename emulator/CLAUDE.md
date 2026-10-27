# Virtual devices

Virtual OpenDAP devices for development on the host: hardware layers, device models composed from them, the QEMU runner and the virtual board's support package for Blackbird OS images.

<!-- >>> odapkit sync: managed block from the workspace templates; edit them, not this block -->
Part of [OpenDAP](https://github.com/opendap-project). This repository is Open Source.

Contributor rules (English, confirmed information only, REUSE licensing, the `docs/` layout and its skills): the [contributor guide](https://github.com/opendap-project/.github/blob/main/CONTRIBUTING.md).
<!-- <<< odapkit sync -->

## Working here

- Requirements are edited in `docs/requirements/requirements.toml`; `docs/requirements/README.md` is rendered from it with `odapkit req render` and never edited by hand.
- A model is a combination of hardware layers; a layer never names a model. Other repositories use this one through `bin/odap-vdev` (its commands and `status --json`) and `export/` (the board support package), never its internals.
