# UI components requirements

Requirements for the official UI components and design system of Blackbird OS.

| ID | Requirement | Priority | Status |
|---|---|---|---|
| DS-001 | All Blackbird OS UI shall follow the official design system. | must | Confirmed |
| DS-002 | Typography: IBM Plex (Sans for the interface, Serif italic for reading and notes, Mono for data). | should | Withdrawn |
| DS-003 | Color: warm black #0E0D0C and ivory #F3EFE7, with one accent per activity (Listen #F0A63C, Watch #7DB3E8, Read #A3C98F, Create #EE7E62, Day #6CC4B4, Advanced #B7A4F0). | should | Withdrawn |
| DS-004 | Layout anatomy: status bar, a strip of activities (the active one large, the others peeking) and a 60 px playback thread at the bottom, replacing tabs and icons. | should | Withdrawn |
| DS-005 | Reference screen: 480 × 800 portrait; some views landscape 800 × 480. | should | Withdrawn |
| DS-006 | Components and templates shall be i18n-first: no hard-coded text, room for text expansion, mirrored layouts for right-to-left languages, and script-aware typography (SYS-025). | must | Confirmed |
| DS-007 | The design system shall define one official theme, with its tokens and templates structured so community themes can replace them without code (SYS-030). | must | Confirmed |
| DS-008 | The [design system spec](../specs/design-system.md) shall be the reference for all Blackbird OS UI, OpenDAP Desktop, documentation visuals and product renders; its tokens file is the single source of design values. | must | Confirmed |
| DS-009 | Plasma and thermal effects shall be a special visual mode, never used on routine screens (lists, settings, ordinary cards), and the UI shall offer a reduced-effects mode that replaces them with static or flat equivalents without losing information. | must | Confirmed |
| DS-010 | Text shall meet WCAG 2 AA contrast against its background in every theme, and no state shall be communicated by colour alone. | must | Confirmed |
| DS-011 | Typography shall define its roles (interface, technical values) and cover the scripts of every shipped language, starting with Latin, Han, Devanagari and Arabic (SYS-028), with right-to-left mirroring. | must | Confirmed |
| DS-012 | Motion shall respect the reduce-motion setting: animation is replaced by a static state, never by the loss of the feedback it carried. | must | Confirmed |
| DS-013 | Button groups, physical and digital, shall use the segmented-capsule pattern: one capsule split into segments with rounded outer ends and flat inner edges. | must | Confirmed |
| DS-014 | Touch targets, spacing and layout shall be sized in millimetres for the T1's 3.5" panel and one-handed thumb use, not scaled from a phone layout. | must | Confirmed |
| DS-015 | The type scale shall be sized for a 35 cm reading distance: text that must be read meets 16 minutes of arc of cap height at that distance, and nothing is smaller than the `caption` role. | must | Confirmed |
| DS-016 | Each shipped script shall have its own minimum sizes and line heights, as per-locale tokens. | must | Confirmed |
| DS-017 | Layouts shall use the T1's side margin and space elements by proximity (within a unit, within a group, between groups), never with one default gap. | must | Confirmed |
| DS-018 | Each kind of control shall keep its job: tabs for navigation, button groups for related actions or a value, toggles for one setting, list lines to go somewhere, one primary action per screen. | must | Confirmed |
| DS-019 | Icons shall come from one open icon set, at whole-pixel strokes, with visible labels except for the standard transport, volume and status glyphs, and an accessible name on every icon-only control. | must | Confirmed |
| DS-020 | Long text shall be cut with an ellipsis, never scrolled automatically, and its full text shall always be reachable. | must | Confirmed |
| DS-021 | OpenDAP shall have its own icon library, addressed by semantic names, which themes and derivatives can redraw icon by icon. | must | Confirmed |
| DS-022 | The design system shall be defined for any display and input set, in units that keep physical size and visual angle (design pixels), with layout classes each optimised for its characteristics; the T1 is the reference, not the rule. | must | Confirmed |

## Context

- DS-022: the UI and design system must not assume one hardware's characteristics as the only valid ones, while seeking the best optimisation for each (SYS-068, system ADR 0013). Design system §18.2.
- DS-021: an icon library of OpenDAP's own, a fork of another or at least the ability to customise. Then: an existing complete family close to the look, complemented only where something is missing. Built as OpenDAP Icons over Tabler Icons, used as upstream draws them, with OpenDAP's own glyphs only where Tabler has none ([ADR 0001](../adr/0001-tabler-icons-as-the-base-family.md), [`icons/`](../../icons/README.md)); replacement by name covers customisation.
- DS-015 to DS-020: values in the [design system](../specs/design-system.md) §8.3, §9.3, §9.6, §9.7, §10.1 and §15.
- DS-014: a 3.5" screen is not a phone; the values are in the design system §8.3.
- DS-008 to DS-012: from the maintainers' design direction, reconciled with the decisions already taken (Slint renderer, 166 ppi panel, 60 fps budget, status LED colours, five shipped languages). Draft until the maintainers confirm the wording.
- DS-013: the T1's Volume + / Volume − pair (T1-059) is the reference instance; on screen it covers button groups and segmented controls such as gain, filter, output or mode selectors.
