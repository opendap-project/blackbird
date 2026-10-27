# Design review checklist

Checks a screen, render or component against the [Engineered Matter design system](../../specs/design-system.md) before review. Section numbers are the design system's; this checklist is its §19.

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
