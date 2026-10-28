# SPDX-FileCopyrightText: 2026 The OpenDAP Project
# SPDX-License-Identifier: GPL-3.0-or-later
"""The virtual T1 as an end-to-end scenario sees it (ADR 0014, level L3): its controls and touch in,
its display and journal out, through virtual-devices' own library over one QMP connection and one
SSH master (no process per key, no polling). Found through ODAP_VIRTUAL_DEVICES_DIR or the
workspace's repos.tsv (tokens.py)."""

import sys
import time
from pathlib import Path

import png
import tokens


sys.path.insert(0, str(tokens.repo("virtual-devices") / "lib"))
from odap_vdev import compose  # noqa: E402
from odap_vdev.instance import Instance  # noqa: E402
from odap_vdev.qmp import Qmp  # noqa: E402

# The focus ring's colour (design system: the focus token), as the display shows it.
RING = tokens.colour("focus")
# The screens of the UI primitives activity, in its list's order: the testbed's own table.
SCREENS_TSV = Path(__file__).resolve().parents[2] / "crates/odap-testbed/screens.tsv"
SCREENS = [
    line.split("\t", 1)[0]
    for line in SCREENS_TSV.read_text().splitlines()
    if line.strip() and not line.startswith("#")
]


class Device:
    def __init__(self, model="virtual-t1", out=Path("e2e-out")):
        self.inst = Instance(model)
        if not self.inst.pid():
            sys.exit(f"e2e: {model} is not running (odap-vdev start)")
        self.q = Qmp(self.inst.qmp_socket)
        self.controls = {k["role"]: k["qcode"] for k in compose.controls(self.inst.model) if k.get("qcode")}
        d = compose.display(self.inst.model) or {}
        self.size = (d["width_px"], d["height_px"])
        self.out = out
        out.mkdir(parents=True, exist_ok=True)

    # Input -----------------------------------------------------------------------------------

    def press(self, role, times=1, gap=0.25, hold=0.06):
        for _ in range(times):
            self.q.key(self.controls[role], True)
            time.sleep(hold)
            self.q.key(self.controls[role], False)
            time.sleep(gap)

    def hold(self, role, seconds):
        self.q.key(self.controls[role], True)
        time.sleep(seconds)
        self.q.key(self.controls[role], False)

    def hold_key(self, qcode, seconds):
        self.q.key(qcode, True)
        time.sleep(seconds)
        self.q.key(qcode, False)

    def keyboard(self, qcode, gap=0.3):
        self.q.key(qcode, True)
        self.q.key(qcode, False)
        time.sleep(gap)

    def tap(self, x, y, hold=0.05, after=0.3):
        w, h = self.size
        self.q.pointer(x, y, w, h)
        self.q.pointer(x, y, w, h, True)
        time.sleep(hold)
        self.q.pointer(x, y, w, h, False)
        time.sleep(after)

    def swipe(self, x1, y1, x2, y2, ms=200, after=0.4):
        w, h = self.size
        self.q.pointer(x1, y1, w, h)
        self.q.pointer(x1, y1, w, h, True)
        steps = max(1, ms // 16)
        for i in range(1, steps + 1):
            time.sleep(ms / 1000 / steps)
            self.q.pointer(x1 + (x2 - x1) * i // steps, y1 + (y2 - y1) * i // steps, w, h)
        self.q.pointer(x2, y2, w, h, False)
        time.sleep(after)

    # Output ----------------------------------------------------------------------------------

    def capture(self, name, settle=0.45):
        """The display now, saved as evidence: its pixels."""
        time.sleep(settle)
        path = (self.out / f"{name}.png").resolve()
        self.q.cmd("screendump", filename=str(path), format="png")
        return png.read(path)

    def ring(self, name, settle=0.45):
        """Where the focus ring is on the display, (x0, y0, x1, y1), or None when it is hidden."""
        return png.bounds(self.capture(name, settle), RING, tolerance=(14, 22, 26))

    def focused(self, name, settle=0.45):
        """The element the focus ring is on, as its pixels (png.inside), or None when it is hidden:
        compared with png.same, it says whether the focus is on the same element."""
        img = self.capture(name, settle)
        box = png.bounds(img, RING, tolerance=(14, 22, 26))
        return png.inside(img, box) if box else None

    def jump(self, to):
        """The debug jump: the long list's focus straight to `start`, `end` or an index (the UI
        primitives activity's /run/opendap/debug-jump), shown, the view there at once."""
        self.ssh(f"echo {to} > /run/opendap/debug-jump")
        time.sleep(0.6)

    def ssh(self, command):
        return self.inst.ssh(command, check=False).stdout

    # The activity ----------------------------------------------------------------------------

    def open_screen(self, screen):
        """From anywhere in the activity: back to its list, then the screen, by the buttons."""
        self.keyboard("esc")
        self.keyboard("esc")
        time.sleep(0.5)
        self.press("volume-up")  # shows the focus, on the list's first line
        self.press("volume-down", SCREENS.index(screen))
        self.press("action")
        time.sleep(0.8)
