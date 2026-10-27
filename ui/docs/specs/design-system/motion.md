# Engineered Matter — Motion

Part of the [Engineered Matter design system](../design-system.md): its status, scope and requirements apply here. Section numbers are the design system's, so a reference such as "design system §11" leads here.

## 11. Motion

Motion is deliberate, controlled and physically motivated.

### 11.1 Principles

Short transitions for direct interactions; smooth, restrained easing; no bounce; no perpetual decorative movement; no sudden full-screen flashes; no animation that delays access to primary controls. Every animation runs at the display's full frame rate (T1-056).

Durations (`motion.duration.fast` for press states and small toggles, `standard` for most transitions, `slow` for large surface changes) and the default easing (`motion.easing.standard`) are in the [token reference](../../reference/tokens.md#motion).

An animation never stands between an input and its effect: the state changes at once, and the animation shows it.

### 11.2 Plasma motion

When used, plasma movement suggests energy flow, not a looping screensaver: slow movement of the core, slight changes in the outer gradient, audio-reactive amplitude or intensity, a brief activation when playback starts, gentle settling when a process ends. No rapidly cycling colours and no high-frequency brightness changes.

### 11.3 Thermal motion

Extremely subtle and slow; atmospheric detail, never a source of information.

### 11.4 Reduced motion

Blackbird OS has a system **reduce motion** setting, and web surfaces honour `prefers-reduced-motion` (DS-012). Reduced motion replaces animation with a static state; it never removes the feedback the animation carried.
