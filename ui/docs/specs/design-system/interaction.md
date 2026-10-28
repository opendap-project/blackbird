# Engineered Matter — Interaction

Part of the [Engineered Matter design system](../design-system.md): its status, scope and requirements apply here. Section numbers are the design system's, so a reference such as "design system §8" leads here.

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

**States** ([ADR 0003](../../adr/0003-screen-states.md)). Empty: what is empty, how to fill it, at most one action. Loading: the structure at once, a 2 px line past 300 ms, "Loading…" past 1 s, a value and Cancel when the length is known. Error: icon and words, what failed and why, the next step and its action, never looking like loading. Disabled: 45 % opacity (`opacity-disabled`) with its reason always visible under it; the buttons skip it (maintainers, 2026-10-24).

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
