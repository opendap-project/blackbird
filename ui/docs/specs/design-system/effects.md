# Engineered Matter — Plasma, thermal and lighting effects

Part of the [Engineered Matter design system](../design-system.md): its status, scope and requirements apply here. Section numbers are the design system's, so a reference such as "design system §4" leads here.

## 4. Plasma gradients

Plasma is OpenDAP's occasional visual signature: concentrated energy and heat, without depicting a literal source in every component. It is a **special visual mode, never the default surface treatment** (DS-009).

### 4.1 Character

- A concentrated white or near-white core.
- A transition through warm orange and amber.
- Localised violet or magenta transitions; occasional cyan dispersal at the outer edge.
- Soft volumetric halos with organic, asymmetric contours, not perfect concentric rings.
- Strong contrast against graphite, and a sense of energy moving through space.

The result is luminous, layered and physically plausible, not a multicolour wallpaper. Violet and cyan are secondary transitions, never competing brand colours.

### 4.2 Reference recipe

Tokens: `plasma.*` in the token file. The CSS below is a reference recipe for web, documentation and OpenDAP Desktop; the device renders with Slint (§18.1).

```css
.plasma-gradient {
  background: radial-gradient(
    ellipse at 46% 54%,
    rgba(255, 253, 249, 0.98) 0%,
    rgba(255, 177, 92, 0.92) 7%,
    rgba(255, 106, 42, 0.82) 19%,
    rgba(220, 107, 255, 0.43) 39%,
    rgba(119, 92, 255, 0.23) 54%,
    rgba(91, 220, 255, 0.12) 68%,
    transparent 78%
  );
}
```

Glow is built from several layers rather than one large blur:

```css
.plasma-glow {
  position: relative;
  isolation: isolate;
  background: var(--bg);
}
.plasma-glow::before {
  content: "";
  position: absolute;
  inset: 8%;
  z-index: -1;
  border-radius: 50%;
  background: radial-gradient(
    ellipse at 48% 50%,
    rgba(255, 247, 237, 0.95) 0%,
    rgba(255, 106, 42, 0.72) 15%,
    rgba(220, 107, 255, 0.28) 38%,
    rgba(91, 220, 255, 0.12) 58%,
    transparent 75%
  );
  filter: blur(22px);
  opacity: 0.82;
  pointer-events: none;
}
```

These are recipes, not mandatory gradients: tune the balance, blur and opacity for the surface, the display and the intended intensity.

### 4.3 Where plasma belongs

**Use for:** a dedicated visualiser or now-playing background; boot and selected transition moments; audio-reactive visualisation; processing indicators where a luminous metaphor fits; a restrained signature in documentation and renders; selected status and experimental screens.

**Never use for:** buttons in general; text backgrounds; routine lists and settings screens; warnings that already have a semantic colour; small icons that a glow would make illegible.

## 5. Thermal refraction

Thermal refraction is the distortion heat produces around an energetic source: subtle optical deformation, not smoke, fire or a plain blur. It stays subordinate to content: nobody ever has to read a distorted label or icon.

### 5.1 Character

Slightly displaced contours; layered translucent waves; low-amplitude irregular distortion; softly shifting gradients; localised luminance changes; organic asymmetry; minimal interference with text and controls.

### 5.2 Layers

1. **Source:** a localised plasma or warm light source.
2. **Refraction:** subtle wave-like distortion of the source or background.
3. **Atmosphere:** a very low-opacity colour haze that gives depth.

The layers are independent, so each can be reduced, disabled or replaced without changing the layout.

### 5.3 Reference recipe (atmosphere only)

```css
.thermal-field {
  position: relative;
  isolation: isolate;
  overflow: hidden;
  background: var(--bg);
}
.thermal-field::before {
  content: "";
  position: absolute;
  inset: -12%;
  z-index: -1;
  background:
    radial-gradient(ellipse at 35% 55%, rgba(255, 106, 42, 0.19), transparent 43%),
    radial-gradient(ellipse at 67% 38%, rgba(119, 92, 255, 0.13), transparent 48%),
    radial-gradient(ellipse at 60% 72%, rgba(91, 220, 255, 0.08), transparent 42%);
  filter: blur(24px);
  pointer-events: none;
}
```

This is the atmospheric field, not optical refraction. Real displacement needs a tested displacement filter, a shader or a pre-rendered texture; `backdrop-filter: blur()` alone does not produce heat distortion.

### 5.4 Rules

- Prefer a static atmospheric field over continuous animation.
- Use real displacement only where the platform renders it within the frame budget (§17).
- Avoid large filter regions that force expensive repaints.
- Keep important text on a stable, undistorted layer.
- No continuous shader work on the battery-powered device unless it is measured within budget.
- Honour the reduced-effects mode (§12.4).
- Test contrast against every background the effect can produce.

## 12. Lighting and effects

### 12.1 Lighting model

Soft diffuse light; localised luminous sources; subtle edge highlights; controlled reflections; deep but readable shadows; occasional plasma bloom. Lighting supports material perception and hierarchy.

### 12.2 Glow

Only on elements that are actually luminous, or in selected energetic visualisations. Never on every icon, border, title or control.

### 12.3 Transparency

Translucency for light guides, overlays and selected atmospheric layers. Text and controls sit on surfaces opaque enough to stay legible.

### 12.4 Effects budget and reduced-effects mode

Every effect costs attention, rendering time and battery. Before adding one, answer:

1. What does it communicate?
2. Why is a static colour or shape not enough?
3. Can it be reduced or disabled?
4. Does it stay within the frame and power budgets on the target hardware (§17)?
5. Does it interfere with reading?

If any answer is unclear, remove the effect.

Blackbird OS has a **reduced-effects** mode that replaces plasma and thermal effects with their static or flat equivalents (DS-009). The interface is complete and correct with every effect off.
