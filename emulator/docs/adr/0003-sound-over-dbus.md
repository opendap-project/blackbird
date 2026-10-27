# ADR 0003: The virtual device's sound over QEMU's D-Bus audio

- **Status:** Accepted
- **Date:** 2026-10-26 (amended 2026-10-27: the QEMU OpenDAP builds, decision 8)
- **Basis:** VD-008 to VD-011 (debugging sound in the simulator, above all auditing the output's audiophile qualities)
- **Amends:** [ADR 0002](0002-display-over-dbus.md), "the window attaches to QEMU's D-Bus display"

## Context

The virtual T1's sound card (`virtio-sound-pci`, one output and one input stream) went to the host's audio through QEMU's CoreAudio or PulseAudio backend. Nothing between the guest and the speakers could be measured, and QEMU 11's CoreAudio backend has no input, so on macOS the guest could not record at all.

QEMU's D-Bus display carries the sound too (`-audiodev dbus`, interfaces `org.qemu.Display1.Audio`, `AudioOutListener`, `AudioInListener`): a listener receives each output stream's format and its PCM, and supplies each input stream's PCM. Measured on the virtual T1 on 2026-10-26 (`odap-vdev-audio`, random full-range references played with `aplay` on the sound card and compared sample by sample):

| Path | 16-bit mono 48 kHz | 16-bit stereo 44.1 kHz | 32-bit stereo 96 kHz |
|---|---|---|---|
| Output, `out.mixing-engine=off` | bit-exact | bit-exact | bit-exact |
| Output through QEMU's mixing engine (`fixed-settings=off`) | +6.02 dB: the engine copies a mono sample to both channels and sums them back | bit-exact | not tested |

| Path | Result |
|---|---|
| Input, `in.mixing-engine=off` | QEMU calls the listener's `Read` in a tight loop for a few bytes each (its rate control is disabled in `dbusaudio.c`); the guest's capture overruns and records nothing |
| Input through QEMU's mixing engine (`in.fixed-settings=off`) | paced by QEMU's audio timer; stereo arrives bit for bit; mono arrives at twice its value (the same sum) |

Measured on 2026-10-26 with the output's mixing engine off: QEMU sends the output in blocks of 480 frames, and a stream's last partial block (its length modulo 480) is not sent when the stream stops but at the start of the next stream, before that stream's own sound: each stream reaches the listener whole but late, and the next one begins with up to 10 ms of the previous one's end (218 and 454 frames in two runs; the body then bit-exact).

Measured on 2026-10-26 in 5- to 20-minute runs (`tests/sound-soak`) with the host loaded by other work:

- With QEMU's audio timer at its default 10 ms, the input is pulled one 480-frame block a tick; the ticks stretched and the input reached the guest at about 73 % of real time, until the guest's capture failed. At 5 ms it is pulled at real time.
- In some runs, after 1 to 10 minutes, the guest's playback and capture both stopped within milliseconds with an I/O error, while QEMU's audio backend went on pulling the input at real time: QEMU's virtio-sound stopped completing the guest's buffers (ALSA's own timeout then ends both streams). Not reproduced in quiet-host runs of up to 10 minutes. **UNVERIFIED:** the cause inside QEMU.

QEMU's D-Bus display serves one client peer to peer: a second `add_client` displaces the first (measured: the window closed when a recorder attached).

## Decision

1. **The sound goes through the D-Bus display** whenever the window is the D-Bus display (`-display dbus,p2p=yes,audiodev=snd0`, `-audiodev dbus,out.mixing-engine=off,in.fixed-settings=off`); with QEMU's own window it goes to the host's audio as before.
2. **The output bypasses QEMU's mixing engine**, so what the listener receives is the guest's PCM, unconverted, in the guest's own rate, depth and channels.
3. **The input goes through the mixing engine**, which paces it. *Amended 2026-10-27: with the QEMU OpenDAP builds (decision 8) the input skips the mixing engine too, and the rest of this point applies only to a stock QEMU 11.1.* A mono input stream is given half its value, so the guest records the level it was given; a 16-bit mono stream then keeps 15 bits (its THD+N measures 6 dB above a stereo one). The window's source paces itself by the clock as well (what is due since the stream started), whatever QEMU asks.
4. **QEMU's audio timer ticks every 5 ms** (`timer-period=5000`), so the input keeps up with real time on a loaded host.
5. **The window never makes QEMU wait:** the input's `Read` is answered from what is due and its samples are handed to an analysis thread through a bounded channel (dropped for the analysis when it falls behind, never for the guest); opening the host's audio never waits under the lock the listeners take.
6. **The window is the device's only D-Bus client.** It plays the output on the host (the guest's own rate and channels when the host takes them; otherwise resampled, for listening only, and it says so), supplies the input (any of the host's inputs, a 997 Hz test tone at -6 dBFS, or silence) and measures both (`opendap-audio-analysis` from system: meters, spectrum, THD, THD+N, SNR) on the exact streams. Other tools ask it over its control socket (`view.sock`, beside the device's state) instead of attaching to QEMU: `odap-vdev-audio record` records either direction's exact bytes through it, and attaches to QEMU itself only when no window runs.
7. **`odap-vdev-audio`** records, analyses (per channel, a sine's analysis, block by block for continuity), compares bit for bit and draws spectra; `tests/sound-soak` plays and records for minutes and checks continuity, exactness and memory.

8. **OpenDAP carries three patches to QEMU's audio and builds QEMU with them** (amended 2026-10-27). They are kept in toolkit-host (`qemu/patches/`), and `odapkit qemu install` builds QEMU 11.1.2's pinned source release with them. With that build:
   - a mono round trip through the mixing engine is no longer doubled (`clip_*_from_mono` averages the channels);
   - the input without the mixing engine is paced by the rate control, in whole blocks;
   - a stream's last partial block is sent when the stream stops.

   `odap-vdev` recognises the build by its version line and then runs both directions without the mixing engine (`dbus,out.mixing-engine=off,in.mixing-engine=off,timer-period=5000`). It tells the window, which then stops halving a mono input. A stock QEMU keeps the configuration of decisions 2 to 4.

## Consequences

- Measured on 2026-10-27, the same guest and window on stock QEMU 11.1.2 and on the patched build (`odap-vdev-audio`, `tests/sound-soak`):

| Test | Stock | Patched |
|---|---|---|
| Mono 16-bit 48 kHz output through the mixing engine | every sample x2.0 | bit-exact (96 000 frames) |
| Input without the mixing engine, 10 s | overruns, I/O error, nothing recorded | 10 s recorded, no overrun |
| 5 minutes, no mixing engine either way | the input fails | output 14 400 000 frames bit for bit; input 300 one-second blocks at the same THD+N (-92.92 dB), bit for bit against what the window gave |
| Two streams of 96 999 and 72 999 frames (1000-frame periods) | the first misses its last 39 frames, the second begins with 40 of them | both whole, nothing leading |

- Measured on 2026-10-26 with `tests/sound-soak --minutes 10` (window measuring both directions): the output carried 28 800 000 of 28 800 000 frames bit for bit; the input, 552 one-second blocks of the test tone with the same THD+N (-92.92 dB) and no zero run, and the guest's file carried what the window gave it bit for bit; the window's resident memory rose 8 MiB in the first five minutes, held flat after and fell when the streams ended, QEMU's held flat. **UNVERIFIED:** the guest's capture stopped at 552 s of 600 in that run (not reproduced in shorter runs).

- The output can be audited as the device's software produced it: bit-perfect transport, levels, distortion and noise, in any format the guest opens.
- The guest records on macOS too, from any of the host's inputs or from a known test signal.
- The sound needs the window: a device started without it plays nothing (the D-Bus listener is the only sink).
- QEMU waits for the window's answer to every input read, so the window must get the CPU on time (amended 2026-10-27). Its app bundle opts out of App Nap (`LSAppNapIsDisabled`), and the threads that answer QEMU run at the highest QoS (`QOS_CLASS_USER_INTERACTIVE`).
  - Before this, a window behind others on a loaded host went 4.5 s without answering; QEMU stopped and the guest's playback and capture failed together within seconds to a minute.
  - After it, measured on 2026-10-27, the input ran a full 5 minutes. Stalls of 1 to 1.5 s remain when the host's load stays above its number of cores (load 10 to 14 on 14 cores, from other work): the guest's default 0.5 s ALSA buffers then overrun and both streams can stop.
  - At moderate load, 5-minute runs carried both directions bit for bit. **UNVERIFIED:** which of QEMU's threads waits during those stalls.
- The patches are OpenDAP's to carry: every QEMU update needs them to apply and the measurements above repeated (`odapkit qemu install` fails when a patch does not apply). With a stock QEMU the limits above stay. The held-back last block cannot be told apart from the next stream by the listener; `odap-vdev-audio compare` finds a reference after such leading sound and reports it, and `tests/sound-soak` reports it as this known limit, apart from the transport's exactness.
