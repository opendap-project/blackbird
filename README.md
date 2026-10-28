# blackbird

| Folder | Content |
| --- | --- |
| [`emulator/`](emulator/README.md) | Virtual OpenDAP devices that run on a developer host with QEMU, as close to the hardware as QEMU allows |
| [`os/`](os/README.md) | Blackbird OS, the operating system of OpenDAP devices: the image definition, its profiles and releases |
| [`system/`](system/README.md) | Blackbird OS system software in Rust: bus, orchestrator, UI runtime, SDK and base services |
| [`ui/`](ui/README.md) | Blackbird UI, the user interface of Blackbird OS: **Engineered Matter**, the OpenDAP design system, with its tokens, OpenDAP Icons, guidelines and components ([spec](ui/docs/specs/design-system.md)), the templates the host renders, and the dynamic components contributed through the registry (including those from HATs) |

Each folder keeps its own `README.md`, `CLAUDE.md` and `REUSE.toml`; the licence texts are in `LICENSES/`. The folders came from the former `virtual-devices`, `blackbird-os`, `system` and `blackbird-ui` repositories with their history ([RFC 0017](https://github.com/opendap-project/rfcs/blob/main/text/0017-naming-and-topology.md)).

## License

This repository follows the [REUSE](https://reuse.software) layout: licence texts in `LICENSES/`, and each folder's `REUSE.toml` maps its paths. Copyright © 2026 The OpenDAP Project.
