# ADR 0001: Tabler Icons as the base family of OpenDAP Icons

- **Status:** Accepted
- **Date:** 2026-10-23
- **Decided by:** the maintainers

## Context

- The maintainers asked for icons from an existing, complete family close to the look the design system wants (they named Lucide as an example). The family should cover the domain model's variety, with OpenDAP drawing only what is missing (DS-021).
- Families were compared on version, licence, grid, stroke, caps and joins, filled variants, maintenance and coverage of the catalogue's names. Tabler 3.49.0 (MIT) has filled variants for stateful controls and covers the catalogue with only a few names missing.

## Decision

- **Base family: [Tabler Icons](https://github.com/tabler/tabler-icons)** (MIT), used **as upstream draws it**: 2 px strokes, round caps and joins, 24 px grid, filled variants for stateful controls.
  - Pinned at `@tabler/icons` 3.49.0, with the npm tarball's integrity recorded in `icons/vendor/tabler-icons/VERSION`.
  - Only the glyphs in use are vendored, unmodified, with upstream's licence beside them.
- **OpenDAP Icons stay semantic names.** `icons/map.toml` maps each name to a Tabler glyph, including where Tabler's glyph is the closest rather than an exact match.
- **The status bar uses the same glyphs**, drawn with a 1.5 stroke so it is 1 px at 16 px. Tabler's battery glyphs (empty to four bars) show the level.
- **OpenDAP draws a glyph only in extreme cases**, where Tabler has none and the meaning matters, in Tabler's style. Today these are the two headphone outputs, 3.5 mm and 4.4 mm balanced, which a DAP must tell apart. They stay MPL-2.0.
- **Replacement is unchanged.** Themes and derivatives redraw any name, or supply another mapping to change the base family (DS-021, SYS-030, OS-028).

## Consequences

- DS-019 no longer asks for a set "matched to the typeface". Design system §15.3 and the icons README describe the Tabler base.
- Updating Tabler is deliberate:
  1. bump the pin;
  2. copy the mapped glyphs again;
  3. regenerate;
  4. review the preview.
- Rejected after the visual review on 2026-10-23:
  - `player-track-prev` and `player-track-next`: they are a rewind shape, so `player-skip-back` and `player-skip-forward` are used for previous and next;
  - `battery-exclamation`: it reads as a letter at small sizes.
