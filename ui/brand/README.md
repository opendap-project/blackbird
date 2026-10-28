# Brand mark and logo

The OpenDAP mark is two identical triangular pieces: the **O** pointing left (◁) and the **D** pointing right (▷), their stems facing each other, the O lower and the D higher. The wordmark **OpenDAP** is the logo for horizontal spaces; the vertical lockup pairs the two.

Use the files in [`svg/`](svg/); never redraw them. See the [design system](../docs/specs/design-system/principles.md#2-identity), §2.

## Files

Every SVG in `svg/` is `opendap-<form>-<colour>.svg`.

| Form | Use |
|---|---|
| `mark` | The symbol alone: favicons, app and device icons, small spaces, and wherever the name is already present |
| `wordmark` | The name alone: the logo for horizontal spaces (headers, banners, documents). The mark never sits beside it |
| `vertical` | Mark above the wordmark, for square or tall spaces |

| Colour | Fill | Use |
|---|---|---|
| `light` | `#F5F6F4` (`color.white`) | On graphite and other dark surfaces: the default |
| `dark` | `#111315` (`color.graphite.950`) | On light surfaces, and on an orange fill |
| `accent` | `#FF6A2A` (`color.orange.500`) | The mark alone, on graphite only |
| `current` | `currentColor` | Inline in a UI or web page, taking the text colour; as an `<img>` it renders black |

The fills are the design tokens' ([`tokens/opendap.tokens.json`](../tokens/opendap.tokens.json)), which the generators read; they are not written in the generators.

The SVGs are flat, so they scale and convert without distortion:

- One filled `<path>` per file, in absolute coordinates; no transforms, groups, strokes, text, styles, masks or metadata.
- Corners are cubic Béziers, never arcs, so tools with poor arc support (cutters, engravers, PCB and font tools) read them correctly.
- Outer contours run clockwise and counters counter-clockwise: the shape is the same under either fill rule.
- The `viewBox` is fitted to the ink, with its origin at 0,0: the file's box is the logo's box, and clear space is added around it, never inside it.
- The wordmark is converted to outlines: no font is needed to display it.

`opendap-mark-construction.svg` is the construction drawing: a document, not a logo asset.

`social/opendap-avatar.png` (1024 × 1024) is the profile picture for the GitHub organisation and other accounts: the light vertical lockup at 56 % of the width, centred, on the graphite studio backdrop of the T1 concept art (a radial gradient from `#1D2125` to `#0D0F11` with a soft cool light pool). The lockup stays inside the circle that platforms crop to. `social/opendap-avatar.svg` is its source; render it with `rsvg-convert -w 1024 -h 1024`.

## App icon

`app_icon.py` writes the app icon of OpenDAP's own host applications (the virtual devices' window, OpenDAP Desktop) to [`../export/brand/`](../export/brand/): `opendap-app-icon.png` (1024 × 1024) and `opendap-app.icns` (macOS, through `iconutil`).

- **Mark:** the light mark at 56 % of the body's width, centred on its ink. It is rasterised from `mark.py`'s own contours, so no SVG renderer is needed.
- **Backdrop:** the avatar's graphite studio backdrop, whose colours are brand data in [`backdrop.toml`](backdrop.toml).
- **Shape:** Apple's icon grid: an 824 px squircle in a 1024 px canvas, with a soft shadow below it.

Run `python3 brand/app_icon.py export/brand` after changing the mark.

## Construction on a 1:1 grid

Grid unit u; major grid every 7 u. The stroke (28 u) is the module: every measure is a grid integer or a simple ratio of it.

| Element | Value | Why |
|---|---|---|
| Piece | A triangle with a vertical stem 192 u long and its apex 168 u away | The D; the O is the same piece turned 180° |
| Angles | 0°, 90° and the sides at 4 u down per 7 u across (29.74°) | Industrial angles on the grid: every vertex falls on it |
| Strokes | Stem 28 u; sloped sides 32 u measured vertically (8/7 of the stroke, 27.8 u perpendicular) | Optically equal strokes, with the inner contour on the grid |
| Counter | The triangle parallel to the outline, vertices (28, 48), (112, 96), (28, 144) | Its three corners are close to 60°, so it reads as a clean triangle |
| Chamfers | One per corner, symmetric, its face perpendicular to the corner's bisector: 16 u outside, 8 u in the counter. The apex face is vertical, 14 u in from the tip | Relative to the triangle, not to the page; one chamfer family in a 2 : 1 ratio |
| Corner rounding | Every corner by a circular arc of 2 u (1/14 of the stroke) | The same small curve at every corner |
| Layout | The stems 14 u apart (half a stroke); the O 14 u lower and the D 14 u higher than centred, one stroke apart | Half-stroke spacing |
| Overall | Ink 322 u × 203.6 u; 322 u × 220 u between the apex faces and the triangles' stem corners | Each triangle 168 u × 192 u |

The triangles' vertices, the counters' vertices and the apex faces fall on the grid; the other chamfer points are set by the face lengths.

## Wordmark and lockup

| Element | Value |
|---|---|
| Wordmark | **OpenDAP**, written exactly so (capital O, D, A, P), in IBM Plex Sans SemiBold, the interface typeface (design system §9), with its own kerning and no added tracking |
| Vertical lockup | Cap height 70 u (2.5 strokes); the wordmark centred 42 u (1.5 strokes) under the mark |
| Horizontal | No lockup: in horizontal spaces the wordmark stands alone |

In the vertical lockup, the mark and the wordmark keep these proportions: never resize or move one against the other, and never set the mark beside the wordmark.

