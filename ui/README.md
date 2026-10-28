# Blackbird UI

Blackbird UI, the user interface of Blackbird OS: **Engineered Matter**, the OpenDAP design system, with its tokens, OpenDAP Icons, guidelines and components ([spec](docs/specs/design-system.md)), the templates the host renders, and the dynamic components contributed through the registry (including those from HATs).

## Layout

- `tokens/`: the token file and `generate.py`
- `export/`: what other repositories read, and only this: `slint/tokens.slint`, `css/tokens.css`, `icons/` (generated); `slint/components.slint` (the index of the components, one file per family in `slint/components/`, [listed here](components/README.md)) and `slint/blueprint.slint` (their blueprint mode), written by hand ([ADR 0002](docs/adr/0002-slint-components-with-a-blueprint-mode.md)); `fonts/` (IBM Plex, vendored, OFL-1.1)
- `icons/`
- `components/`
- `docs/`

## License

This repository follows the [REUSE](https://reuse.software) layout: license texts live in `LICENSES/` and `REUSE.toml` maps each path to its license.

- Default: **Mozilla Public License 2.0** (`MPL-2.0`)
- Markdown files: **Creative Commons Attribution-ShareAlike 4.0** (`CC-BY-SA-4.0`)
- `docs/`: **Creative Commons Attribution-ShareAlike 4.0** (`CC-BY-SA-4.0`)
- `brand/` and `export/brand/` (the OpenDAP mark, wordmark, avatar, app icon and the Blackbird OS lockups): **Creative Commons Attribution-ShareAlike 4.0** (`CC-BY-SA-4.0`); their generators `brand/*.py`: **GNU General Public License v3.0 or later** (`GPL-3.0-or-later`)
- `export/fonts/`: IBM Plex under the **SIL Open Font License 1.1** (`OFL-1.1`), © IBM Corp.

Copyright © 2026 The OpenDAP Project.
