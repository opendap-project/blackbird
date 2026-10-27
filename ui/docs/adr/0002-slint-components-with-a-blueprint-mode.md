# ADR 0002: Slint components, each with a blueprint mode

- **Status:** Accepted
- **Date:** 2026-10-23
- **Decided by:** the maintainers (where the blueprint drawing lives, its semantics)

## Context

- Blackbird OS gets a debug UI mode, off by default and on by a flag, that shows the real interface as a blueprint at run time (system SYS-070). The UI primitives tests become a developer activity (SYS-071), and the mode applies to it and to every activity, element by element.
- The components had no format yet (`components/`: "Format not defined yet"). Each screen drew its own elements, so no mode could reach them all.
- The maintainers chose, of three options, that the components draw themselves as blueprints and the composer draws its own references above (regions, physical controls, touches, gestures). The other options were the composer drawing outlines from the accessibility tree only, or both.
- They asked the drawing to use a plan's semiotics: line types, weights, patterns and semantic colours. A physical control is, for the OS, an interactive area with event handlers, so it reacts as one when pressed.

## Decision

- **Format.** Slint files in `export/slint/`, next to the generated tokens, imported as `@blackbird/slint/components.slint`: the controls of the design system §15 (header, label roles, button, primary action, list line, toggle, slider, segment of a button group). Every OpenDAP application builds from them: the product, activities, and later the OS Shell's layers.
- **Two drawings, one geometry.** Each component draws itself as the product, or, with `Blueprint.on`, as a technical drawing of itself (`export/slint/blueprint.slint`). The geometry never changes between the two: size, radii and place stay, so the blueprint shows the real layout.
- **One semantics for every element and the composer's references** (`Blueprint` global):

  | Meaning | Drawing |
  |---|---|
  | Structure: containers, text, dimensions | Ink (white), hairline |
  | An interactive area (event handlers): button, line, toggle, slider, segment, physical control | `state-success` |
  | Live input on it: pressed, touched, held | `brand-base`, the accent the design system keeps for active states (§15) |
  | The composer's regions (the status bar), and an element's measure | `state-info` |
  | Side margins | `state-warning`, dotted |
  | Edge-gesture zones | `state-processing` |

- **On demand.** An element's measure (its box and size in design pixels) shows while a pointer or a finger is on it; at rest the drawing stays calm.

## Consequences

- An application turns the mode on from the system's flag (`/run/opendap/debug-ui`, or `ODAP_DEBUG_UI=1`) by setting `Blueprint.on`.
- Elements an application draws without these components are not drawn as blueprints. Detecting any element by its primitives (interactive areas and their kinds) is an open question in the system requirements.
- Slint has no dashed borders; dotted and dashed lines are drawn as segments where they matter (the margins).
