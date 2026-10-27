# Engineered Matter — Principles and identity

Part of the [Engineered Matter design system](../design-system.md): its status, scope and requirements apply here. Section numbers are the design system's, so a reference such as "design system §1" leads here.

## 1. Philosophy and principles

OpenDAP combines the precision of aerospace instrumentation with the ergonomics and approachability of contemporary consumer electronics. The product should feel like a carefully engineered instrument, not a gaming accessory or a generic smartphone.

The design language draws on:

- **Aerospace instrumentation:** functional surfaces, material discipline and purposeful indicators.
- **Material restraint:** restrained geometry, graphite surfaces, low-reflectivity finishes and subtle structural details.
- **Contemporary consumer electronics:** rounded corners, tactile controls, visual clarity and intuitive interactions.
- **Thermal and plasma physics:** controlled luminous gradients, heat distortion and energetic colour transitions, used as a distinctive and occasional visual signature.
- **Human-centred industrial design:** ergonomic controls, predictable interactions, accessible contrast and minimal cognitive load.

### Principles

1. **Function before decoration.** Every visible detail communicates function, improves interaction or reinforces product identity.
2. **Approachable engineering.** The device feels sophisticated without intimidating non-technical users.
3. **Material authenticity.** Surfaces suggest plausible manufactured materials, not artificial science-fiction textures.
4. **Controlled energy.** Plasma and thermal effects give identity; they are never continuous visual noise.
5. **Tactility matters.** Physical and digital controls communicate their purpose through shape, position, feedback and consistency.
6. **Quiet by default.** Most of the interface is neutral; colour and motion are reserved for meaningful states.
7. **Open by design.** The visual language never implies a sealed enclosure, needless complexity or inaccessible technology.

## 2. Identity

### 2.1 Attributes

| Attribute | Expression |
|---|---|
| Technical | Precise geometry, consistent alignment, functional details |
| Human | Rounded shapes, comfortable proportions, clear labels |
| Open | Transparent hierarchy, understandable controls, minimal ornament |
| Refined | Restrained colours, subtle textures, deliberate spacing |
| Energetic | Plasma gradients and thermal refraction in selected places |
| Durable | Matte finishes, robust shapes, realistic materials |

### 2.2 What the design is not

- Aggressive or hostile aesthetics, or overly sharp geometry.
- Cyberpunk or neon styling.
- Decorative circuit-board patterns.
- Exposed screws, vents or machined parts that have no function.
- Full-metal construction as the default visual assumption.
- Excessive glow, bloom, lens flares or animated backgrounds.
- Dense instrumentation where ordinary playback controls are enough.
- Smartphone interfaces copied without adapting them to a dedicated audio device.

The target: precision without aggression, sophistication without complexity, technology without intimidation.

### 2.3 Mark and logo

The mark is two identical triangular pieces: the **O** pointing left (◁) and the **D** pointing right (▷), their stems facing each other half a stroke apart, offset diagonally by one stroke. It is built on a 1:1 grid with industrial angles, a 28 u stroke as the module, one symmetric chamfer per corner (16 u outside, 8 u in the triangular counters) and a uniform 2 u corner radius. The wordmark is **OpenDAP** in IBM Plex Sans SemiBold (§9), converted to outlines.

- **Forms:** the mark alone; the wordmark alone, the logo for horizontal spaces (the mark never sits beside it); the vertical lockup, mark above wordmark, for square or tall spaces.
- **Colours:** light (`color.white`) on graphite, the default; dark (`color.graphite.950`) on light surfaces and on an orange fill; the orange accent for the mark alone, on graphite only; `currentColor` for inline use. One colour per logo, at least 3 : 1 against its background.
- **Clear space:** three strokes on every side of the mark or the vertical lockup (about 0.4 of the mark's height); the cap height around the wordmark.
- **Minimum size (screen, 1×):** mark 16 px (24 px recommended), wordmark 16 px, vertical lockup 48 px.
- **Never** stretch, rotate, mirror, recolour or rebuild it, or move its pieces.

The construction, the files (flat SVGs: one path, no transforms, strokes, arcs or text), the full usage rules and the generators are in [`brand/`](../../../brand/README.md). Status: approved by the maintainers on 2026-10-22.

## 16. Product renders

Renders, previews and concept art follow the same language as the interface. The rules for the physical product are in the [case industrial-design spec](https://github.com/opendap-project/case/blob/main/docs/specs/industrial-design.md) (§ Rendering rules); renders are built from components' `export/` and the [Blender library](https://github.com/opendap-project/blender-lib). In short: matte synthetic enclosure by default; soft, controlled studio light; orange only on the Action button and sparingly elsewhere; plasma confined to display content or a conceptual visualisation; accurate proportions; the hardware stays credible as a manufacturable object.
