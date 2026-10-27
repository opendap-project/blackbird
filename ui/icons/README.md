# OpenDAP Icons

The icon library of the Engineered Matter design system (DS-021, [ADR 0001](../docs/adr/0001-tabler-icons-as-the-base-family.md)): semantic names over **[Tabler Icons](https://github.com/tabler/tabler-icons)** (MIT). [`icons.py`](icons.py) writes the SVGs, the catalogue and the [preview](../renders/web/icons.html).

## Sources

- **Tabler Icons, as upstream draws them.**
  - Pinned in [`vendor/tabler-icons/VERSION`](vendor/tabler-icons/VERSION) (`@tabler/icons` 3.49.0, with the tarball's integrity).
  - Only the glyphs in use are vendored, unmodified, with upstream's `LICENSE`.
  - [`map.toml`](map.toml) maps each name to its glyph (`[icons]` for the interface, `[status]` for the status bar).
- **OpenDAP's own glyphs, only in extreme cases:** where Tabler has none and the meaning matters. Today there are two, `output-jack` (3.5 mm) and `output-balanced` (4.4 mm), drawn in Tabler's style.
- **Updating Tabler:**
  1. change the pin;
  2. copy the mapped glyphs again;
  3. run `icons.py`;
  4. review the preview.
- **Generated files:** `icons.py` writes the SVGs in `export/icons/`, `icons.json` and the preview; nobody edits them by hand. `python3 icons.py --check` fails when any of them is missing, different or left over, and pre-commit runs it on every commit that touches them or `icons/`.

## Sizes

- **Interface:** 24 px, with Tabler's 2 px stroke.
- **Status bar:** 16 px, with the same glyphs at a 1.5 stroke, which is 1 px. The battery shows its level with Tabler's glyphs: `status-battery-0` to `status-battery-4`.
- **On the pixel grid:** at 16 px a unit of the 24 grid is 2/3 px, so strokes on whole units fall between pixels. Each status glyph is moved as a whole, never redrawn, by the translation that puts the most straight stroke length on pixel centres, at most half a pixel ([`pixelgrid.py`](pixelgrid.py), in the generated file's `transform`). `python3 pixelgrid.py` lists each glyph's move; `--raster` prints each exported glyph's ink per column and row (librsvg's `rsvg-convert`). Diagonals and curves stay as drawn, such as the charging bolt's slanted strokes; its vertical stroke is moved onto a pixel centre.
- **Filled glyphs** (Tabler's filled set) are for learned, stateful controls: transport and Liked.

## Names

Names are **semantic**: what the icon means (`signal-path`, `output-balanced`, `waiting`), not what it shows. Code, templates and capabilities refer to icons only by name (`icon: "like"`), never by file. That is what makes replacement possible.

The catalogue [`icons.json`](icons.json) lists each icon's name, grid, category, style and default accessible label. The label is a fallback: the control's own accessible name always says what it does.

## Customising

- **Themes** (SYS-030) may redraw any catalogued icon. A theme ships `icons/<grid>/<name>.svg` for the names it changes; every other name falls back to these. Replacements are checked with `python3 icons.py --check <theme>/icons`:
  - known names only;
  - the right canvas;
  - `currentColor` only, so the theme's tokens colour them.
- **Derivatives** (OS-028) replace the set the same way, in their image.
- **Capabilities** use catalogue names only (template tier A, system ADR 0002). A glyph the catalogue lacks is contributed here first (system ADR 0004), so every surface and theme gets it.
- **New icons** are a Tabler glyph added to `map.toml` (and copied into `vendor/`), regenerated, and checked in the preview before they are merged. A drawing of OpenDAP's own is the exception.

## Licence

- **Tabler's glyphs**, the vendored originals and the files in `export/icons/` built from them: MIT, © 2020-2026 Paweł Kuna. Each generated file says so in its header.
- **OpenDAP's own glyphs, the build and the catalogue:** MPL-2.0, like the rest of the design system's assets (REUSE).