## Clear space

Nothing else (text, images, edges, other logos) enters the clear space around the ink:

- **Mark and vertical lockup:** three strokes on every side, 84 u at the files' scale, about 0.4 of the mark's height.
- **Wordmark:** its cap height (the height of the D) on every side.

## Minimum size

Checked in Chrome at 1× pixel density (2026-10-22):

| Form | Screen (height, 1×) | Print (height) |
|---|---|---|
| `mark` | 16 px; 24 px recommended | 5 mm |
| `vertical` | 48 px | 15 mm |
| `wordmark` | 16 px | 4 mm |

The print values keep the wordmark's cap height at 3 mm or more. At 16 px the gap between the pieces is about one pixel: below it, the pieces merge. In any physical process (print, engraving, silkscreen, moulding), the gap, 6.9 % of the mark's height, must stay at or above the process's minimum feature, and the counters must stay open. The print values assume a 0.3 mm minimum feature and are **UNVERIFIED** until a first print run.

## Colour and backgrounds

| Logo | Allowed backgrounds | Contrast (WCAG 2) |
|---|---|---|
| `light` | `graphite.950` to `graphite.800`; dark photos or plasma areas at least as dark | 17.2 to 12.6 on the graphite surfaces |
| `dark` | `color.white` and light surfaces; the orange fill `orange.500` | 17.2 on white; 6.5 on orange |
| `accent` (mark only) | `graphite.950` to `graphite.850` | 6.5 to 5.5 |

- At least 3 : 1 against the background everywhere the logo sits (WCAG 1.4.11, graphical objects).
- Orange on a light surface is not allowed (2.6 : 1), and the wordmark is never orange.
- One colour per logo: the mark and the wordmark always share it.
- On plasma (design system §4), only over an area that keeps 3 : 1, never over the bright core.

## Never

- Stretch, skew, rotate or mirror the logo. Mirroring swaps the O and the D.
- Move, resize, separate or realign the pieces, or change the gap or the offset between them.
- Add an outline, a stroke, a shadow, a glow, a gradient or plasma inside the logo.
- Recolour it outside the four colours above, or use two colours in one logo.
- Retype the wordmark, set it in another typeface or weight, or use live text in place of the outlines.
- Place it in a container or next to anything that enters its clear space.

## Product lockups

OpenDAP software products have a lockup with the OpenDAP wordmark above the product name, generated by `products.py` in `products/<slug>/`. The first is **Blackbird OS**: [`products/blackbird-os/`](products/blackbird-os/). Releases carry no codename ([blackbird-os ADR 0011](https://github.com/opendap-project/blackbird-os/blob/main/docs/adr/0011-blackbird-os.md)).

| File | Use |
|---|---|
| `<slug>-stacked-<colour>.svg` | "OpenDAP" above the product name, no mark: release notes, documents, headers |
| `<slug>-vertical-<colour>.svg` | The mark above both lines: square or tall spaces |
| `<slug>-banner.svg` | 1280 × 640 px, the vertical lockup on graphite: release pages and repository previews |

- **Type:** "OpenDAP" in SemiBold, as the wordmark; the product name in IBM Plex Sans Regular at half the wordmark's cap height, its cap top three quarters of a cap height under the wordmark's baseline. Both lockups are centred.
- **Colour:** `light`, `dark` or `current`, one colour for the whole lockup, as the logo. Never orange.
- **Clear space:** as the wordmark (stacked) or the vertical lockup (vertical).
- **Minimum size (screen, 1×):** stacked 48 px, vertical 128 px, as checked for the earlier release lockups (Chrome, 2026-10-22); the longer name "Blackbird OS" is **UNVERIFIED** at those sizes.
- The product name never stands alone with the mark, and never replaces the wordmark.

## Generators

The OpenDAP project's marks and the product lockups have separate generators; both read their colours from the design tokens through `colours.py`.

- `mark.py`: the mark from the construction above. `mark.py OUT.svg [--fill …]` writes one mark; `--grid` writes the construction drawing.
- `logo.py`: the OpenDAP marks, every file in `svg/` and the construction drawing.
- `products.py`: the product lockups and banners in `products/<slug>/`. A new product is one more name in `PRODUCTS`.
- `outline.py`: what the two share: text to outlines and the flat SVG file. `colours.py`: the logo colours from the tokens and the backdrop from `backdrop.toml`.

`logo.py` and `products.py` need fontTools, uharfbuzz and IBM Plex Sans 1.1.0 SemiBold and Regular (SIL OFL 1.1) from [IBM/plex](https://github.com/IBM/plex) (`ibm-plex-sans`, `fonts/complete/otf`); the fonts are not stored here. `--check` fails when a committed file differs from what would be written:

```sh
python3 brand/logo.py --plex path/to/ibm-plex-sans/fonts/complete/otf [--check]
python3 brand/products.py --plex path/to/ibm-plex-sans/fonts/complete/otf [--check]
```

`test_goldens.py` checks that every generated file is byte-identical to the committed one: the marks, the construction drawing and the app icon always; the wordmark, the lockups and the product lockups when `ODAP_PLEX_SANS_OTF` names the Plex folder. Pre-commit runs it on every change to `brand/`, `export/brand/` or the token file.

```sh
ODAP_PLEX_SANS_OTF=path/to/ibm-plex-sans/fonts/complete/otf python3 -m unittest brand/test_goldens.py
```

Status: approved by the maintainers on 2026-10-22. The print minimum sizes stay **UNVERIFIED** until a first print run.
