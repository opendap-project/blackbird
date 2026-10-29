# Engineered Matter — Components

Part of the [Engineered Matter design system](../design-system.md): its status, scope and requirements apply here. Section numbers are the design system's, so a reference such as "design system §15" leads here.

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

Button groups, physical and digital, use one pattern (DS-013): **one capsule split into segments**. The outer ends are rounded; the inner edges at each split are flat; every segment is its own button; splits are marked by a consistent gap or divider. The T1's Volume + / Volume − pair is the reference instance ([controls and status LED spec](https://github.com/opendap-project/t1/blob/main/product/docs/specs/controls-and-status-led.md)).

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

- **Set: OpenDAP Icons** ([`icons/`](../../../icons/), DS-021, [ADR 0001](../../adr/0001-tabler-icons-as-the-base-family.md)). Semantic names over [Tabler Icons](https://github.com/tabler/tabler-icons) (MIT, pinned), used as upstream draws them: 2 px strokes, round caps and joins, on a 24 px grid. The status bar uses the same glyphs with a 1 px stroke at 16 px, each moved as a whole onto the pixel grid so its straight strokes are sharp 1 px lines ([icons README](../../../icons/README.md)). OpenDAP draws a glyph only where Tabler has none and the meaning matters (the 3.5 mm and 4.4 mm outputs). Names are semantic, so themes (SYS-030) and derivatives (OS-028) can redraw any icon, or change the base family through the mapping, without touching code. The [preview](../../../renders/web/icons.html) shows every icon at its real sizes.
- **Sizes:** status bar 16 px (2.3–2.4 mm, `size-icon-status`); beside text 16–20 px; header actions and secondary transport 24 px (3.7 mm); primary transport 28–32 px (4.3–4.9 mm). Every interactive icon sits in a hit area of at least 48 px.
- **Strokes in whole pixels** (2 px at 24 and 32); a scaled master that lands on 1.25 or 1.5 px is redrawn, not shipped.
- **Filled** glyphs for learned, stateful controls (play/pause, favourite on); outline for the rest.
- **Labels:** visible words on every icon except the transport set, volume and the status-bar glyphs; every icon-only control has an accessible name for its function, and toggles expose their state separately. Long press is never the only way to learn what a control does.

### 15.4 Splash screen

What the screen shows while the OS loads (blackbird-os OS-035): the OpenDAP vertical lockup (`brand/svg/opendap-vertical-light.svg`, mark above "OpenDAP"), light, centred on `semantic.bg`, at 40 % of the display's short side. Nothing else: no text, no progress bar, no console.

- **Minimum time:** 5 s on screen, so it is seen as an intended moment while the system loads, not as a flash (maintainers, 2026-10-23). If the interface is ready sooner it waits; if not, the splash screen stays until it is.
- **To the interface:** a fade of `motion.duration.slow` (320 ms). With reduced motion, no fade.
- **Who draws it:** the boot UI (`blackbird-apps-boot`) from the moment a display exists, then the composer's first frames, from the same rasterization (system `blackbird-ui-splash`), so the hand-over shows no change. On a board with a boot-loader logo, that logo is the same lockup.
- **The same place** shows the boot-mode menu, progress of updates that need a reboot and other screens that keep the system from normal use, in the same visual language. Updates that do not need a reboot happen in the background, with the system in use.
