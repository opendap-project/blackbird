# Engineered Matter — Performance and platform

Part of the [Engineered Matter design system](../design-system.md): its status, scope and requirements apply here. Section numbers are the design system's, so a reference such as "design system §17" leads here.

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
