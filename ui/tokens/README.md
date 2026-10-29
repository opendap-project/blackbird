# Tokens

Design tokens of the official design system: every colour, font, spacing, radius, timing and effect value the UI uses. The rules for using them are in the [design system spec](../docs/specs/design-system.md).

## Files

| File | Contents |
|---|---|
| [`opendap.tokens.json`](opendap.tokens.json) | The **official theme**, in the W3C Design Tokens Community Group format ([2025.10](https://www.designtokens.org/tr/2025.10/format/)). Status: Draft; values are starting points to validate on the prototype display. |

## Structure

| Group | Contents |
|---|---|
| `color` | Primitive palette: graphite, white, orange. Components do not use it directly. |
| `semantic` | Roles: background, surfaces, text, brand, state (success, warning, error, info, processing), borders, focus. Most are aliases of primitives. |
| `font` | Interface and technical families (IBM Plex with its script companions) and weights. |
| `space`, `size`, `radius` | Spacing scale (4 px base), strokes, targets, bar heights and the status bar's icon size, digital radii including the button-group outer radius. |
| `opacity` | The Disabled state's opacity (`opacity-disabled`). |
| `motion` | Durations and easing. |
| `effect` | Shadow and brand glow. |
| `plasma` | The plasma and thermal special mode, kept as one group so the reduced-effects mode can turn it off as a whole. |

Units: on the T1, `px` is one panel pixel (about 0.153 mm at 166 ppi). Values the format cannot express (elliptical radial gradients, layer blur and inset) are in `$extensions` under `cc.opendap.blackbird.*`.

## Generated outputs

The JSON file is the single source. [`generate.py`](generate.py) (Python 3, no dependencies) writes three outputs from it, which are never edited by hand:

- **Slint**: `export/slint/tokens.slint`, the global `Tokens`, for Blackbird OS on the device ([system ADR 0006](../../system/docs/adr/0006-slint-as-host-renderer.md)). Use it as `Tokens.text-primary`, `Tokens.space-margin`, `Tokens.font-size-body`.
- **CSS**: `export/css/tokens.css`, custom properties for the web, documentation and OpenDAP Desktop (`--text-primary`, `--space-margin`, `--radius-control`…).
- **Reference**: [`docs/reference/tokens.md`](../docs/reference/tokens.md), every token with its value, its CSS and Slint names and its description. The design system links it instead of keeping its own tables of values.

Names are the token path joined with `-`. The `semantic` prefix is dropped, so `semantic.text.primary` becomes `text-primary`. Aliases are resolved.

Slint has no type for shadows, gradients with stops or easing curves. Those tokens are listed in the generated file's header and drawn by components (design system §18.1).

```sh
python3 tokens/generate.py                      # write the three outputs
python3 tokens/generate.py --check              # fail if they are out of date
python3 -m unittest tokens/test_generate.py     # tests of the reference output
```

Pre-commit runs the check and the tests on every commit that touches `tokens/` or the outputs.

Other repositories read `export/`, never the token file (`system` finds it through `ODAP_BLACKBIRD_DIR` or the workspace's `repos.tsv`).

## Themes

There is one official theme (DS-007, SYS-030). A community theme is another token file with the same structure that replaces values, without code. A theme may change values, not the meaning of semantic colours, the contrast minimums, the target sizes or the reduced-motion and reduced-effects behaviour.
