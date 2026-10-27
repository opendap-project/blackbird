# Engineered Matter — Design System

Engineered Matter is the name of OpenDAP's design system (maintainers, 2026-10-23).

- **Status:** Draft. This is the maintainers' design direction (2026-10-22). Values are starting points: colours, sizes and timings are to be validated on the first prototype's display and hardware.
- **Date:** 2026-10-22; revised 2026-10-23 (type scale for a 35 cm reading distance, margins and proximity, controls, icons, long text and accessibility)
- **Scope:** visual language, interaction patterns and design tokens for the Blackbird OS interface, OpenDAP Desktop, documentation and product renders. The physical product's materials and geometry are summarised here and specified in the [case industrial-design spec](https://github.com/opendap-project/case/blob/main/docs/specs/industrial-design.md); the body buttons in the [controls and status LED spec](https://github.com/opendap-project/t1/blob/main/docs/specs/controls-and-status-led.md).
- **Tokens:** [`tokens/opendap.tokens.json`](../../tokens/opendap.tokens.json) is the canonical source of every value below.
- **Requirements:** DS-001, DS-006 to DS-020 in this repository; SYS-061 to SYS-066 (accessibility) and SYS-009, SYS-021 to SYS-030, SYS-033 to SYS-037 in [`system`](https://github.com/opendap-project/system/blob/main/docs/requirements/README.md); T1-014, T1-015, T1-022, T1-056, T1-057, T1-059 in [`t1`](https://github.com/opendap-project/t1/blob/main/docs/requirements/README.md).

**Design principle: engineering excellence, made approachable.**

The OpenDAP T1 is a Digital Audio Player first. Every rule here serves that: playback, volume and the current track come before anything else on screen, and nothing visual may cost a frame of the interface or a sample of the audio.

## Contents

1. [Philosophy and principles](#1-philosophy-and-principles)
2. [Identity](#2-identity)
3. [Colour](#3-colour)
4. [Plasma gradients](#4-plasma-gradients)
5. [Thermal refraction](#5-thermal-refraction)
6. [Materials and geometry (summary)](#6-materials-and-geometry-summary)
7. [Physical controls (summary)](#7-physical-controls-summary)
8. [Interaction](#8-interaction)
9. [Typography](#9-typography)
10. [Spacing, radii, borders and elevation](#10-spacing-radii-borders-and-elevation)
11. [Motion](#11-motion)
12. [Lighting and effects](#12-lighting-and-effects)
13. [Accessibility](#13-accessibility)
14. [Design tokens](#14-design-tokens)
15. [Components](#15-components)
16. [Product renders](#16-product-renders)
17. [Performance](#17-performance)
18. [Alignment with OpenDAP requirements](#18-alignment-with-opendap-requirements)
19. [Review checklist](#19-review-checklist)
20. [Decisions and open questions](#20-decisions-and-open-questions)

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

The construction, the files (flat SVGs: one path, no transforms, strokes, arcs or text), the full usage rules and the generators are in [`brand/`](../../brand/README.md). Status: approved by the maintainers on 2026-10-22.

## 3. Colour

The palette is graphite, white and one restrained orange accent. Plasma colours add a wider spectrum only where they serve a specific purpose (§4).

### 3.1 Core palette

| Token | Value | Purpose |
|---|---|---|
| `color.graphite.950` | `#111315` | Primary dark background |
| `color.graphite.900` | `#191C1F` | Main surfaces |
| `color.graphite.850` | `#202428` | Raised panels |
| `color.graphite.800` | `#292E33` | Secondary controls |
| `color.graphite.700` | `#3A4148` | Separators |
| `color.graphite.500` | `#737C85` | Muted icons and large labels (see §3.4) |
| `color.white` | `#F5F6F4` | Primary text, high-contrast details |
| `color.white-muted` | `#C6CBD0` | Secondary text |
| `color.orange.500` | `#FF6A2A` | Brand accent |
| `color.orange.400` | `#FF874D` | Hover, active highlight, focus |
| `color.orange.700` | `#C84719` | Pressed, gradient transitions |

The orange is to be validated against the physical accent part (§7) under the same light, and every pair is to be checked for contrast before production.

### 3.2 Semantic colours

Semantic colours communicate state, never decoration, and keep one meaning everywhere.

| Token | Value | Meaning | Status LED equivalent (IEC 60073) |
|---|---|---|---|
| `color.state.error` | `#FF5D65` | Error, fault or critical state | Red: danger, fault |
| `color.state.warning` | `#F3C66B` | Warning, attention required | Amber: abnormal or warning |
| `color.state.success` | `#64D6A1` | Successful operation, normal | Green: normal |
| `color.state.info` | `#69BFFF` | Information that asks for attention or an action; system activity | Blue: action required |
| `color.state.processing` | `#A995FF` | Processing or computational activity | none (see §18.4) |

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
  background: var(--color-bg);
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
  background: var(--color-bg);
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

## 6. Materials and geometry (summary)

The physical product is specified in the [case industrial-design spec](https://github.com/opendap-project/case/blob/main/docs/specs/industrial-design.md). In short:

- A predominantly synthetic palette: matte graphite polymer for the body (CASE-020, CASE-021), with satin or anodised details, translucent light guides and elastomer only where they have a function.
- Three geometry families, shared by hardware and interface: **primary** (rounded rectangles, broad radii: enclosure, panels, cards), **control** (circles, capsules, segmented capsules, soft concavity: buttons, button groups and their on-screen counterparts) and **instrumentation** (fine lines, arcs, restrained markings: secondary information). Instrumentation never overpowers the primary geometry.
- Edges are softened by geometry, highlights and material transitions, never by bright outlines or exaggerated chamfers.

Digital radii (§10.2) are interface tokens; physical radii are set by industrial design and manufacturing (CASE-028).

## 7. Physical controls (summary)

Specified in the [controls and status LED spec](https://github.com/opendap-project/t1/blob/main/docs/specs/controls-and-status-led.md) (§ Physical control design). The button design in the [T1 concept art](https://github.com/opendap-project/t1/tree/main/docs/concept-art) governs (T1-059); this spec describes it. Other details in that concept art are stale (480 × 800 screen, 1.27 mm pins, earlier dimensions) and are not a source.

Seen from the front:

- **Left edge, top: Action** (T1-014, T1-022). A round disc, subtly concave, with deeper travel and a clearer click. It is the main action button and the only control in the orange brand accent.
- **Left edge, below Action: Volume + and Volume −.** Two half-capsules: together they form one capsule split across the middle, each half with a rounded outer end and a flat inner edge at the split, operated as two separate buttons. Not a rocker. This pair is the reference instance of the **button group** pattern (§15.1).
- **Right edge: Main (power)** (T1-015). A taller capsule with horizontal ribs, so it is identified by touch.
- Volume + / − and Main are silver-finished plastic: a metallic silver paint under a satin clear coat on the polymer cap, with the + and − reliefs in graphite (maintainers, 2026-10-23). Action stays in the orange accent; the body stays graphite.

On screen, the same hierarchy holds: one primary action per screen carries the orange emphasis, and button groups use the same segmented capsule (§15).

## 8. Interaction

### 8.1 A dedicated device

The T1 is an audio player, not a general-purpose smartphone. The interface puts first:

1. immediate playback control;
2. fast access to the current track or source;
3. clear volume and output status;
4. reliable navigation through the library, at any size (T1-046, SYS-033);
5. readable metadata;
6. settings and diagnostics when needed.

Do not add a conventional app launcher, dashboards or complex navigation only because they are familiar from smartphones.

**One instrument, not a small phone.** The screen and the four body buttons are one interface, and familiar mobile patterns are used only after asking what they do on this device ([interface mockups](https://github.com/opendap-project/blackbird-ui/blob/main/renders/web/ui-mockups.html)):

- **Natural mapping.** What a control does is drawn beside it, wherever the [device description](https://github.com/opendap-project/system/blob/main/docs/specs/hardware-support.md) places it: the volume level beside the volume controls, power choices beside the control whose role is `main` (a second press of it confirms power off), an edge mark beside the `action` control when it does something other than play/pause. When a role has no physical control, its function moves to the screen. On the T1 reference board the controls sit at about 38 (Action), 144 and 226 px (Volume + and −) on the left edge and 90 px (Main) on the right, from the concept art, to be confirmed with the case.
- **Music is home.** The T1 wakes in Now Playing; the library lies below it, the queue to its right, details below the player. No bottom tab bar, no launcher.
- **Content, not chrome.** Type, artwork and hairlines carry structure: no card inside a card, no floating action button, no hamburger, no tile grid for status. The status bar unfolds into its own detail, item by item in the same order.
- **Engineered clarity.** The signal path is drawn as a chain, profiles as stops on the axis below, technical values as a right-aligned mono readout.

Audio screens follow one axis (SYS-041): from the audiophile, who wants to enjoy the music, to the engineer, who wants to monitor, analyse and compare. Defaults serve the audiophile (the music first, technical detail quiet); every engineering view (signal path, meters, A/B) is one step away, never buried.

### 8.2 Hierarchy

Every screen has a **primary** level (the current task or the most important information), a **secondary** level (supporting controls and metadata) and a **tertiary** level (technical detail, diagnostics, optional configuration). Technical detail is available on demand, never permanently in the way.

### 8.3 Touch targets

Targets are defined in millimetres, so they hold on any display (§18.2). On the T1 reference panel, which has the original iPhone's size and density (320 × 480 at about 166 ppi), a phone layout cannot be shrunk onto it: targets are sized in millimetres for one-handed thumb use (DS-014).

| Target | Minimum | On the T1 reference panel (166 ppi) |
|---|---|---|
| Primary action (one per screen, for example play/pause) | 9.8 mm | 64 × 64 px (`size.target-primary`) |
| Frequent actions (skip, back, like) | 7.9 mm | 52 × 52 px (`size.target-frequent`) |
| Any other touch target | 7.4 mm | 48 × 48 px (`size.target-min`) |
| Gap between hit areas | 1.2 mm | 8 px (`space.target-gap`); hit areas never overlap |
| Gap between destructive or opposite actions | 2.5 mm | 16 px (`space.4`) |
| On-screen keyboard keys (documented exception) | 4.9 mm wide, 7.0 mm tall | 32 × 46 px, with long-press alternates and hit-target correction |

- A hit area may be larger than the drawn control; status-bar icons are not targets of their own.
- **Side margins are 16 px** (2.4 mm; `space.margin`), leaving 288 px of content width. The 20 px edge-gesture zones overlap the margin; they react only to swipes that start at the edge.
- **Rows:** 52 px for one line, 60 px for two lines (`size.row`, `size.row-two`); 6–7 one-line rows fit between the header and the playing strip.
- At most five targets in a row: five 48 px targets with their 8 px gaps take 272 of the 288 px content width (five 52 px targets do not fit).
- Primary and frequent actions sit in the lower half, where the thumb rests; the top holds what is used least, and each of those has a gesture too (§8.5).

For physical controls, evaluate actual size, finger clearance, reach and tactile discrimination rather than applying pixel targets.

### 8.4 Feedback

Every interaction gets immediate, unambiguous feedback within the [experience budgets](https://github.com/opendap-project/t1/blob/main/docs/specs/experience-budgets.md) (touch to visual response ≤ 50 ms ideal; T1-057):

- a volume change shows a visible value or level change;
- the playback state has a clear icon, and a label where it helps;
- connection errors look different from buffering or loading;
- long operations show progress and allow cancelling where possible;
- destructive actions ask for proportionate confirmation;
- what the screen shows always matches what the device is doing (T1-057).

Animation clarifies a change of state; it never decorates every interaction.

**States** ([ADR 0003](../adr/0003-screen-states.md)). Empty: what is empty, how to fill it, at most one action. Loading: the structure at once, a 2 px line past 300 ms, "Loading…" past 1 s, a value and Cancel when the length is known. Error: icon and words, what failed and why, the next step and its action, never looking like loading. Disabled: 45 % opacity (`opacity-disabled`) with its reason always visible under it; the buttons skip it (maintainers, 2026-10-24).

**The buttons and the keys, by meaning** (maintainers, 2026-10-23). Volume + is the upper button and Volume − the lower one, so their meaning follows place and amount, which agree vertically: in focus navigation Volume + is the previous element (up) and Volume − the next (down); on a locked slider and as the volume, Volume + is more and Volume − less; held, both repeat with acceleration. Action follows the focus: the screen's primary action by default (the focus lands there on entry), the element the buttons moved the focus to once they did, and with the focus hidden the primary action where there is one (system input events §3.4). Escape is back, after a lock's release. A keyboard maps ↑ ↓ to the buttons by place, Enter and Space to Action; ← and → are a keyboard's arrows, never buttons (the buttons stand vertically, and → cannot mean both "next" and "more"), and a focused control takes them: → more and ← less on a slider or an axis of stops. ↑ ↓ change a value only where Volume does: on a locked control. With the display off, Action is play/pause and Volume the volume.

**Touch and the focus** (maintainers, 2026-10-23; CSS `:focus-visible`, Android's touch mode). The focus is always somewhere (focus on entry), but its ring shows only once the buttons or a keyboard are used, and hides when the screen is touched. Touching an element gives it the focus, hidden, so the buttons continue from where the finger was; a pointer hovering moves nothing; touching what takes no focus leaves the focus where it was, and hides its ring too, as does scrolling. A navigation button or key (Volume, Tab, an arrow) while the focus is hidden only reveals where it is: it does not move it. Action is not one of them: with the focus hidden it is the screen's primary action, and only reveals the focus where the screen has none. A long press is never also a tap, and a touch that scrolled or dragged takes nothing.

**Sliders under the buttons** (maintainers, 2026-10-23). A slider's focus is on its knob, the control, not on the field: the focus moves through it like any control. Enter, Action or Space locks it for adjustment (a thicker ring with a filled halo, in the accent of live input); while locked, Volume + raises the value and Volume − lowers it, held with the same acceleration, and the same keys or leaving release it. A range takes the focus one knob at a time: low then high going down, high then low coming back up. An axis of stops (radio stops) locks the same way, Volume + to the next stop, Volume − to the previous; ← → move it at any time. A keyboard's ↑ ↓ act only while locked, as the buttons they stand for.

**Focus on entry** (maintainers, 2026-10-23). Every view or activity that comes to the foreground, a new one or one returned to, moves the focus: to the element it declares as its primary action, always, where it has one; otherwise to its first focusable element in reading order (the header's way back is reached by Volume +, not landed on). So the buttons alone (Volume to move, Action to take it) work from the first press, and Action alone does the screen's main job: the power menu opens with its first choice focused.

### 8.5 Gestures

Gesture navigation is supported on touch faceplates (SYS-039); it follows Android's model where it fits, and uses the bottom edge for "more" because the T1 has no phone home screen (§8.1).

| Gesture | Action |
|---|---|
| Swipe in from the leading edge (left; right in RTL) | Back |
| Swipe in from the trailing edge | The screen's companion screen, entering from that side: from Now Playing, the queue |
| Swipe up from the bottom | More of the current screen; on Now Playing, the signal path and technical details, lyrics, credits |
| Swipe down from the status bar | Notifications and quick settings |
| Swipe down on Now Playing | Minimise the player |

- Edge zones are 20 px (3 mm) wide at the sides and the bottom; the top zone is the status bar (28 px), where "swipe down from the status bar" starts. Where two meet, the square is a **corner**, a region of its own (maintainers, 2026-10-23): a contact that starts there belongs to the corner only, never to either edge, whatever the gesture (a tap, a long press, a pan, a swipe, a drag). A swipe from a corner is not an edge swipe; corners have no action yet. Every gesture carries the region it started in: the content, an edge or a corner.
- Controls that are dragged (the seek bar, sliders, horizontal lists) exclude their area from the side zones, and exclusions stay small. **Not built yet:** the composer recognises edge swipes only in the debug UI mode so far; exclusions come with the system's edge navigation.
- Composite gestures (a pan, drag and drop) and their directives (axis, start, bounds, snap) are system input events §3.2: a drag is free by default and starts after a long press, so a plain move still scrolls.
- Every gesture has a visible single-tap alternative (back control, chevron, queue control, a "more" control, the status bar), as WCAG 2.2 requires for path gestures and dragging (2.5.1, 2.5.7).
- Non-touch faceplates navigate with HID keys (§13); gestures are never the only way.

## 9. Typography

Type communicates precision and stays comfortable for everyday reading.

### 9.1 Roles

| Role | Family | Use |
|---|---|---|
| Interface | IBM Plex Sans (and its script companions, §9.4) | All interface text |
| Technical | IBM Plex Mono | Values, timestamps, identifiers, formats and sample rates, diagnostics, tabular data |
| Fallback | System sans-serif and monospace | Web and Desktop only, when Plex is not available |

The technical face is for values that benefit from fixed-width comparison, not for every paragraph.

### 9.2 Hierarchy

| Role | Treatment |
|---|---|
| Screen title | Clear, medium or semibold |
| Primary value | Prominent, highly legible |
| Section label | Compact and restrained |
| Body text | Comfortable regular weight |
| Technical metadata | Monospaced where it improves comparison |
| Auxiliary label | Muted but readable (`text-muted`, never `graphite.500` for small text) |

Avoid wide tracking, tiny uppercase labels and condensed futuristic faces. Uppercase never applies to scripts without case, and is never forced by a template (DS-006).

### 9.3 Legibility

Consistent line height; no long uppercase passages; a clear difference between primary and secondary text; no small text over plasma or thermal effects; the minimum type size is tested on the target display.

The type scale (tokens `font.size.*` and `font.line-height.*`; revised 2026-10-23, to be checked on the prototype panel; DS-015). It is sized by visual angle for the handheld distance class, **35 cm** (other classes scale with their distance, §18.2): character height should be at least 16 minutes of arc where legibility matters and 20–22′ is preferred (ANSI/HFS 100, as quoted in the FAA Human Factors Design Standard). The first scale (14 px body, 11 px caption) fell below 16′ beyond about 33 cm. On the T1 one pixel is 0.153 mm (§18.2); cap heights are IBM Plex Sans's (0.698 em):

| Role | Size / line height (px) | Cap height | At 35 cm | Use |
|---|---|---|---|---|
| `display` | 32 / 40 | 3.4 mm | 33.6′ | Large numbers (volume overlay, readouts) |
| `headline` | 24 / 32 | 2.6 mm | 25.2′ | Screen titles that carry the screen |
| `title` | 20 / 28 | 2.1 mm | 21.0′ | Track title, screen titles |
| `body-large` | 17 / 24 | 1.8 mm | 17.8′ | Primary list rows, reading text |
| `body` | 15 / 22 | 1.6 mm | 15.7′ | Secondary lines, sentences |
| `label` | 13 / 18 | 1.4 mm | 13.6′ | Control labels, metadata, section labels |
| `caption` | 12 / 16 | 1.3 mm | 12.6′ | Status bar, timestamps, technical values (IBM Plex Mono); the minimum |

Nothing smaller than `caption` (12 px, the watchOS minimum; one point on the original iPhone panel is about one T1 pixel). Text that must be read while walking uses `body` or larger; `label` and `caption` carry identification that is not time-critical. Line heights are set explicitly on a 2 px grid, never "normal": Plex Sans SC's 1.0 em line gap would double them. Larger text sizes scale these roles nonlinearly (§13).

### 9.4 Script coverage

The UI ships with English, Mandarin Chinese, Hindi, Spanish and Arabic (SYS-028, DS-011). IBM Plex has a companion family for each launch script, all under the SIL Open Font License 1.1, checked in the [IBM/plex repository](https://github.com/IBM/plex) on 2026-10-22:

| Script | Languages | Family | Package version | Regular OTF size |
|---|---|---|---|---|
| Latin | English, Spanish | IBM Plex Sans; IBM Plex Mono | 1.1.0; 2.5.0 | — |
| Han (Simplified) | Mandarin Chinese | IBM Plex Sans SC | 1.1.0 | 6.98 MB (hinted) |
| Han (Traditional) | not a launch locale | IBM Plex Sans TC | 1.1.1 | — |
| Devanagari | Hindi | IBM Plex Sans Devanagari | 1.1.0 | 0.18 MB |
| Arabic | Arabic | IBM Plex Sans Arabic | 1.1.0 | 0.17 MB |

Each family ships eight weights (Thin to Bold, plus Text). Checked on 2026-10-23 against the package versions above (fontTools, HarfBuzz through uharfbuzz 0.56.2, HarfRust 0.15.0):

- **Chinese coverage.** Plex Sans SC maps all 6,763 hanzi of GB 2312 and 20,971 CJK unified ideographs in all; of GB 2312's 682 symbols only `§` is missing, and IBM Plex Sans has it, so the fallback chain covers it.
- **Shaping.** HarfRust 0.15.0, the shaper the device uses, produces the same glyph sequence as HarfBuzz for all 23 test strings (Devanagari conjuncts, reph, reordered i-matra; Arabic joining, lam-alef, Arabic-Indic digits; Chinese with full-width punctuation), with no missing or dotted-circle glyphs. Glyph positions and parley's line layout are not checked yet.
- **Digits.** Plex Mono and Plex Sans have only Western digits; Plex Sans Arabic adds Arabic-Indic and extended Arabic-Indic digits, Plex Sans Devanagari adds Devanagari digits. Western digits are tabular in every family (600 units); the script digits are proportional, and none of the families has a `tnum` feature.
- **Storage.** Plex Sans SC Regular is 6.97 MB as an OTF (a Noto Sans SC subset is 8.33 MB); a GB 2312 subset of it, with ASCII, CJK punctuation and full-width forms, is 2.34 MB.

Decided on 2026-10-23 from these results:

- **CJK font:** IBM Plex Sans SC, not a Noto subset: it covers the launch locale, matches the rest of Plex, and is smaller. The image ships it complete at the weights the type scale uses (Regular and SemiBold, about 14 MB), so rare characters in library metadata never fall back; no first-boot subset.
- **Technical values** (formats, sample rates, bit depths, timestamps in diagnostics) use Western digits in IBM Plex Mono in every locale, so they stay aligned. Other numbers (the time, track numbers, counts) follow the locale's digits through the script's own Plex family.

### 9.5 Right-to-left

Arabic makes right-to-left a requirement from the first release (SYS-025). Layouts mirror: leading and trailing swap, horizontal sequences reverse, directional icons (back, forward, chevrons, progress direction) flip. Media transport controls (previous, play/pause, next) and the seek bar's direction do not mirror: they show the direction of the tape, not of reading, and physical players look the same everywhere ([Material Design, bidirectionality](https://m2.material.io/go/design-bidirectionality), checked 2026-10-23). Volume, which stands for a physical order too, does not mirror either (§15.1). Slint does not mirror layouts yet ([#2294](https://github.com/slint-ui/slint/issues/2294)), so Blackbird UI does it: its rows are `DirRow`, a one-line `FlexboxLayout` reversed right to left by the `Layout` global (the focus keeps reading order, start to end), `Label` aligns to the layout's start (or its end, with `end`), and the components flip what they draw (chevrons, a capsule's ends, a toggle's knob, a progress fill). Tokens are written in logical terms (start, end), never left and right.

### 9.6 Per-script metrics

Per-locale size and line-height tokens, never per-string overrides (DS-016). Measured on the Plex files; renders on the T1's own renderer are still to be checked:

| Script | Minimum for captions and metadata | Line height | Note |
|---|---|---|---|
| Latin | 12 px | 1.3–1.4 em in rows, 1.45 em in sentences | — |
| Han (Plex Sans SC) | 12 px | 1.4–1.5 em | Digits from the Latin face (SC's are 630 units wide, Plex Sans's 600) |
| Devanagari | 13 px | 1.5–1.6 em | Matras: ink reaches about 1.25 em on common words |
| Arabic | 13 px | 1.5–1.6 em | Vowel marks reach about 1.34 em; layout mirrors (§9.5) |

### 9.7 Long text and translation

- **Cut, not scrolled** (DS-020). End ellipsis for titles, artists and albums; middle ellipsis for file and device names; the ellipsis at the logical end in right-to-left text. No automatic marquee: scrolling only the focused row after a delay is a setting, off by default (WCAG 2.2.2; driving guidelines reject automatically scrolling text). The Now Playing title wraps to two lines before it is cut; the full text is always reachable (the More view and the accessible name).
- **Expansion budgets** against English: 300 % for buttons and tabs of up to 10 characters (or two lines), 200 % for 11–20 characters, 160–180 % for sentences (W3C). Side-by-side buttons only where the longest translation fits; otherwise they stack.
- **Formats** from CLDR (grouping, decimal separator, digits, plurals, relative time), never concatenated; sentence case everywhere.

## 10. Spacing, radii, borders and elevation

### 10.1 Spacing

A 4 px base: `space.1` 4, `space.2` 8, `space.3` 12, `space.4` 16, `space.5` 20, `space.6` 24, `space.8` 32, `space.10` 40, `space.12` 48 (px).

**Proximity decides the gap, not a default** (DS-017). Spacing says what belongs together:

| Relationship | Space | Token | Example |
|---|---|---|---|
| Within one unit | 0–4 px | `space.unit` (4) | Title and artist; a label and its value; a value and its slider |
| Within a group | 8–12 px | `space.group` (12) | Items of a section; artwork and its caption; the format line and the seek bar |
| Between groups | 16–24 px | `space.section` (20) | Sections of a screen; a readout and the list below it |

Lists separate lines with hairlines (`border`, 1 px), not with gaps or cards. A section label sits one group step after the previous group and one unit step before its first line. Side margins are 16 px (`space.margin`, §8.3). Adapt density to the display, but never compress important controls to fit more content. Layouts leave room for text expansion between languages (DS-006).

### 10.2 Radii

`radius.control` 10 px · `radius.card` 16 px · `radius.panel` 20 px · `radius.device` 28 px · `radius.pill` 999 px · `radius.round` 50 % · `radius.group-outer` (outer ends of a button group, = `radius.pill`; §15.1). These are digital tokens; physical radii are decided by industrial design and manufacturing.

### 10.3 Borders

Low-contrast graphite borders; brighter borders for active or focused elements; not every component outlined; orange borders are not a general emphasis mechanism.

### 10.4 Elevation

Prefer tonal contrast between surfaces (`graphite.950` → `900` → `850` → `800`) over large shadows. A raised surface differs from its background by its treatment, not by floating dramatically above it.

## 11. Motion

Motion is deliberate, controlled and physically motivated.

### 11.1 Principles

Short transitions for direct interactions; smooth, restrained easing; no bounce; no perpetual decorative movement; no sudden full-screen flashes; no animation that delays access to primary controls. Every animation runs at the display's full frame rate (T1-056).

| Token | Value | Use |
|---|---|---|
| `motion.duration.fast` | 120 ms | Press states, small toggles |
| `motion.duration.standard` | 200 ms | Most transitions |
| `motion.duration.slow` | 320 ms | Large surface changes |
| `motion.easing.standard` | `cubic-bezier(0.2, 0, 0, 1)` | Default easing |

An animation never stands between an input and its effect: the state changes at once, and the animation shows it.

### 11.2 Plasma motion

When used, plasma movement suggests energy flow, not a looping screensaver: slow movement of the core, slight changes in the outer gradient, audio-reactive amplitude or intensity, a brief activation when playback starts, gentle settling when a process ends. No rapidly cycling colours and no high-frequency brightness changes.

### 11.3 Thermal motion

Extremely subtle and slow; atmospheric detail, never a source of information.

### 11.4 Reduced motion

Blackbird OS has a system **reduce motion** setting, and web surfaces honour `prefers-reduced-motion` (DS-012). Reduced motion replaces animation with a static state; it never removes the feedback the animation carried.

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

## 13. Accessibility

Accessibility is part of engineering quality (DS-010):

- Text meets WCAG 2 AA contrast: 4.5:1 for normal text, 3:1 for large text and for the visual cues needed to identify a control (§3.4).
- Critical state is never shown by colour alone: a label, an icon, a shape or a pattern carries it too.
- Active, disabled, loading and error states are distinguishable.
- Reduced motion and reduced effects are supported (§11.4, §12.4).
- Text stays legible at the smallest supported size on the real panel.
- Every control has a clear accessible name.
- Primary actions never need fine motor precision; physical controls have spacing and tactile distinction.
- Plasma effects never cover important information.
- Keyboard and non-touch input reach every control (SYS-021 to SYS-026; non-touch faceplates send HID navigation).

- Built-in features (SYS-061 to SYS-066): voice guidance driven from the buttons and by touch, turned on without sight; text size in five steps to about 200 % with reflow, and a large-text mode (EN 301 549 V4.1.1 5.1.4: x-height of at least 2.38 mm, about a 30 px font, one item per line); increase contrast (text ≥ 7:1), bold text, reduce motion, transparency and effects; a light, high-contrast appearance for sunlight and low vision (an accessibility appearance, not a second theme, SYS-030); colour filters; mono and balance; configurable press and hold timings.

The display and physical controls are also evaluated under realistic light and viewing angles; in direct sun only panel luminance, anti-reflection, larger and bolder type and the light appearance keep text readable.

## 14. Design tokens

The tokens live in one platform-neutral file, [`tokens/opendap.tokens.json`](../../tokens/opendap.tokens.json), in the W3C Design Tokens Community Group format ([2025.10](https://www.designtokens.org/tr/2025.10/format/)). Slint globals for the device and CSS custom properties for web, documentation and OpenDAP Desktop are generated from it; neither is edited by hand (§18.1).

Groups: `color` (primitives), `semantic` (surfaces, text, brand, state, border, focus), `size` (strokes such as the button-group divider, target and bar sizes, the status bar's icon size `size-icon-status`), `opacity` (`opacity-disabled`, the Disabled state), `plasma` (spectrum and reference gradients, kept apart so it can be disabled as one group), `font`, `space`, `radius`, `motion`, `effect`.

The CSS names generated from the semantic tokens follow the draft's names, for example:

```css
:root {
  --color-bg: #111315;
  --color-surface: #191c1f;
  --color-surface-raised: #202428;
  --color-surface-active: #292e33;
  --color-text: #f5f6f4;
  --color-text-secondary: #c6cbd0;
  --color-text-muted: #939ba3;
  --color-brand: #ff6a2a;
  --color-brand-hover: #ff874d;
  --color-brand-pressed: #c84719;
  --color-border: #343a40;
  --color-border-strong: #515a63;
  --color-focus: #ff874d;
  --font-ui: "IBM Plex Sans", system-ui, sans-serif;
  --font-mono: "IBM Plex Mono", ui-monospace, monospace;
  --duration-standard: 200ms;
  --ease-standard: cubic-bezier(0.2, 0, 0, 1);
}
```

No colour, radius or timing is written outside the tokens without a recorded reason.

## 15. Components

- **Primary action:** its shape follows its content (maintainers, 2026-10-23): a 64 px circle when it is an icon (play/pause in the transport row), a full-width capsule 64 px tall when it is a word (Continue, Pair); the only orange emphasis on the screen; dark foreground on the orange fill (§3.4); a clearly visible pressed state (`brand-pressed`); no persistent glow; an accessible name and state.
- **Secondary action:** graphite surface; neutral border or tonal separation; white label or icon; orange only for selected or active states.
- **Tabs:** navigation between views of the same level. Text labels in a row, the current one in `text` with a 2 px underline, the others `text-muted`; no container, no capsule; each tab at least 48 px wide and tall (§15.2).
- **Button groups and segmented controls:** actions that belong together, or one value chosen from two to five options; see §15.1. Never for navigation.
- **Playback controls:** play and pause immediately distinguishable; skip controls subordinate to the primary action; volume easy to reach; the playback state never shown by animation alone; output and format settings reachable without cluttering the playback screen. Transport controls do not mirror in right-to-left layouts (§9.5).
- **Status indicators:** stable semantic colours with a label, icon or other non-colour cue; animated luminous indicators only for active processing; no permanent decorative blinking. Where the screen shows the same state as the status LED, it uses the same colour family (§18.4).
- **Panels and cards:** graphite tonal surfaces; consistent radii; minimal borders; clear internal spacing; no deep nesting; no plasma background on ordinary cards.
- **Diagnostics and technical information:** IBM Plex Mono for values that need fixed-width alignment; advanced information on demand; expert use supported without turning every screen into a diagnostic dashboard. Under "100 % of the hardware" (T1-045), everything the hardware does is reachable; the hierarchy decides where, not whether.
- **On-screen keyboard:** follows these tokens and the keyboard sizes in §8.3; its layout mirrors for right-to-left languages.

### 15.1 Button groups: the segmented capsule

Button groups, physical and digital, use one pattern (DS-013): **one capsule split into segments**. The outer ends are rounded; the inner edges at each split are flat; every segment is its own button; splits are marked by a consistent gap or divider. The T1's Volume + / Volume − pair is the reference instance (§7).

On screen this covers button groups and segmented controls, for example gain, filter, output or mode selectors:

| Part | Rule | Token |
|---|---|---|
| Outer ends | Full capsule radius on the first and last segment's outer side; square inner corners | `radius.group-outer` (= `radius.pill`) |
| Splits | One divider style per group, gap or line, never mixed | `size.group-divider` (1 px) with `semantic.border.group-divider`, or `space.group-gap` (2 px) |
| Segments | Equal height; equal width unless the labels are of very different lengths; each meets the touch minimum (§8.3) | — |
| Selected segment | Tonal fill (`surface-active`) with a non-colour cue (weight, icon or indicator); orange only when the group is the screen's primary action | — |
| Right-to-left | Segment order mirrors with the layout, except groups that stand for a physical order (volume, transport) | — |
| Accessibility | Each segment has its own accessible name; a single-choice group is announced as one group with a selected item | — |

A group of two to five segments fits the 320 px width at the touch minimum; more options go to a list or a picker.

### 15.2 Choosing the control

Each job has its control (DS-018); they are not interchangeable:

| Control | Job | Example |
|---|---|---|
| Tabs | Switch between views of the same level; the content below changes | Library: Albums, Artists, Playlists, More |
| Button group (§15.1) | Several related actions, or one value from two to five options that acts at once | Pause / Sync now; a DAC filter selector |
| Toggle | One setting on or off, its state in words or a switch | Bluetooth on; blind comparison off |
| Radio stops on an axis | One of a few options on a meaningful scale | Profiles from "enjoy" to "analyse" (SYS-041) |
| List line | Go to a thing or a sub-screen; full width, hairline, chevron | Settings parts, artists |
| Primary action | The screen's one main action, also on the Action button; orange, 64 px | Play, Eject, Pair |

### 15.3 Icons

- **Set: OpenDAP Icons** ([`icons/`](../../icons/), DS-021, [ADR 0001](../adr/0001-tabler-icons-as-the-base-family.md)). Semantic names over [Tabler Icons](https://github.com/tabler/tabler-icons) (MIT, pinned), used as upstream draws them: 2 px strokes, round caps and joins, on a 24 px grid. The status bar uses the same glyphs with a 1 px stroke at 16 px, each moved as a whole onto the pixel grid so its straight strokes are sharp 1 px lines ([icons README](../../icons/README.md)). OpenDAP draws a glyph only where Tabler has none and the meaning matters (the 3.5 mm and 4.4 mm outputs). Names are semantic, so themes (SYS-030) and derivatives (OS-028) can redraw any icon, or change the base family through the mapping, without touching code. The [preview](../../renders/web/icons.html) shows every icon at its real sizes.
- **Sizes:** status bar 16 px (2.3–2.4 mm, `size-icon-status`); beside text 16–20 px; header actions and secondary transport 24 px (3.7 mm); primary transport 28–32 px (4.3–4.9 mm). Every interactive icon sits in a hit area of at least 48 px.
- **Strokes in whole pixels** (2 px at 24 and 32); a scaled master that lands on 1.25 or 1.5 px is redrawn, not shipped.
- **Filled** glyphs for learned, stateful controls (play/pause, favourite on); outline for the rest.
- **Labels:** visible words on every icon except the transport set, volume and the status-bar glyphs; every icon-only control has an accessible name for its function, and toggles expose their state separately. Long press is never the only way to learn what a control does.

### 15.4 Splash screen

What the screen shows while the OS loads (blackbird-os OS-035): the OpenDAP vertical lockup (`brand/svg/opendap-vertical-light.svg`, mark above "OpenDAP"), light, centred on `semantic.bg`, at 40 % of the display's short side. Nothing else: no text, no progress bar, no console.

- **Minimum time:** 5 s on screen, so it is seen as an intended moment while the system loads, not as a flash (maintainers, 2026-10-23). If the interface is ready sooner it waits; if not, the splash screen stays until it is.
- **To the interface:** a fade of `motion.duration.slow` (320 ms). With reduced motion, no fade.
- **Who draws it:** the boot UI (`odap-boot`) from the moment a display exists, then the composer's first frames, from the same rasterization (system `opendap-splash`), so the hand-over shows no change. On a board with a boot-loader logo, that logo is the same lockup.
- **The same place** shows the boot-mode menu, progress of updates that need a reboot and other screens that keep the system from normal use, in the same visual language. Updates that do not need a reboot happen in the background, with the system in use.

## 16. Product renders

Renders, previews and concept art follow the same language as the interface. The rules for the physical product are in the [case industrial-design spec](https://github.com/opendap-project/case/blob/main/docs/specs/industrial-design.md) (§ Rendering rules); renders are built from components' `export/` and the [Blender library](https://github.com/opendap-project/blender-lib). In short: matte synthetic enclosure by default; soft, controlled studio light; orange only on the Action button and sparingly elsewhere; plasma confined to display content or a conceptual visualisation; accurate proportions; the hardware stays credible as a manufacturable object.

## 17. Performance

The interface runs at the display's refresh rate under any load (60 fps; T1-056, SYS-036; [experience budgets](https://github.com/opendap-project/t1/blob/main/docs/specs/experience-budgets.md)), and nothing visual may take time from the audio path.

- Static gradients are the default; animated effects are the exception and isolated from frequently redrawn content.
- Gradients drawn by the renderer before raster images, where they fit; pre-rendered textures for effects the renderer cannot draw cheaply.
- No stacked blur filters and no large displacement regions.
- No core function depends on GPU-intensive rendering; the interface is complete with every effect off.
- Every effect is measured on the target hardware (frame time and energy) before it ships; an effect that drops a frame, or moves the playback autonomy towards its warning value, is removed or reduced.
- Visual fidelity never costs responsiveness, legibility or battery life.

## 18. Alignment with OpenDAP requirements

How this design direction fits the decisions already taken.

### 18.1 Rendering: Slint on the device, tokens as the single source

- The device UI is rendered by **Slint** ([system ADR 0006](https://github.com/opendap-project/system/blob/main/docs/adr/0006-slint-as-host-renderer.md)), not by a browser. The CSS in this document is a **reference recipe** for web, documentation and OpenDAP Desktop, and a visual description for the device.
- Tokens are canonical in `tokens/opendap.tokens.json` and generated into Slint globals and CSS custom properties (DS-008) by `tokens/generate.py`, into `export/slint/` and `export/css/`.
- Slint 1.18.1 draws linear, conic and **circular** radial gradients only ("only circular gradients are supported", [Slint reference](https://github.com/slint-ui/slint/blob/v1.18.1/docs/astro/src/content/docs/reference/property-types/colors-and-brushes.mdx)), and has no general blur or displacement filter. On the device, the elliptical plasma field is a circular gradient stretched by its element or a pre-rendered image, and blur, glow and thermal displacement are pre-rendered textures or an application-rendered GPU texture. Which of these meets the budgets is **UNVERIFIED** until measured.

### 18.2 Displays: any display, each one optimised

The design system is written for any display, and optimised for each one it meets (DS-022, SYS-068). Nothing in it assumes one panel as the only valid one.

- **Design pixels.** Every size in this document and in the tokens is in **design pixels**. A design pixel is defined by angle, not by a panel: 0.153 mm seen from 35 cm (about 1.5 minutes of arc), which is one physical pixel on the T1 reference panel.
- **Scale.** The composer computes each display's scale from its physical size and the distance class in the [device description](https://github.com/opendap-project/system/blob/main/docs/specs/hardware-support.md) §3: `scale = (display pixels per mm × 0.153) × (distance ÷ 35 cm)`. It rounds to steps the renderer draws crisply. When a display's size is unknown, it falls back to a scale of 1, as GNOME does. Millimetre minimums (§8.3) therefore hold on every display, and type keeps its visual angle (§9.3).
- **Layout classes,** each designed for its characteristics rather than scaled from another: small portrait handheld (the T1's), small landscape, round, monochrome or character displays, and no display at all, served through voice guidance and the buttons. Templates implement every class they are offered in, as they implement every input profile (§13).
- **Shape and insets.** Rounded corners, round displays and cutouts come from the description as safe insets; content and targets stay inside them.
- **Technology.** OLED, LCD, e-paper and monochrome displays get their own treatment of colour, motion and refresh (an e-paper class draws no motion; an OLED class keeps large dark areas truly black).
- **The T1 reference panel:**
  - 3.5", 320 × 480 portrait, active area 48.96 × 73.44 mm, about 166 ppi, so 48 px ≈ 7.4 mm and the scale is 1 ([faceplate-touch ADR 0001](https://github.com/opendap-project/faceplate-touch/blob/main/docs/adr/0001-focus-lcds-e35gh-display.md));
  - its on-screen keyboard's 32 px keys (4.9 mm) are the documented exception, compensated by 46 px rows, long-press alternates and hit-target correction;
  - on OpenDAP Desktop and the web, design pixels follow the host's scaling and distance class.

### 18.3 Fluidity, effects and battery

- 60 fps under any load is a must (T1-056); effects never drop frames. Static by default; plasma and thermal only on the screens listed in §4.3; a reduced-effects mode (§12.4).
- The UI never waits on work (SYS-036) and an animation never delays an action's effect (T1-057, SYS-037).
- Effects count against the power budget (playback autonomy > 12 h ideal); an always-on animated effect on the playback screen needs a measurement before it is allowed.

### 18.4 Semantic colours and the status LED

The status LED follows IEC 60073 ([controls and status LED spec](https://github.com/opendap-project/t1/blob/main/docs/specs/controls-and-status-led.md)). The on-screen semantic colours use the same families for the same meanings: red for fault or danger (`error`), amber for warning (`warning`), green for normal (`success`), blue for a state that needs the user's action or patience (`info`, as for pairing, recovery or an update in progress), and neutral white for neutral information. `processing` (violet) has no LED counterpart; see §20. The screen never contradicts the LED.

### 18.5 Languages and scripts

Five shipped languages, including Arabic, so right-to-left and four scripts (Latin, Han, Devanagari, Arabic) are requirements from the first release (SYS-025, SYS-028, DS-006, DS-011). Typography in §9.4 and §9.5; templates have no hard-coded text and leave room for expansion.

### 18.6 Themes

One official theme, defined by these tokens (DS-007, SYS-030). A community theme is a replacement token set (and template layouts), with no code. Themes may change values, never the meaning of semantic colours, the contrast minimums, the target sizes or the reduced-motion and reduced-effects behaviour.

### 18.7 What this supersedes

DS-002 to DS-005 (withdrawn requirements: Plex with a Serif role, a warm black and ivory palette with per-activity accents, a layout anatomy and a 480 × 800 reference screen) are superseded by this spec.

## 19. Review checklist

### Identity and materials

- [ ] Engineered but approachable; nothing aggressive or gaming-like.
- [ ] Graphite, white and orange remain the vocabulary; orange stays at 2–5 %.
- [ ] Synthetic materials look believable; every detail has a function or an identity purpose.

### Ergonomics

- [ ] The main action is easy to find and operate; the round, concave Action button keeps its geometry and the accent.
- [ ] Volume + / − are two half-capsules of one split capsule; Main is the ribbed capsule; physical controls are distinguishable by touch.
- [ ] Every button group, physical or digital, uses the segmented capsule (§15.1).
- [ ] Touch targets and gaps meet the millimetre minimums on the real panel (§8.3).
- [ ] Every gesture has a single-tap alternative, and draggable controls near an edge exclude their area (§8.5).

### Plasma and thermal

- [ ] Plasma appears only on the screens §4.3 allows; it has a luminous core and controlled transitions; violet and cyan stay secondary.
- [ ] Glow is localised; thermal refraction is subtle and never touches legibility.
- [ ] Effects are static or restrained by default; reduced-motion and reduced-effects alternatives exist and were tested.

### Interface

- [ ] Primary information is understandable at a glance; routine screens are quiet.
- [ ] Every control gives immediate feedback; the screen matches the device state.
- [ ] Contrast meets §3.4; colour is never the only cue.
- [ ] Layout works in all five shipped languages, including right-to-left, with no clipped text.

### Engineering

- [ ] Measured at 60 fps on the target hardware under background load; energy measured for any animated effect.
- [ ] Every value comes from the tokens.
- [ ] The interface is complete with all effects off.
- [ ] Real hardware dimensions and the real display were used.

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
