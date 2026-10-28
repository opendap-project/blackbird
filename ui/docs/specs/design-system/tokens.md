# Engineered Matter — Design tokens

Part of the [Engineered Matter design system](../design-system.md): its status, scope and requirements apply here. Section numbers are the design system's, so a reference such as "design system §14" leads here.

## 14. Design tokens

The tokens live in one platform-neutral file, [`tokens/opendap.tokens.json`](../../../tokens/opendap.tokens.json), in the W3C Design Tokens Community Group format ([2025.10](https://www.designtokens.org/tr/2025.10/format/)). Slint globals for the device and CSS custom properties for web, documentation and OpenDAP Desktop are generated from it; neither is edited by hand (§18.1).

Groups: `color` (primitives), `semantic` (surfaces, text, brand, state, border, focus), `size` (strokes such as the button-group divider, target and bar sizes, the status bar's icon size `size-icon-status`), `opacity` (`opacity-disabled`, the Disabled state), `plasma` (spectrum and reference gradients, kept apart so it can be disabled as one group), `font`, `space`, `radius`, `motion`, `effect`.

The generated [token reference](../../reference/tokens.md) lists every token with its value, its CSS custom property and its Slint property. Names are the token path joined with `-`, without the `semantic` prefix: `semantic.text.primary` is `--text-primary` in CSS and `Tokens.text-primary` in Slint.

No colour, radius or timing is written outside the tokens without a recorded reason.
