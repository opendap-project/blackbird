# Engineered Matter — Spacing, radii, borders and elevation

Part of the [Engineered Matter design system](../design-system.md): its status, scope and requirements apply here. Section numbers are the design system's, so a reference such as "design system §10" leads here.

## 10. Spacing, radii, borders and elevation

### 10.1 Spacing

A 4 px base, `space.1` to `space.12`; the steps and their values are in the [token reference](../../reference/tokens.md#space).

**Proximity decides the gap, not a default** (DS-017). Spacing says what belongs together:

| Relationship | Space | Token | Example |
|---|---|---|---|
| Within one unit | 0–4 px | `space.unit` (4) | Title and artist; a label and its value; a value and its slider |
| Within a group | 8–12 px | `space.group` (12) | Items of a section; artwork and its caption; the format line and the seek bar |
| Between groups | 16–24 px | `space.section` (20) | Sections of a screen; a readout and the list below it |

Lists separate lines with hairlines (`border`, 1 px), not with gaps or cards. A section label sits one group step after the previous group and one unit step before its first line. Side margins are 16 px (`space.margin`, §8.3). Adapt density to the display, but never compress important controls to fit more content. Layouts leave room for text expansion between languages (DS-006).

### 10.2 Radii

`radius.control` · `radius.card` · `radius.panel` · `radius.device` · `radius.pill` · `radius.round` · `radius.group-outer` (outer ends of a button group, = `radius.pill`; §15.1); values in the [token reference](../../reference/tokens.md#radius). These are digital tokens; physical radii are decided by industrial design and manufacturing.

### 10.3 Borders

Low-contrast graphite borders; brighter borders for active or focused elements; not every component outlined; orange borders are not a general emphasis mechanism.

### 10.4 Elevation

Prefer tonal contrast between surfaces (`graphite.950` → `900` → `850` → `800`) over large shadows. A raised surface differs from its background by its treatment, not by floating dramatically above it.
