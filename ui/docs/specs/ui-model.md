# UI model

- **Status:** Draft
- **Scope:** nesting, inheritance, override and propagation, the layout model, scrolling and motion, from the models that proved themselves
- **Builds on:** [ADR 0002](../adr/0002-slint-components-with-a-blueprint-mode.md) (components, blueprint mode), [ADR 0003](../adr/0003-screen-states.md) (screen states), the [design system](design-system.md), system [input events](https://github.com/opendap-project/system/blob/main/docs/specs/input-events.md), system ADR 0006 (Slint)

How OpenDAP's interface is put together, for whoever writes an activity or a component. Each section names the models that proved themselves, what Slint already gives (1.18), and what Blackbird UI adds. Nothing here replaces a standard Slint already implements; Blackbird UI adds only what Slint leaves out.

## 1. Scope and nesting

**Proven:** a tree of components composed, not inherited (React, SwiftUI, Compose, Flutter); children passed in as content (slots, `children`); each component owns its private state; one root per screen.

- **Slint gives:** components, `@children` for content, `inherits` for one base, private properties, `in`/`out`/`in-out` for the public surface.
- **Rules:**
  - **Composition over inheritance.** A component wraps others and takes content through `@children`; `inherits` only from a primitive (`Rectangle`, a layout, `Text`), never to extend another Blackbird UI component's behaviour.
  - **The public surface is the contract.** `in` for what the caller decides, `out` for what the component reports, `in-out` only for a value both sides own (a slider's `value`); callbacks for what happened. Nothing else is reached from outside.
  - **One screen, one root:** `Screen`, then its header, its scroll area, its content. Activities nest screens by navigation, never by drawing one screen inside another.

## 2. Inheritance of context

**Proven:** values that flow down the tree without being passed at every level and that the nearest provider overrides: CSS inherited properties (`color`, `direction`), React context, SwiftUI's environment, Compose's `CompositionLocal`, Flutter's `InheritedWidget`.

- **Slint gives:** globals (one value for the whole window), property bindings.
- **Blackbird UI's context**, as globals read by every component:

  | Context | Holds | Who sets it |
  |---|---|---|
  | `Tokens` | The theme's values (DS-007) | The theme (generated) |
  | `Blueprint` | The debug UI mode (SYS-070) | The application, from the system's flag |
  | `Motion` (to add) | Reduce motion (DS-012), reduced effects (DS-009) | The application, from the system's settings |
  | `Layout` (to add) | The direction (left-to-right or right-to-left, §9.5), the layout class (§18.2), the design-pixel `unit` | The application, from the description and the language |
  | `FocusReveal` | Where the focused element is | The components |

- **One focus target.** Every focusable control is built on `FocusTarget` (it inherits Slint's `FocusScope`), which holds what all of them do alike: disabled leaves the focus chain, gaining the focus reports where it is (`FocusReveal`; `fixed` outside a scroll area), the first navigation key while the focus is hidden only shows it, and `ring` says the focus is here and shown. A control adds only what its keys do, and what it does on gaining or losing the focus (a slider's lock ends). A focus scope's ends are `EndStop`s, first and last, which say when the buttons reached past them; the runtime then sends the focus back, and leaves Shift as it found it (`FocusNav.shift`), so a held Volume + or Shift+Tab stays at the first element. One implementation of each behaviour (system ADR 0014), so a fix or a new rule reaches every control at once.
- **Cascading states.** `enabled` cascades down as HTML's `fieldset disabled` and SwiftUI's `.disabled()`: a disabled group disables what it holds, and a child cannot re-enable itself inside it. The same for a reason (ADR 0003): the group's reason is shown once, not on every child.
- **Window-wide, not subtree.** Slint's globals are per window, so a context that must differ inside one subtree (a right-to-left island, a nested blueprint) is passed as an `in` property by the components that need it. This is the one gap against the environment model; it is rarely needed on a 3.5" screen, and it is named here so no one fakes it.

## 3. Override

**Proven:** layered tokens (primitive → semantic → component; the W3C Design Tokens format), theming by replacing a layer, local overrides at the call site, variants instead of ad hoc styling.

- **Order, weakest first:** the theme's primitives → its semantic tokens (`surface-*`, `text-*`, `state-*`) → a component's defaults → its variant (`PrimaryButton` round or capsule by content) → the call site's `in` properties. A community theme replaces the first two layers only (DS-007), and every component follows.
- **What a call site may override:** content, size where the layout needs it, its state and its callbacks. Not colours, radii or type: those come from the semantic layer, so a theme and the blueprint mode reach every element. A need that the tokens cannot express becomes a token or a variant, not a local value.

## 4. Data and propagation

**Proven:** data down, events up (unidirectional flow: Elm, React, Redux, SwiftUI's single source of truth); state lifted to the nearest owner; two-way binding only for a value the user edits in a control.

- **Slint gives:** bindings (`<=>` two-way), callbacks, `changed` handlers.
- **Rules:**
  - **State has one owner.** The activity's model (in Rust, the application core of the crate architecture) owns what the screen shows; components hold only what is theirs (pressed, focused, an animation's progress).
  - **Events go up as callbacks** with the domain payload (input events §6); the owner changes its state; the screen follows by binding. A component never reaches into another.
  - **`<=>` only for a control's value** (a slider, a toggle), and only between the control and its owner.
  - **Input propagation** is the input events' model: the system first, then capture, target and bubble, with directives and implicit defaults (input events §4, §5).
  - **Hardware in flight:** a value sent to the hardware is fire-and-forget, the latest winning; a physical state (a card in its socket) waits for the hardware's confirmation before the control says it changed.

## 5. Interaction states

**Proven:** one fixed set of states, layered in one order (Material's state layers; CSS's `:hover`, `:active`, `:focus-visible`, `:disabled`, `:checked`).

- **Slint gives:** `states [ … when … ]` with `in` and `out` transitions; `TouchArea.has-hover`, `pressed`; `FocusScope.has-focus`.
- **The set, in the order they combine:** disabled (dims everything; nothing else shows) → selected or checked → hovered (a pointer only) → focused (the ring, design system §8.4) → locked for adjustment (a slider) → pressed (live input). Each component uses Slint's `states` for them, so their transitions come from the motion tokens in one place.
- **Every state is visible without colour alone** (DS-010): weight, a ring, an icon, a fill, never a hue only.

## 6. Layout

**Proven:** constraints go down, sizes go up, the parent places (Flutter's and Compose's layout model); one-dimensional flex with gap, alignment and growth (CSS Flexbox, SwiftUI stacks, Compose `Row`/`Column` with weights); a two-dimensional grid for aligned tables (CSS Grid); logical directions (`start`/`end`, not left/right) for right-to-left; layout chosen by the space a component has (container queries), not by the device's name.

- **Slint gives:** `HorizontalLayout` and `VerticalLayout` (one dimension: `spacing`, `padding`, `alignment`, stretch factors, minimum, preferred and maximum sizes), `FlexboxLayout` (CSS Flexbox with wrapping), `GridLayout`; minimum, preferred and maximum sizes propagate up; elements without `x` and `y` are centred in their parent.
- **Rules:**
  - **One model per job** (the table below), chosen by what the content is, never by habit. Constraint solvers (Cassowary, Auto Layout, `ConstraintLayout`) are not used: their cost and their ambiguous or unsatisfiable layouts show only at run time, which is why SwiftUI, Compose and Flutter moved to stacks and single-pass constraints.
  - **Spacing is proximity** (design system §10.1, DS-017): a stack's spacing is a space token (`space-unit` within a unit, `space-group` within a group, `space-section` between groups); never a number at the call site.
  - **Side margins belong to groups,** so lists run the full width under them (§15.2).
  - **Logical directions:** `start` and `end` in every component's API; the `Layout` context mirrors them for right-to-left (§9.5); transport and physical-order groups do not mirror.
  - **By the space, not the device:** a component that changes shape does it by its own width (a container query), and the screen picks the layout class (§18.2); no component tests for "the T1".
  - **Explicit position only for overlays** (a badge, a focus ring, a toast); always set `x` and `y` there, since Slint centres what has none.
  - **A fixed size inside a stack caps the stack.** Slint gives a layout the smallest maximum of its children across its axis, so one element with a fixed width (a label sized to its text) makes the whole column that narrow. Put such an element in a row aligned to the start, or let it stretch.

**Which model when** (CSS Flexbox and Grid, SwiftUI, Compose, Flutter, Figma's auto layout; Yoga and Taffy show flex and grid cover applications on predictable engines):

| Model | For | Blackbird UI | OpenDAP examples |
|---|---|---|---|
| Stack (one axis) | Almost everything: children along one axis, with a gap, alignment and growth | `Stack` (vertical), `Row` | A screen's spine (header, content, playing strip); a settings row (icon, label filling, value, chevron); the status bar (start group, fill, end group); the power menu |
| Flow (wrap) | Items of their own size that continue on the next line, a few dozen at most | `Wrap` (Slint's `FlexboxLayout`) | Chips, format badges (FLAC 24/96, DSD) |
| Grid (aligned tracks) | Columns that must line up across rows | `ReadoutTable`, `Keyboard` (Slint's `GridLayout`, `colspan`) | The signal path's readouts, the on-screen keyboard, a specification sheet |
| Overlay (z inside one box) | Layers within one element | absolute children, always `x` and `y` | A badge on a cover, a focus ring, progress over the art |
| Anchored | Placed by another element, outside its container | `Overlay` (§9) | A popover, a context menu |
| Lazy (virtualized) | Unbounded data | `ListView`; `CoverGrid` (a list of rows, columns by the space: auto-fit) | Every library list, the cover grid, search results (no limits, T1-046) |
| Scroll composition | Mixed scroll content: a collapsing header, sticky sections, a list then a grid | `Scroll` with `SectionHeader` (sticky) and `CollapsingHeader` (transforms only) | An album (art header, then tracks); an artist (bio, a row of albums, tracks); A–Z letters |
| Adaptive | The same content, arranged by the space it has | `AdaptivePair` (a label and its value beside or under, SwiftUI's `ViewThatFits` for the commonest case, decided from the width its container gives, never its own), the `Layout` context's layout class | Now Playing: art above the controls, or beside them when wider; a setting's value inline, or under a long label; list and detail side by side on OpenDAP Desktop |
| Custom | A geometry no generic model expresses, measured in one pass | its own component | A radial dial, a waveform, a spectrum |

### 6.1 Spacing

- **Scale:** a 4 px base, the design system's space tokens (§10.1), in design px times `unit`: primitive steps, then semantic aliases (`space-unit`, `space-group`, `space-section`, `space-margin`, `space-target-gap`). A theme or a density mode replaces the layer, never a call site.
- **The container owns the space** (Every Layout's stack; Slint has no margins, which fits): a gap between children, padding inside the container, and no element ever spaces itself from its neighbours. Nothing collapses, nothing doubles.
- **Proximity reads as grouping** (Gestalt, DS-017): each step at least about 1.5 to 2 times the one below, so a unit, a group and a section read apart without lines.
- **Optical, not geometric:** measured from the ink (a text's cap height, an icon's visual weight), with the corrections as tokens (`play` nudged by its optical offset), never as numbers at a call site.
- **Rhythm:** line heights on the 4 px grid, so stacked text lands on it.
- **Density:** a compact mode tightens the semantic gaps; it never shrinks a touch target (§8.3).
- **Insets:** logical (`start`, `end`), mirrored for right to left; the system's insets (the status bar, a docked keyboard: `Window.safe-area-insets` and the composer's content area, system input events §3.2.1) applied once, at the screen's root.

### 6.2 Alignment

- **Main and cross axis** (CSS Box Alignment): `start` by default; `center` only for content with one focus (Now Playing's art, the power menu); `stretch` for what fills a row; logical, so right to left mirrors it.
- **Keylines:** the side margin (16 px), and a content keyline after a row's leading icon, so the text of every row starts at the same x (Material's keylines).
- **Across components** (CSS subgrid): rows of one list share their label and value columns through a `ColumnTrack` context the list publishes (the widest label), not each row by itself.
- **Baselines:** a row that mixes sizes (a label beside a large value) aligns on the first baseline; Slint has no baseline alignment yet, so a component pads the smaller text by the font metrics' difference inside itself.
- **Numbers:** tabular figures, aligned to the end, decimals aligned in readouts; a time counter never jitters (IBM Plex Mono's figures are tabular; Plex Sans needs `tnum`, which Slint cannot ask for yet).
- **Optical:** icons as Tabler draws them in their 24 px box (ADR 0001); a play triangle takes its optical offset.

### 6.3 What a complete layout also defines

| Concept | Rule |
|---|---|
| Z-order | Named bands (§9), never numbers |
| Intrinsic sizing | Minimum, preferred, maximum going up, in one pass; never an intrinsic measure inside a list row (Flutter documents `IntrinsicHeight` as possibly quadratic) |
| Overflow | Chosen per container (§6.4): wrap, elide, clip or scroll; never content overlapping by accident |
| Aspect ratio | One primitive (`AspectBox`), not a pair of fixed sizes |
| Density and scale | One `unit` scale (§18.2) and the density layer above |
| Text | End ellipsis by default, middle ellipsis for file paths, a line limit where it matters; the full text always in the accessible name |
| Translation | Room for 30 to 40 % longer short strings (W3C); tested with a pseudo-locale |
| Adaptive classes | The window's class from the `Layout` context, and each component by its own allocated width; never "if T1" |
| Text size | Layouts reflow at 200 % text (WCAG 1.4.4, 1.4.10): `AdaptivePair` stacks what no longer fits |
| Touch target and visual size | The touch area of at least 48 px reaches past a smaller visual (WCAG 2.5.8, §8.3) |
| Focus geometry | The ring outside the shape, drawn as an overlay, never clipped (§9) |
| Pixel snapping | Edges snapped to device pixels after the `unit` scale; a hairline is one device pixel; positions rounded with the error carried, so tiles never gap; nested radii: outer = inner + padding |
| Layout changes | Animated as transforms (FLIP), never a layout pass per frame (§8) |
| Scroll position | Restored on the way back; the focus kept in view (`FocusReveal`) |
| Empty, one, many | Each container defines its empty state (ADR 0003), one item, and a very large count, with bounded memory (T1-046, T1-047) |

**Avoid:** absolute positions for layout; a number for a space or a size; a component reading its own size to change it (a binding loop in Slint: read the size the parent gives); breakpoints by device name; a constraint solver; nested intrinsic passes; a layout pass per frame.

**Slint gaps** to report upstream, each with a minimal reproduction: a lazy grid (or virtualization in `FlexboxLayout` and `GridLayout`); horizontal `ListView` virtualization; baseline alignment; OpenType features (`tnum`); an `aspect-ratio` property; the allocated size without a binding loop (a `FlexboxLayout` whose direction follows a width panics with "Recursion detected", so a generic `AdaptiveRow` waits for it); sticky positions in `Flickable`; named grid areas and auto-fit tracks; overscroll and snap hooks (§7); nested scrolling: a `Flickable` takes any drag past its threshold from the controls inside it, and none can keep it (no "disallow intercept", no nested scroll connection), so a vertical list or a dial cannot be dragged inside a vertically scrolled page (the UI primitives activity's Spaces screen reproduces it). Focus (system input events §3.7): the Tab chain wraps around the window and cannot stop at a screen's ends, so `Screen` puts an invisible stop at each end and the runtime sends the opposite key back (`edge`); no context is inherited down a subtree, so a disabled `Group` cannot take its children out of the chain and each child takes its `enabled` explicitly; no way to focus the next or previous element from code.

### 6.4 Container and content

Every element is both: a box its container places, and a container of its own content. Three questions, answered separately and in this order, as CSS, Flutter and SwiftUI separate them: where the box goes (**positioning**), how big it is (**sizing**), and how its content sits inside it (**fit**, **alignment**, **overflow**).

**Positioning** (CSS's `position`, Flutter's `Stack`/`Positioned`):

| Model | Is | In Blackbird UI |
|---|---|---|
| Flow | The container's layout places it (the default) | Stacks and rows (§6); everything that is not one of the below |
| Relative | Placed by the flow, then offset without moving its neighbours | Only for motion (§8): an element entering moves at most 8 px toward its place |
| Absolute | Placed by its container's frame, outside the flow | Overlays inside one element: a badge, a focus ring, a knob on its track; `x` and `y` always both set |
| Anchored | Placed by another element, outside its container's clipping and order | `Overlay` (§9): a popover at its anchor, kept on screen |
| Fixed to a frame | Stays put while a container scrolls | A `Layer` above the scroll area (the header, the playing strip): declared outside the scroll content, never inside it with a counter-offset |
| Sticky | Flows, then stays at a container's edge once scrolled to it | A section label in a long list (to build on `Scroll`, §7) |

**Sizing** (Flutter's constraints, CSS's `min-`/`max-` and `fit-content`):

| Model | Is | Slint |
|---|---|---|
| Fill | Takes the space its container gives along an axis | A stretch factor (`horizontal-stretch: 1`), or no fixed size in a layout |
| Fit content (hug) | As big as its content, within min and max | Its preferred size; `horizontal-stretch: 0` |
| Fixed | A size from a token (a 48 px target) | `width`/`height` from `Tokens`; beware that it caps a stack (§6) |
| Bounded | Fill or fit, held within a minimum and a maximum | `min-*`/`max-*`; the touch minimum (§8.3) is always a minimum |
| Aspect | One side follows the other by a ratio (a cover: 1:1) | Height bound to width × ratio |

Constraints go down and sizes go up (§6): a container offers a range, the content picks its size in it, the container places it. A content's size never depends on where it is placed.

**Fit** of content that has its own shape (an image, a cover, a waveform) in a box of another shape (CSS's `object-fit`, Slint's `image-fit`):

| Fit | Does | Use |
|---|---|---|
| `contain` | Whole content, scaled to fit, letterboxed | A logo, a diagram, a HAT's photo: nothing may be cut |
| `cover` | Fills the box, scaled to cover, cut at the edges | Album art in a tile, a background: no bars |
| `fill` | Stretched to the box, aspect lost | Never for pictures; only for content without an aspect (a gradient, a meter's bar) |
| `none` (`preserve`) | At its own size, cut by the box | Pixel art and icons at their drawn size (Tabler at 24 px, design system §15.3) |
| `scale-down` | `none` unless too big, then `contain` | Third-party art of unknown size |

Album art uses `cover` in tiles and `contain` on Now Playing, so the whole art is seen where it is the subject (decided 2026-10-24).

**Alignment** of content within its box: logical (`start`, `center`, `end`, `stretch`), mirrored for right to left (§9.5); a cut content keeps its focal point (the centre by default).

**Overflow** of content bigger than its box, decided per axis:

| Overflow | Does | Use |
|---|---|---|
| Wrap | Text flows to more lines | Body text, descriptions |
| Elide | Text cut with an ellipsis, the full text in its accessible name and on focus | Titles in lines and headers |
| Clip | Cut at the box | Scroll content, modals' content; never a focus ring (§9) |
| Scroll | The box becomes a viewport over the content (§7, input events §6.2) | Screens taller than the display, rows of covers |
| Visible | Drawn past the box | Focus rings, halos, a knob's ring past its track |
| Scale | Text shrinks, never below the minimum size | Never: text stays at its role's size (§9) |

## 7. Scrolling

**Proven:** momentum with deceleration after a fling (iOS `decelerationRate`, Android `OverScroller`); an edge that answers when the content ends (iOS's rubber band, Android 12's stretch); scroll-snapping to items (CSS `scroll-snap`); nested scrolling that hands the motion to the parent at the end (CSS `overscroll-behavior`, Compose's nested scroll); the focused element kept in view; scroll indicators that show only while scrolling; scrolling to the top from the status bar (iOS).

- **Slint gives:** `Flickable` (panning, wheel and touchpad, deceleration after a flick, `content-x`/`content-y`, `flicked`); `ScrollView` adds scroll bars. No edge effect, no snapping, no nested hand-over.
- **Blackbird UI's `Scroll`** (to build on `Flickable`):
  - **Edge effect:** a stretch at the top and the bottom, not a rubber band that moves the content far past its end. The content stretches by at most 3 % along the scroll axis while the finger pulls past the edge, and settles back on release with a spring (`spring(0)`, critically damped: no overshoot). It says "this is the end" without making the screen wobble, which fits a restrained design (§2.2). With reduce motion, no stretch: a hairline at the edge, held 200 ms.
  - **Deceleration:** Slint's, kept.
  - **Snapping:** opt-in, to items (a cover grid, a picker), and to the start of a section after a fling.
  - **Nested:** an inner scroll hands the remaining motion to its parent at its end, horizontal inside vertical included (a row of covers inside a page).
  - **Focus kept in view:** `FocusReveal` (done), with the edge effect not triggered by focus moves.
  - **Indicator:** a 2 px bar at the end edge, shown while scrolling and 1 s after, for screens taller than two pages.
  - **To the top:** a tap on the status bar scrolls the screen on display to its top (iOS's convention).
  - **Buttons:** focus navigation scrolls (§8.4); there is no separate page-scroll binding.

### 7.1 Large content

Lists and grids of any size build only what the viewport needs, in bounded memory and time (T1-046, T1-047), from system's `opendap-layout` (platform-agnostic, with tests; tried in the UI primitives activity's Virtual screen: 100 000 rows, about 30 built, a few dozen slots):

| Primitive | Does | From |
|---|---|---|
| Extents, fixed or measured | Item sizes along the axis; measured ones correct an estimate in a Fenwick tree, so offsets and lookups are O(log n) at any count | RecyclerView, LazyColumn |
| Window | The items to build: the viewport, overscan on both sides, and prefetch ahead by velocity, capped | RecyclerView's prefetch, Flutter's cache extent |
| Scroll anchoring | What the user looks at stays put when items above change size, appear or go | CSS `overflow-anchor` |
| Recycling | A bounded set of slots, rebound by key as the window moves; never rebuilt, never more than the largest window | RecyclerView's view pool |
| Auto-fit grid | Columns by the space (`minmax`), rows windowed as a list (the cover grid) | CSS `auto-fit`, `GridCells.Adaptive` |
| Sticky headers | A section's header pinned while its section scrolls, pushed out by the next | CSS `position: sticky`, `stickyHeader` |

Slint's `ListView` virtualizes one vertical list of rows of one kind; these primitives cover the rest (grids, mixed rows, measured sizes, anchoring, horizontal rows) on top of a plain model, as `CoverGrid` and `Scroll` will.

### 7.2 Data on demand

What the window needs is loaded when it is needed, and let go when it is not (system's `opendap-layout::data`; planners only, which say what to fetch, cancel and evict, while the caller runs the fetches however it runs work, so they test with a virtual clock). Tried in the Virtual screen: pages that arrive 250 ms later, prefetch cancelled when a fling overtakes it, covers that wait for the scroll to slow, a cache that stays at its budget.

| Primitive | Does | Directives | From |
|---|---|---|---|
| Pages | Items a page (or a cursor's chunk) at a time: the visible pages first, then prefetch in the scroll's direction; placeholders of the right size until a page arrives; cancel what went far; evict past a budget; a cursor learns its end; a late answer for a cancelled page is ignored; a failed page shows an error in place and retries | `page_size`, `ahead`, `keep` | Android's Paging, iOS's prefetching data sources |
| Requests | One request per key in flight (asking twice joins it); generations, so an answer to a superseded request never overwrites the newer one | none | every data layer's lesson |
| Cache | Least recently used goes first, by cost (a decoded cover's bytes), within a budget | `budget` | image caches everywhere |
| Gate | The cheap (text, a page of rows) loads at once; the costly (decoding a cover, a waveform) only while the scroll is slow, so a fling keeps its 60 fps (T1-056) | `fast` (px/s) | iOS's and Android's deferred image loading |
| Stream | Results in parts (a search over the library): appended without moving what is shown, deduplicated by key, dropped when their query is no longer current | none | incremental search |

Proposed values (pages of 50, two ahead, twelve kept; covers decoded below 1500 px/s) are to measure on the T1.

### 7.3 Media

Images, icons, video, music, metadata and plots reach the screen or the output at the right size, in the right order, within a budget, on whatever the hardware offers (system's `opendap-media`, planners and pure math; the components and services run the decoding and the I/O). Tried in the Media screen: a million samples drawn in 144 columns with the peak kept; a 3000 px cover decoded at 96 px in a tile.

| Primitive | Does |
|---|---|
| Resource and variants | One identity, several renditions; the one for a target: the smallest that covers it (HTML's `srcset`) |
| Fit and decode size | `contain`, `cover`, `fill`, `none`, `scale-down` (§6.4): where content lands, the source part shown, and the size to decode at: the box's physical pixels, never more than the source |
| Decode queue | Visible, then prefetch, then background; bounded in parallel (audio comes first, T1-008), deduplicated, cancellable |
| Progressive chain | Dominant colour, thumbnail, full; a fling stops at the thumbnail |
| Tiers | Decoded in memory, encoded on disk, or to fetch; each tier its own budget |
| Audio buffering | Reads between watermarks in few long bursts (the storage sleeps: battery); the next track opened ahead, sample-exact, for gapless playback |
| Video pacing | The frame for each refresh; repeats and drops counted (3:2 pulldown on 60 Hz) |
| Plot decimation | Min and max per column, so every peak and clip shows; a ring buffer for live meters |
| Render path | The GPU where the hardware meets a feature's needs, software otherwise, each feature on its own (SYS-040) |

## 8. Motion

**Proven:** durations and easing curves as tokens (Material, the design system's `motion.*`); springs for anything a hand moves, since they carry the gesture's velocity and can be interrupted (SwiftUI, Compose, UIKit dynamics); enter and exit transitions for what appears and disappears (SwiftUI transitions, Framer's presence); layout animation of position changes (FLIP); shared-element transitions between screens; reduce motion that keeps the information and drops the movement.

- **Slint gives:** `animate` on any property (duration, delay, easing), easing curves including `cubic-bezier`, `ease-out-bounce` and `spring(bounce)`; `states` transitions.
- **Primitives** (system's `opendap-motion`, pure functions of time): springs solved in closed form, critically damped by default, carrying a gesture's release velocity; cubic-bezier curves solved exactly; a fling's decay and where it stops; an interruptible animator that retargets from where the value is, with its velocity, and lands at once with reduce motion; a frame budget that asks for frames only while something moves. Tried in the Media screen: a block thrown by the finger, or by Action, springs back; frames stop at rest.
- **Transition primitives** (system's `opendap-motion`, `opendap-layout`, `opendap-input`): a transition is one progress from 0 to 1, driven by time when the system starts it and by a hand when a gesture does, settling on the end a fling from where it is would reach, reversible and interruptible (`transition`); what moves with it is a function of that progress: numbers, a shared element's frame and corners, colours mixed in linear light, a container transform's fade through (`interpolate`); enter and exit keep what leaves drawn until it is gone (`presence`); pages settle one at most per fling and resist past their ends (`pager`); a shared element is paired by id in a namespace, from its source's frame to its destination's (`opendap-input::matched`). Tried in the UI primitives activity (Transitions): a pager by touch (a slow release settles on the nearest page, a fling moves one, rubber band at the ends) and by the buttons (Action locks it, Volume turns the pages), right to left too; a tile that morphs into its detail and back, by a tap, by Action, or by dragging the detail down (interactive, a flick decides); a line that leaves and comes back in its place. The transitions between activities (the composer moving buffers, composer spec §4) come with the OS Shell.
- **Rules:**
  - **Timed for what the system starts, springs for what a hand moves.** A screen change, a toast or a state change takes the motion tokens (`fast` 120 ms, `standard` 200 ms, `slow` 320 ms, the standard easing). A drag released, a sheet thrown or a knob let go settles with a critically damped spring from its velocity. `bounce` is never used for a routine screen (§2.2); `ease-out-bounce` is not used at all.
  - **Everything is interruptible:** a new input takes over from wherever the animation is, never waits for it to end.
  - **Enter and exit:** what appears fades and moves at most 8 dp toward its place; what leaves does the reverse, faster (`fast`).
  - **Navigation:** going deeper slides the new screen from the end edge; going back reverses it; the header's title cross-fades. A shared element (a cover to Now Playing) morphs between screens where both show it.
  - **Lists:** an inserted or removed line animates its height and the lines below move into place (layout animation), so nothing jumps.
  - **Reduce motion** (DS-012): durations become zero and springs settle at once; fades remain where they carry information, at `fast`.
  - **Budget:** 60 frames per second on the T1 (§17); only transforms and opacity animate on screens with many elements, never a layout pass per frame.

## 9. Layers and groups

**Proven:** z-order by named layers, never by numbers (Wayland's layer-shell: background, bottom, top, overlay; Android's window types; iOS's window levels); an overlay that escapes its parent's clipping and stacking wherever it is declared (the web's top layer for dialogs and popovers, Flutter's `Overlay`, Compose's `Popup`); one primitive that makes the rest inert, traps the focus and hides the rest from a screen reader (HTML's modal `dialog` with `inert`, iOS's `accessibilityViewIsModal`); groups that gather without creating a stacking context (a Figma group, against a frame that has an origin and constraints). Avoided: z-index wars and magic numbers; stacking contexts made by accident (CSS `opacity` and `transform` create them); invisible layers that still take touches; focus or a screen reader escaping a modal; drawing order and hit-testing order that differ (iOS's `zPosition` trap).

**System layers** (the composer's bands, bottom to top, where the OS Shell will draw its parts; composer spec §2 and §7): `background` → `activity` → `activity-overlay` (the activity's popovers and sheets, still its own surface) → `keyboard` → `system-ui` (status bar and panel, volume, toasts) → `system-view` (the power menu: the full screen, no client gets input) → `trusted` (consent, pairing) → `debug` (the blueprint references) → `touch-indicator`. The last two never take input. A system layer that takes input cancels the activity's contacts and releases its keys (system input events §5.1).

**Backdrop** (a primitive of the layer model; maintainers' direction, 2026-10-24: current patterns, frosted rather than darkened). A layer may treat what is under it: a blur (its radius), a tint (a colour and its strength) or nothing, with a fallback when reduce transparency is on. Built so far: system `opendap-media::blur` (shrunk, blurred once, about a Gaussian; 0.6 ms for the full screen on the virtual board) and Blackbird UI's `Backdrop` global, which the UI primitives activity's dialog tries; the composer will apply it live on the GPU (composer spec §7). Which layers use it, and with which values, is the components' design, not done yet.

**In an activity**, four bands, named, in this order: `content`, `raised` (a sticky header, the playing strip), `overlay` (popovers, menus), `modal` (a dialog and its scrim). Slint's `z` orders siblings only and must be a constant, so Blackbird UI's primitives place themselves in their band; no screen writes a `z`.

| Primitive | Is | z-order | Hit testing | Focus | Screen reader | Enabled | Coordinates | Blueprint |
|---|---|---|---|---|---|---|---|---|
| `Group` | Elements that belong together, with an optional title | none: children keep their order | none of its own; disabled, it takes the touches meant for its children | one traversal group | one node with its title | `enabled: false` dims everything in it to 45 % and takes their touches; `reason` shows once, under the title (ADR 0003) | its box is the children's local origin | its outline and title |
| `Layer` | A band of a screen (`raised`) | the band's | `hit: self`, `through` or `none` | normal | normal | normal | its own | the band's name |
| `Overlay` | A popover anchored to an element | the `overlay` band, wherever it is declared | its own box; a touch outside dismisses it (`dismiss: outside`) | enters it on open, returns to the anchor on close | announced, tied to its anchor | follows its anchor | the anchor's position, converted | an anchor line |
| `Modal` | A dialog: the rest inert | the `modal` band, above everything of the activity | the scrim takes every touch below | trapped inside, on its primary action or first element (focus on entry); back to where it was on close | the rest hidden (**UNVERIFIED**: no screen reader runs on the virtual board yet; to test with one) | everything below inert | the screen | the scrim hatched |

- **Effects on the rest.** A group's `enabled` cascades and cannot be undone by a child (§2). Events bubble through groups as through any element; a group may listen (input events §4). Opacity on a group is for fades only, never a way to hide: hidden is `visible: false` or `hit: none`, so nothing invisible takes touches. Motion enters and leaves a group as one (§8). Clipping belongs to scroll areas and modals' content only, so a focus ring is never clipped by a group.
- **Coordinates** follow the tree (system input events §6.1): every element with a box is a local origin; a group without a box is transparent; an overlay converts its anchor's position to the surface.

## 10. What Blackbird UI adds, in order

1. The `Motion` and `Layout` contexts (§2), read by every component.
2. `states` in every component for the interaction states (§5).
3. `Scroll`: edge stretch, snapping, nested hand-over, the indicator, to-the-top (§7).
4. Stack wrappers with spacing by proximity tokens (§6): `Stack`, `Row`, `Group`, so no screen writes a spacing number.
5. Enter, exit and navigation transitions (§8), and layout animation for lists.
6. `Group`, `Layer`, `Overlay`, `Modal` and `Scrim` (§9), with their effects on focus, touch, the screen reader and the blueprint.
7. `Cover` (art with its aspect and fit, §6.4), `AspectBox`, and sticky section labels in `Scroll`.
8. `Stack`, `Row`, `Wrap` with proximity tokens; `AdaptivePair` and the `Layout` context's layout class; `ColumnTrack`; `CoverGrid`; `ReadoutTable` (§6).

Each one shows in the UI primitives activity (system SYS-071) and in the blueprint mode (ADR 0002).

## Open questions

- Subtree context (a right-to-left island, §2): needed only if a screen mixes directions; Slint's globals are per window.
- The edge stretch's amount (3 %) and the indicator's threshold (two pages) are proposals to try on the T1.
