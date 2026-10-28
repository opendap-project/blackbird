# ADR 0002: The simulator window's display over QEMU's D-Bus display

- **Status:** Accepted
- **Date:** 2026-10-25
- **Basis:** the maintainers' report that transitions in the simulator window looked like jumps
- **Amends:** [ADR 0001](0001-simulator-window.md), "The display over VNC"

## Context

The simulator window drew the display from QEMU's VNC server. Motion measured on the virtual T1 on 2026-10-25 (toolkit-verify `odapkit motion`):

| Path | A 200 ms transition after an idle moment | Sustained motion |
|---|---|---|
| The guest's own display (QMP `screendump`, 2 ms a frame) | the whole transition, about 66 frames a second | about 66 frames a second |
| VNC, into the window | one jump: the last frame only | about 32 updates a second |
| QEMU's D-Bus display (`org.qemu.Display1`), into a listener | the whole transition | about 32 updates a second in a debug prototype; up to 60 in the window's release build |

QEMU's VNC server spaces its updates out while the display is idle and shortens the interval again only after changes keep coming, so a short transition after a still moment arrives as its last frame. The window then showed transitions as jumps, and anyone judging motion by eye in it judged something the device does not do.

QEMU's D-Bus display pushes each change to a registered listener as the guest draws it, with no back-off. It runs peer to peer (`-display dbus,p2p=yes`): no bus daemon, one socket pair handed to QEMU through QMP (`getfd`, `add_client`), the same way on macOS and Linux. It also carries the absolute pointer (`org.qemu.Display1.Mouse`). Homebrew's QEMU (11.1) includes it.

## Decision

1. **The window draws the display from QEMU's D-Bus display**, through `opendap-qemu-display` (viewer/crates): attached over the window's QMP socket at start (QMP serves one client at a time; the window's keys open their session afterwards), a listener on the first console that keeps the frame from `Scanout` and `Update`, and the pointer through `Mouse`. On the listener's connection QEMU is the server: the window's end is the client.
2. **QEMU starts with `-display dbus,p2p=yes`** for every virtual device; without a client it costs nothing. VNC stays on the loopback for other clients; the window no longer uses it, and `opendap-rfb` is removed.
3. **The window closes when QEMU closes the display**, as it did when the VNC connection ended.
4. **The window's own frame rate can be measured**: `ODAP_VIEW_FRAMES_LOG=FILE` writes one line per change it receives.

## Consequences

- Transitions show in the window as they run: measured, a 200 ms pager turn reaches the window as about 16 changes, where VNC gave one.
- The window is still not the reference for motion: QEMU composes its display on its own refresh, and the host's compositor adds its own. Motion is judged on the device's display with `odapkit motion --qmp` (toolkit-verify), and on a T1 when there is one.
- A QEMU built without the D-Bus display cannot open the window; `ODAP_WINDOW=qemu` (QEMU's own window) remains the fallback.
