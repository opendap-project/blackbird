# Engineered Matter — Design System

Engineered Matter is the name of OpenDAP's design system (maintainers, 2026-10-23).

- **Status:** Draft. This is the maintainers' design direction (2026-10-22). Values are starting points: colours, sizes and timings are to be validated on the first prototype's display and hardware.
- **Date:** 2026-10-22; revised 2026-10-23 (type scale for a 35 cm reading distance, margins and proximity, controls, icons, long text and accessibility)
- **Scope:** visual language, interaction patterns and design tokens for the Blackbird OS interface, OpenDAP Desktop, documentation and product renders. The physical product's materials and geometry are specified in the [case industrial-design spec](https://github.com/opendap-project/case/blob/main/docs/specs/industrial-design.md); the body buttons in the [controls and status LED spec](https://github.com/opendap-project/t1/blob/main/docs/specs/controls-and-status-led.md).
- **Tokens:** [`tokens/opendap.tokens.json`](../../tokens/opendap.tokens.json) is the canonical source of every value; the generated [token reference](../reference/tokens.md) lists them.
- **Requirements:** DS-001, DS-006 to DS-020 in this repository; SYS-061 to SYS-066 (accessibility) and SYS-009, SYS-021 to SYS-030, SYS-033 to SYS-037 in [`system`](../../../system/docs/requirements/README.md); T1-014, T1-015, T1-022, T1-056, T1-057, T1-059 in [`t1`](https://github.com/opendap-project/t1/blob/main/docs/requirements/README.md).

**Design principle: engineering excellence, made approachable.**

The OpenDAP T1 is a Digital Audio Player first. Every rule here serves that: playback, volume and the current track come before anything else on screen, and nothing visual may cost a frame of the interface or a sample of the audio.

## Contents

The design system is a set of topic specs. Section numbers are those of the whole, so a reference such as "design system §8.4" names a section in the topic listed here.

| § | Topic | Where |
|---|---|---|
| 1 | Philosophy and principles | [Principles and identity](design-system/principles.md#1-philosophy-and-principles) |
| 2 | Identity: attributes, what the design is not, mark and logo | [Principles and identity](design-system/principles.md#2-identity) |
| 3 | Colour | [Colour](design-system/colour.md) |
| 4 | Plasma gradients | [Effects](design-system/effects.md#4-plasma-gradients) |
| 5 | Thermal refraction | [Effects](design-system/effects.md#5-thermal-refraction) |
| 6 | Materials and geometry, including the geometry families shared with the interface | [Case industrial-design spec](https://github.com/opendap-project/case/blob/main/docs/specs/industrial-design.md) §1 and §2 |
| 7 | Physical controls | [Controls and status LED spec](https://github.com/opendap-project/t1/blob/main/docs/specs/controls-and-status-led.md) §1.4 |
| 8 | Interaction: a dedicated device, hierarchy, touch targets, feedback, gestures | [Interaction](design-system/interaction.md) |
| 9 | Typography | [Typography](design-system/typography.md) |
| 10 | Spacing, radii, borders and elevation | [Layout](design-system/layout.md) |
| 11 | Motion | [Motion](design-system/motion.md) |
| 12 | Lighting and effects, the effects budget and reduced-effects mode | [Effects](design-system/effects.md#12-lighting-and-effects) |
| 13 | Accessibility | [Accessibility](design-system/accessibility.md) |
| 14 | Design tokens | [Design tokens](design-system/tokens.md); values in the generated [token reference](../reference/tokens.md) |
| 15 | Components, button groups, choosing the control, icons, splash screen | [Components](design-system/components.md) |
| 16 | Product renders | [Principles and identity](design-system/principles.md#16-product-renders) |
| 17 | Performance | [Performance and platform](design-system/platform.md#17-performance) |
| 18 | Alignment with OpenDAP requirements: rendering, displays, fluidity, the status LED, languages, themes | [Performance and platform](design-system/platform.md#18-alignment-with-opendap-requirements) |
| 19 | Review checklist | [Design review checklist](../guides/developers/review-checklist.md) |
| 20 | Decisions and open questions | [Below](#20-decisions-and-open-questions) |

## 20. Decisions and open questions

Decided on 2026-10-22:

- **Processing colour.** Updates and other long system operations use `info` blue on screen, matching the status LED's blue breathing (T1 controls spec). Violet `processing` is reserved for computational visualisations (for example DSP or analysis views), never for system state.
- **Greys.** The token list is canonical: `text-muted` `#939BA3` for muted text (passes AA) and `border` `#343A40` for borders. `graphite.500` `#737C85` is kept only for icons and large text, and `graphite.700` `#3A4148` for separators.
- **Button groups.** Divider 1 px and gap 2 px.

Decided on 2026-10-23:

- **Type scale.** Seven sizes from 12 to 32 px with their line heights (§9.3); 12 px is the minimum (DS-015; maintainers, 2026-10-23, resolving an earlier 11 px here).
- **Target sizes.** 9.8 mm (64 px) for the screen's primary action, 7.9 mm (52 px) for frequent actions, 7.4 mm (48 px) for any other target, 1.2 mm (8 px) between hit areas (§8.3).
- **Gestures.** Back from the leading edge, the companion screen from the trailing edge, more from the bottom, notifications from the status bar (§8.5).
- **Fonts.** IBM Plex Sans SC for Chinese, shipped complete; Western digits in Plex Mono for technical values in every locale (§9.4).
- **Scale factor.** One design pixel is one panel pixel on the T1 reference panel; other displays scale by size and distance (§18.2).
- **Orange validation method.** The physical Action part matches `orange.500` when its colour, measured with a spectrophotometer under D65, is within ΔE00 2.0 of `#FF6A2A` in CIELAB; the same check applies to every edition.

Pending, because they need the prototype:

- **The orange measurement** on the first Action parts, with the method above.
- **The type scale and target sizes on the real panel:** confirm or adjust after reading and using them on the prototype display; the values above are what the software is built with until then.
- **Glyph positions and line layout** through parley, on the device.
- **Effects on Slint** (§18.1): which of the circular-gradient, pre-rendered or GPU-texture routes meets the frame and power budgets, measured on the T1.
