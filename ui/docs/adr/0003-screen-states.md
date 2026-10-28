# ADR 0003: Empty, loading, error and disabled-with-reason states

- **Status:** Accepted
- **Date:** 2026-10-23

## Context

- No document defined how a screen or a template looks when it has nothing to show, while it loads, when it fails, or how a control that cannot act says why. The requirements need them: disabled with its reason (system SYS-040, os-architecture §4.4), active, disabled, loading and error told apart (§13), errors different from loading and long operations with progress and cancel (§8.4), failures with "a reason and next step" (devices-and-sync spec §6), never colour alone (DS-010).
- The design system is restrained: no illustrations on routine screens, plain words, one primary action per screen (§15.2).

## Decision

| State | What it shows | Rules |
|---|---|---|
| **Empty** | A muted 32 px icon of the thing, one line saying what is empty (`body-large`), one line saying how to fill it (`body`, `text-secondary`), and at most one action that does it | Left-aligned at the side margin, a section step under the header. The action is the screen's primary action only if filling it is the screen's purpose |
| **Loading** | The screen's structure at once (header, tabs, rows' places); then, past 300 ms, a 2 px indeterminate line under the header (`state-info`); past 1 s, "Loading…" where the content goes | Nothing for waits under 300 ms (no flash). A known length shows a determinate bar with its value, and Cancel where it can be cancelled. Motion follows reduce-motion: the line holds still and the words remain |
| **Error** | The `state-error` icon, one sentence with what failed and why, the next step, and one action that takes it (Try again, or the fix) | Never colour alone: icon and words. It replaces the content, so it never looks like loading. A failure inside a list marks that line only |
| **Disabled with its reason** | The control at 45 % opacity, its reason as a caption under it (`caption`, `text-secondary`) | The reason is always visible, never a tooltip (a finger has no hover). The control keeps its place; navigation skips it, and a disabled group's controls with it (amended by the maintainers, 2026-10-24: the reason is always visible, and a screen reader reads it in its own reading order, so the buttons are not spent on what does nothing). Accessible description: the reason |

## Consequences

- Blackbird UI's components carry them: `EmptyState`, `LoadingLine` (indeterminate or with a value), `ErrorState`, and a `reason` on every interactive component.
- The templates of slots (os-architecture §6.3) use the same states.
- In the blueprint mode (ADR 0002) the states keep their meaning: loading in `state-info`, error in `state-error`, a disabled control's area drawn dashed-looking at low opacity with its reason.
