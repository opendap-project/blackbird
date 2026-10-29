# Virtual board support package

What a Blackbird OS image adds and trims to run on the virtual board (system hardware support spec §5: one board support package per board; blackbird-os OS-004). It is a mkosi configuration directory: toolkit-image `odapkit image build qemu` includes it (`mkosi --include=export/bsp`).

## `mkosi.conf`

| Setting | Purpose |
|---|---|
| `ExtraTrees=tree` | Installs the files below |
| `KernelCommandLine=systemd.ssh_auto=no` | The board has no AF_VSOCK device; SSH is on the management link |
| `KernelModulesInitrdInclude=` | The initrd carries mkosi-initrd's default set and the drivers of the board's devices (virtio, fw_cfg, the microSD and USB controllers); every module stays in the root file system |
| `RemoveFiles=/usr/lib/modules/*/dtb` | QEMU builds the devicetree at run time, so no board's devicetree is needed |

## `tree/`

| Path | Purpose |
|---|---|
| `usr/lib/modules-load.d/opendap-virtual-board.conf` | Loads `qemu_fw_cfg`, the channel the virtual board uses to hand its device description to the guest |
| `usr/lib/systemd/system/blackbird-device-description.service` | Copies that description to `/run/blackbird/device/description.toml` at boot; enable it in the image |
| `usr/lib/systemd/network/10-virtual-management.network`, `10-virtual-lan.network` | The management link (MAC `52:54:00:0d:a1:00`, fallback route) and the LAN NIC that stands for Wi-Fi (MAC `52:54:00:0d:a1:01`, default route, mDNS) |
| `usr/lib/systemd/system/blackbird-composer.service.d/50-virtual-board.conf` | Turns the composer's touch indicator on (`BLACKBIRD_SHOW_TOUCHES=1`): a bubble that follows the window's pointer and every contact; the debug UI mode on by default (`BLACKBIRD_DEBUG_UI=1`); pulls in `blackbird-emulator-settings` before the composer |
| `usr/libexec/opendap/virtual-usb-device` | The USB-C port in device mode, on `dummy_hcd` (plug / unplug the cable) |
| `usr/libexec/opendap/virtual-settings` | The simulator's settings kept across reboots (debug UI mode, battery, charger, light): `set`, `get`, `restore`, in `/var/lib/blackbird/emulator/settings` |
| `usr/lib/systemd/system/blackbird-emulator-settings.service` | Puts those settings back at boot, before the composer; `odap-vdev-hw` reads the light itself |

The image carries no model: the running model's description comes from the virtual board, so one image runs every virtual model.

The board's own program, `odap-vdev-hw` (the faceplate's status LED, light sensor and microphone light), is not in the image: like the system's programs, it arrives as a development extension until packages carry them (`odapkit deploy` in `board/`). `odap-vdev led` and `odap-vdev light` say so when it is missing.
