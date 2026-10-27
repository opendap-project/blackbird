# Components

The UI components of the official design system are Slint files in [`export/slint/`](../export/slint/): `components.slint` (the controls of the design system §15) and `blueprint.slint` (their blueprint mode and its semantics). See [ADR 0002](../docs/adr/0002-slint-components-with-a-blueprint-mode.md).

`components.slint` is the index: it re-exports every component, so applications import from it only (`import { Button, … } from "@blackbird/slint/components.slint";`). The components themselves live in one file per family under [`export/slint/components/`](../export/slint/components/):

| File | Components |
| --- | --- |
| `foundation.slint` | `Motion`, `Layout`, `DirRow`, `Ink` |
| `focus.slint` | `FocusVisible`, `FocusReveal`, `FocusNav`, `FocusTarget`, `Adjustable`, `EndStop`, `TapArea` |
| `text.slint` | `Label`, `SectionLabel`, `Readout`, `EdgeLabel` |
| `layout.slint` | `Stack`, `Row`, `Wrap`, `AdaptivePair`, `AspectBox` |
| `surfaces.slint` | `Screen`, `Header`, `Panel`, `Group`, `Layer` |
| `buttons.slint` | `Button`, `PrimaryButton`, `Segment`, `IconButton`, `Tab` |
| `selection.slint` | `ListLine`, `Toggle`, `RadioStops`, `CheckLine` |
| `sliders.slint` | `Slider`, `RangeSlider`, `SeekBar` |
| `progress.slint` | `ProgressBar`, `LoadingLine`, `StatusIndicator` |
| `states.slint` | `Toast`, `EmptyState`, `ErrorState` |
| `overlays.slint` | `Overlay`, `Backdrop`, `Scrim`, `Modal` |

Each family file imports exactly what it uses from `../tokens.slint`, `../blueprint.slint` and the other families. Imports between families never form a cycle: `foundation` imports no family, `focus` and `text` import only `foundation`, and the rest build on those (`layout` and `progress` on `text`, `states` on `buttons`, `overlays` on `surfaces`). `DirRow` sits in `foundation` because `text` and `layout` both build on it.
