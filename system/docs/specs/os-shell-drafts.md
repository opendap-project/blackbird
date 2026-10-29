# OS Shell drafts

> The OS Shell is the system's own interface on top of the composer, not designed yet ([RFC 0012](https://github.com/opendap-project/rfcs/blob/main/text/0012-composer-and-os-shell.md)). Until it is, the composer draws a few of its parts itself, as drafts that shape the primitives: the status bar, the power menu and toasts. This spec states what those drafts do; the composer that hosts them is in the [composer and UI runtime spec](composer-and-ui-runtime.md).

- **Status:** Draft, 2026-10-27. Split out of the composer and UI runtime spec, unchanged.
- **Where they live:** `blackbird-composer`'s `ui/shell/` (`status-bar.slint`, `power-menu.slint`, `toast.slint`), drawn by its system layers (`ui/layers.slint`); what they decide is platform-agnostic and tested on any host (`src/status_bar.rs`, `src/power_menu.rs`, `src/combinations.rs`).

## 1. The status bar

The status bar is the composer's system layer, its one owner: drawn above the top 28 px (`size-status-bar`) of every activity and under system views (SYS-069). Activities, the product host included, draw no bar of their own and keep that band clear for it. Basis: [ADR 0012](../adr/0012-shell-process-model-and-system-layers.md), SYS-038, SYS-040, SYS-012, DS-017, design-system §10.1 and the icons section.

- **Items**, in visual order. At the start: the time and the microSD card. At the end: Bluetooth, Wi-Fi, then the battery's percentage and its icon. The volume item, a speaker icon and the level in the semantic safe and warning colours (SYS-012), joins the end of the start group, after the microSD card. It is drawn once the audio engine (`/usr/lib/blackbird/audio`) provides the level and its safe or warning state; the unit and the thresholds are an open item of SYS-038.
- **Spacing** from the token scale (design system §10.1, DS-017): the groups sit on the 16 px side margins (`space-margin`); items are `space-2` (8 px) apart; the parts of one item (the percentage and the battery icon, the speaker and the level) are `space-unit` (4 px) apart. Text is the caption size in IBM Plex Mono, in `text-secondary`.
- **Icons:** the status set as drawn, the same glyphs with a 1 px stroke at 16 px (`size-icon-status`, Blackbird UI's `icons/16/status-*`), never the 24 px glyphs scaled down.
- **Radios:** one the hardware lacks is not shown (SYS-040); one that is off is drawn at the design system's Disabled opacity (`opacity-disabled`); one that is on, as every other item.
- **Battery:** five levels, the nearest of 0, 25, 50, 75 and 100 % (`status-battery-0` to `-4`); while charging the battery shows its bolt inside (`status-charging`), and the percentage gives the level. At 15 % or less, discharging, the icon is amber (`state-warning`), and a toast warns once at 15 % and once at 5 % (SYS-076: Android's defaults, the status LED's thresholds).
- **Accessibility:** one label per item, read in visual order, stating its state: "Time 10:42", "microSD card", "Bluetooth on", "Wi-Fi off", "Battery 50 %, charging", "Battery 10 %, low".
- **State:** it reads the kernel's classes (power_supply, block devices of type SD, rfkill, wireless network devices) on their uevents and the clock at each minute, so it costs one wakeup a minute.

## 2. The power menu

The power menu is the first system layer: Slint (`ui/shell/power-menu.slint`, Blackbird UI's tokens and fonts) drawn by the software renderer into a buffer the compositor places above every client, with glyphs embedded at build time. It follows the mockup "Power, beside Main":

- the choices beside `main`, at the height the device description gives it;
- Power off, then Restart, then Restart to recovery only if the device has that mode;
- Cancel at the bottom.
- The choices run the full width, without dividers, their labels at the right, beside Main.
- It is a system view (SYS-069): it precedes the OS Shell and takes the whole screen, with no status bar or other OS Shell layer showing.
- **Buttons alone** (SYS-039, as recovery menus work). The menu opens with its first choice focused (design system §8.4, focus on entry); Volume + and − move the visible focus (a `focus` border) through the choices and Cancel, wrapping around. Action takes the focused choice. Main powers off.
- **Confirmation.** Pressing `main` again powers off. Touching "Power off" only points at that press ([composer and UI runtime spec](composer-and-ui-runtime.md) §7: power requires it). Restart and recovery are touch choices. While the layer is up, no client receives touch or the pointer.
- **The power service.** The composer's user cannot power off or pick a boot mode, so it asks blackbird-os's `blackbird-power.socket` (root, the `odap` group only, `odap-reboot --serve`). It needs no polkit, and stands in until the orchestrator's `cc.opendap.Blackbird.Power`. No request blocks the event loop (T1-047): power-off and reboot are sent from a thread of their own, and the boot modes are fetched once when the composer starts and answered from that cache, so a power menu opened before the service has answered shows no recovery entry.

## 3. Toasts

A short bar that leaves by itself, with no input (user-facing conventions: Material's snackbar): 48 px high, inside the side margins, 12 px above the bottom, in `surface-raised` with the body text. The composer shows one for a screenshot ("Screenshot saved", 2 s) and for each battery threshold crossed (4 s).
