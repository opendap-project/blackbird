# Virtual devices requirements

Requirements for the virtual OpenDAP devices that run on a developer host.

| ID | Requirement | Priority | Status |
|---|---|---|---|
| VD-001 | A virtual device shall mimic its hardware as closely as possible: display, controls, touch, audio output, battery and charging, storage, microSD, USB in host and device mode, networking, and the coprocessor and HAT presence where they can be emulated. | must | Confirmed |
| VD-002 | A virtual device shall work with the `opendap` CLI and OpenDAP Desktop running on the host, as the hardware would. | must | Confirmed |
| VD-003 | A virtual device shall have network access, to the local network and to the internet. | must | Confirmed |
| VD-004 | Virtual devices shall live in their own repository, separate from the toolkit and from the operating system. | must | Confirmed |
| VD-005 | A virtual device shall be organised by its hardware: a combination of hardware layers (SoC, board, components, faceplate, HATs), with the device model defined on top of that combination. | must | Confirmed |
| VD-006 | Everything that lets a device be emulated on the host during development shall be provided, for every requirement, as far as possible; what cannot be emulated is shown by the toolkit (cli TK-018). | must | Confirmed |
| VD-007 | The window of a virtual device shall always open at the right size, and keep it while the device boots. | must | Confirmed |
| VD-008 | The simulator window shall instrument the virtual device's sound on demand: what it plays and what it records, as level meters, a spectrum and a sine's distortion (THD, THD+N, SNR), measured on the stream exactly as the device produced it, so the audio path's audiophile qualities can be audited. | must | Confirmed |
| VD-009 | The guest's recorded input shall be selectable among every input of the host, a test signal and silence. | must | Confirmed |
| VD-010 | Instrumentation should cover the hardware's signals, not only the operating system's view: each hardware layer should declare the signals it carries (the stream from the SoC to the DAC, the DAC's output, the microphone's converter, buses, GPIOs, LEDs, power rails), and each one a layer models should be observable in the simulator under the same name a probe uses on the hardware. | should | Draft |
| VD-011 | The simulator window should offer a task manager for the virtual device: every process with its state, CPU and memory, sorted by any of them, and the usual actions on one (end, kill, pause, resume, priority). | should | Confirmed |

## Context

- VD-001 to VD-003 were blackbird-os OS-018, OS-033 and OS-034 (moved with VD-004).
- VD-004 to VD-007. VD-005 applies the system hardware support spec's layers (H0 to H5) to virtual hardware: one image runs every virtual model, because the model's description comes from the virtual board (fw_cfg), as a real board is identified by its hardware.
- VD-007: "the right size" is the panel's pixels at 1:1, so the picture stays crisp; on a 34-inch 3440 × 1440 monitor that is already larger than the T1's panel (about 74 mm against 49 mm wide).
- VD-008 to VD-011: debugging sound in the simulator with the same analysis used for motion, above all the output, to audit its audiophile qualities; the inputs selectable among all the host's sources once a real input is bridged; instrumentation of any signal, the hardware's as well as the system's (for example the music that leaves the SoC for the DAC by its own interface); a task manager as its own tab, as on Windows or a typical Linux. How the sound reaches the window: [ADR 0003](../adr/0003-sound-over-dbus.md).
- An audio file as an input source (the guest records a file played into its microphone) was raised as an idea; not built.
- Specification: [virtual devices](../specs/virtual-devices.md).
