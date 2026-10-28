# Engineered Matter — Accessibility

Part of the [Engineered Matter design system](../design-system.md): its status, scope and requirements apply here. Section numbers are the design system's, so a reference such as "design system §13" leads here.

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
