# ADR 0006: Slint as the host renderer

- **Status:** Accepted
- **Date:** 2026-10-22

## Context

UI components are SDK-defined data that the host renders ([ADR 0005](0005-sdk-defined-services-components-and-schemas.md)), so the toolkit is an implementation detail of the host. Slint and Iced were compared as host renderers. Iced maps a data-defined tree to widgets more directly; Slint renders one by generating `.slint` source and compiling it with `slint-interpreter` (stable API). Slint has fine-grained property updates instead of rebuilding the view per message, declarative animations, AccessKit accessibility (upstream Iced has none), and a GLES-first renderer (FemtoVG), which suits the Mali-G52: Mesa's PanVK does not support Bifrost v7 by default, while its GLES 3.1 driver is conformant. Iced renders through wgpu, whose GLES backend is best effort. Slint's royalty-free licence excludes embedded systems, so the host uses it under GPLv3; with ADR 0005 no third party links the toolkit.

## Decision

- **Slint, under GPLv3**, renders the OS Shell's chrome and the SDK-defined components.
- It sits behind an OpenDAP renderer interface with conformance tests, so switching toolkits stays possible.
- Memory, idle wake-ups and energy are measured on the device before committing further.
