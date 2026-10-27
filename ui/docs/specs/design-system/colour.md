# Engineered Matter — Colour

Part of the [Engineered Matter design system](../design-system.md): its status, scope and requirements apply here. Section numbers are the design system's, so a reference such as "design system §3" leads here.

## 3. Colour

The palette is graphite, white and one restrained orange accent. Plasma colours add a wider spectrum only where they serve a specific purpose (§4).

### 3.1 Core palette

The palette's tokens, `color.graphite.*` (950 to 500), `color.white.*` and `color.orange.*` (400, 500, 700), are listed with their values and purposes in the generated [token reference](../../reference/tokens.md#color).

The orange is to be validated against the physical accent part (the Action button, [controls and status LED spec](https://github.com/opendap-project/t1/blob/main/docs/specs/controls-and-status-led.md)) under the same light, and every pair is to be checked for contrast before production.

### 3.2 Semantic colours

Semantic colours communicate state, never decoration, and keep one meaning everywhere.

The state tokens, `semantic.state.error`, `warning`, `success`, `info` and `processing`, are listed with their values, meanings and status LED equivalents (IEC 60073; §18.4) in the [token reference](../../reference/tokens.md#semantic).

Neutral information uses the neutral text colours, as the status LED uses white.

### 3.3 Distribution

As a starting guideline, not a pixel quota: 75–90 % graphite surfaces; 5–15 % white and muted content; 2–5 % orange; plasma and semantic colours only in localised areas.

### 3.4 Measured contrast

WCAG 2 contrast ratios computed from the token values (sRGB relative luminance):

| Foreground | on `graphite.950` | on `graphite.900` | on `graphite.850` | on `graphite.800` |
|---|---|---|---|---|
| `text` `#F5F6F4` | 17.2 | 15.8 | 14.4 | 12.6 |
| `text-secondary` `#C6CBD0` | 11.4 | 10.5 | 9.6 | 8.4 |
| `text-muted` `#939BA3` | 6.6 | 6.1 | 5.6 | 4.9 |
| `graphite.500` `#737C85` | 4.4 | 4.0 | 3.7 | 3.2 |
| `brand` `#FF6A2A` | 6.5 | 6.0 | 5.5 | 4.8 |
| `error` `#FF5D65` | 6.2 | 5.7 | 5.2 | 4.6 |
| `border` `#343A40` | 1.6 | 1.5 | 1.4 | 1.2 |
| `border-strong` `#515A63` | 2.7 | 2.4 | 2.2 | 2.0 |

Consequences:

- `graphite.500` fails AA (4.5:1) for body text on every surface. Use it only for icons and large text (3:1), or use `text-muted`.
- On an orange fill, the foreground is dark: `graphite.950` on `brand` is 6.5:1; `white` on `brand` is 2.6:1 and is not allowed for text or icons.
- `border` and `border-strong` are below 3:1. They may separate surfaces, but a control whose boundary is needed to find it carries a 3:1 cue another way (fill contrast, label or icon), per WCAG 1.4.11.
