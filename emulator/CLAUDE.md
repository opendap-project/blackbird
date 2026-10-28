# Virtual devices

Virtual OpenDAP devices for development on the host: hardware layers, device models composed from them, the QEMU runner and the virtual board's support package for Blackbird OS images.

## Working here

- Requirements are edited in `docs/requirements/requirements.toml`; `docs/requirements/README.md` is rendered from it with `odapkit req render` and never edited by hand.
- A model is a combination of hardware layers; a layer never names a model. Other repositories use this one through `bin/odap-vdev` (its commands and `status --json`) and `export/` (the board support package), never its internals.
