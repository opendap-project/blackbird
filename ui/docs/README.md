# Documentation

| Folder | Contents |
|---|---|
| [`requirements/`](requirements/) | What this project must achieve. Each requirement has an ID, a priority (**must** / **should**) and a status. Their source is [`requirements.toml`](requirements/requirements.toml); the README is rendered from it with `odapkit req render`. |
| [`specs/`](specs/) | Specifications: the [design system](specs/design-system.md), an index of its topic specs in [`specs/design-system/`](specs/design-system/), and the [UI model](specs/ui-model.md). |
| [`reference/`](reference/) | Generated, never edited by hand: the [token reference](reference/tokens.md), written by `tokens/generate.py`. |
| [`guides/`](guides/) | How-to by audience: for developers, the [design review checklist](guides/developers/design-review-checklist.md). |
| [`adr/`](adr/) | Decisions the maintainers have taken: [0001](adr/0001-tabler-icons-as-the-base-family.md) Tabler Icons as the base family of OpenDAP Icons; [0002](adr/0002-slint-components-with-a-blueprint-mode.md) Slint components, each with a blueprint mode; [0003](adr/0003-screen-states.md) empty, loading, error and disabled-with-reason states. |

Only information confirmed by the maintainers is recorded here. Earlier experiments are not a source.

**Requirement status:** *Confirmed* (agreed, build on it) · *Draft* (agreed intent, wording or detail still open).
