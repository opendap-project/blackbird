# Documentation

| Folder | Contents |
|---|---|
| [`requirements/`](requirements/) | What this project must achieve. Each requirement has an ID, a priority (**must** / **should**) and a status. Their source is [`requirements.toml`](requirements/requirements.toml); the README is rendered from it with `odapkit req render`. |
| [`specs/`](specs/) | Specifications: [virtual devices](specs/virtual-devices.md). |
| [`adr/`](adr/) | Decisions: [ADR 0001](adr/0001-simulator-window.md), the simulator window; [ADR 0002](adr/0002-display-over-dbus.md), its display over QEMU's D-Bus display; [ADR 0003](adr/0003-sound-over-dbus.md), the device's sound over QEMU's D-Bus audio. |

Only information confirmed by the maintainers is recorded here.
