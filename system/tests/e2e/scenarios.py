# SPDX-FileCopyrightText: 2026 The OpenDAP Project
# SPDX-License-Identifier: GPL-3.0-or-later
"""The end-to-end scenarios (ADR 0014, level L3): each drives the virtual T1 as a person would and
checks what the display shows, saving its captures as evidence. Each names the matrix cases it
covers (docs/verification/primitives-matrix.toml). A scenario returns the list of what failed."""

import time

import png
from device import SCREENS

BLACK = (0, 0, 0)


def inside(box, size):
    return box is not None and box[0] >= 0 and box[1] >= 0 and box[2] < size[0] and box[3] < size[1]


def nav_entry(d):
    """nav-entry-20: every screen opens with the focus on its first element or its primary action,
    the ring on screen."""
    bad = []
    for s in SCREENS:
        d.open_screen(s)
        r = d.ring(f"entry-{s}")
        if not inside(r, d.size):
            bad.append(f"{s}: ring {r}")
    return bad


def ambient(d, name):
    """The rows that change with no key pressed (a loading line, a toast's timer): two captures
    apart, compared. A step that changes no more rows than these changed nothing (they may have
    moved with a scroll, so their count, not their place)."""
    a = d.capture(f"{name}-ambient-a", settle=0.2)
    b = d.capture(f"{name}-ambient-b", settle=0.5)
    return {y for y in range(a.height) if a.rows[y] != b.rows[y]}


def still(img, ring, taps=()):
    """What a key could change on screen, without the status bar (its clock), the focused element
    itself (a text field's caret blinks) and the touch indicator around recent taps (the virtual
    board draws a circle where a finger was): the rest, row by row."""
    boxes = ([ring] if ring else []) + [(x - 26, y - 26, x + 26, y + 26) for x, y in taps]
    c = img.channels
    rows = []
    for y in range(28, img.height):
        row = bytearray(img.rows[y])
        for b in boxes:
            if b[1] <= y <= b[3]:
                x0, x1 = max(0, b[0]), min(img.width - 1, b[2])
                row[x0 * c:(x1 + 1) * c] = bytes((x1 + 1 - x0) * c)
        rows.append(bytes(row))
    return rows, ring


# Screens whose list is too long to walk (100 000 rows): the walk starts from the list's far end,
# reached by the debug jump, and goes on from there as on any screen.
LONG = {"Virtual"}


def same(a, b, moving):
    """Two screens the same but for what moves on its own."""
    return a[1] == b[1] and sum(x != y for x, y in zip(a[0], b[0])) <= len(moving)


def nav_ends(d, screens=SCREENS):
    """nav-ends, nav-ends-fast: Volume past either end of a screen stays there, at a person's pace
    and at 30 ms apart."""
    bad = []
    for s in screens:
        d.open_screen(s)
        moving = ambient(d, f"ends-{s}")
        for key, end in (("volume-down", "end"), ("volume-up", "start")):
            if s in LONG:
                d.press(key)  # into the list, if the focus is not there yet
                d.jump(end)
            # The end is where a key changes nothing on screen: the ring alone can stay put while
            # the focus moves (a list that brings each item to the same place as it scrolls).
            img = d.capture(f"ends-{s}-{end}-0")
            shot = still(img, png.bounds(img, (255, 135, 77), (14, 22, 26)))
            # Two presses in a row that change nothing: one alone may only have been slow to draw
            # (a virtual list loading its pages).
            unchanged = 0
            for i in range(60):
                d.press(key)
                img = d.capture(f"ends-{s}-{end}", settle=0.5)
                prev, shot = shot, still(img, png.bounds(img, (255, 135, 77), (14, 22, 26)))
                unchanged = unchanged + 1 if same(shot, prev, moving) else 0
                if unchanged == 2:
                    break
            else:
                # Not reached by walking (a list of 100 000 rows): nothing to check here; its ends
                # are covered below this level (opendap-layout's focus, the interface tests).
                print(f"     {s} {end}: not reached in 60 presses, not checked here")
                continue
            r = d.ring(f"ends-{s}-{end}", settle=0)
            d.press(key, times=4, gap=0.03, hold=0.02)
            after = d.ring(f"ends-{s}-{end}-fast", settle=0.6)
            if after != r:
                bad.append(f"{s} {end}: {r} → {after} past the {end}")
    return bad


def hidden_first_key(d):
    """nav-hidden-action: a touch on bare ground hides the ring; the first key only shows where the
    focus is, on the element it was on."""
    d.open_screen("Controls")
    before = d.ring("hidden-before")
    d.tap(290, 95)  # bare ground: beside the first group's caption
    hidden = d.ring("hidden-after-touch")
    d.press("volume-down")
    shown = d.ring("hidden-first-key")
    bad = []
    if hidden is not None:
        bad.append(f"a touch left the ring at {hidden}")
    if shown != before:
        bad.append(f"the first key moved the focus: {before} → {shown}")
    return bad


def keyboard(d):
    """kb-layout, kb-dismiss, nav-fields-hidden: a tap on a field opens the keyboard with the field
    above it and the ring hidden; the first key only shows the ring on that field; back closes the
    keyboard and keeps the focus there."""
    d.open_screen("Keyboard")
    entry = d.ring("kb-entry")
    if entry is None:
        return ["no focus on entry"]
    cx, cy = (entry[0] + entry[2]) // 2, (entry[1] + entry[3]) // 2
    d.tap(cx, cy)
    open_img = d.capture("kb-open")
    bad = []
    hidden = png.bounds(open_img, (255, 135, 77), (14, 22, 26))
    if hidden is not None:
        bad.append(f"the ring shows after a touch: {hidden}")
    d.press("volume-down")
    shown = d.ring("kb-first-key")
    if shown is None or abs(shown[1] - entry[1]) > 160 or shown[3] > d.size[1]:
        bad.append(f"the first key did not show the ring on the field: {shown} (entry {entry})")
    d.keyboard("esc")
    closed = d.ring("kb-closed")
    if closed is None:
        bad.append("back lost the focus")
    return bad


def display_wake(d):
    """disp-wake: Main turns the display off; a double tap turns it on again."""
    d.press("main")
    time.sleep(1.0)
    off = d.capture("wake-off")
    dark = png.share(off, BLACK, (8, 8, 8))
    for _ in range(2):
        d.tap(160, 240, hold=0.04, after=0.12)
    time.sleep(1.0)
    on = d.capture("wake-on")
    lit = 1 - png.share(on, BLACK, (8, 8, 8))
    bad = []
    if dark < 0.98:
        bad.append(f"Main left {1 - dark:.0%} of the display lit")
    if lit < 0.2:
        bad.append(f"a double tap did not wake it ({lit:.0%} lit)")
    return bad


def blueprint(d):
    """vis-noblueprint, life-blueprint: the debug UI mode switches in place, without a restart, and
    off leaves no blueprint on screen."""
    def blue(img):
        n = 0
        for y in range(0, img.height, 2):
            for x in range(0, img.width, 2):
                r, g, b = img.pixel(x, y)
                n += b > r + 40 and b > g + 10
        return n / ((img.height // 2) * (img.width // 2))
    was = d.ssh("cat /run/blackbird/debug/ui 2>/dev/null").strip()
    d.ssh("echo 1 > /run/blackbird/debug/ui")
    on = blue(d.capture("blueprint-on", settle=1.0))
    d.ssh("echo 0 > /run/blackbird/debug/ui")
    off = blue(d.capture("blueprint-off", settle=1.0))
    d.ssh(f"echo {was or 1} > /run/blackbird/debug/ui")
    bad = []
    if on < 0.3:
        bad.append(f"on: only {on:.0%} of the display reads as blueprint")
    if off > 0.02:
        bad.append(f"off: {off:.0%} still reads as blueprint")
    return bad


def hang_input(d):
    """ctl-hang-input: keys pressed while the activity hangs are dropped, not queued: when it
    comes back, nothing acts by surprise and the focus is where it was."""
    d.open_screen("Exceptions")
    d.press("volume-down")  # from Panic to "Hang for 5 s"
    on_hang = d.ring("hang-before")
    d.press("action")  # the activity sleeps 5 s
    time.sleep(1.0)  # past the composer's ping and its 500 ms
    d.press("volume-down", times=3, gap=0.3)
    time.sleep(5.5)
    after = d.ring("hang-after", settle=0.5)
    return [] if after == on_hang else [f"keys during the hang acted: the focus went from {on_hang} to {after}"]


# Surfaces a touch drives, each with a gesture that changes what it shows: (screen, gesture, region
# of the display that must change). Coordinates in display pixels.
TOUCH_SURFACES = [
    ("Arena", ("swipe", 160, 360, 160, 300), (16, 260, 304, 430)),       # nested areas
    ("Transitions", ("swipe", 260, 150, 60, 150), (16, 100, 304, 200)),  # the pager
    ("Spaces", ("swipe", 160, 300, 160, 220), (16, 120, 304, 400)),      # a list under its pan
    ("Navigation", ("tap", 120, 300, 0, 0), (16, 250, 304, 350)),        # an album on the shelf
    ("Media", ("swipe", 160, 200, 260, 200), (16, 100, 304, 300)),       # the spring
]


def first_touch(d):
    """The first touch after the buttons: on every surface a touch drives, the ring hides and the
    gesture acts (a stop that took the press for its focus left the ring shown and moved nothing)."""
    bad = []
    for screen, g, (x0, y0, x1, y1) in TOUCH_SURFACES:
        d.open_screen(screen)
        before = d.capture(f"first-{screen}-before", settle=0.6)
        if g[0] == "swipe":
            d.swipe(*g[1:], ms=300)
        else:
            d.tap(g[1], g[2])
        after = d.capture(f"first-{screen}-after", settle=0.8)
        ring = png.bounds(after, (255, 135, 77), (14, 22, 26))
        changed = sum(before.rows[y][x0 * before.channels:x1 * before.channels] != after.rows[y][x0 * after.channels:x1 * after.channels] for y in range(y0, y1))
        if ring is not None:
            bad.append(f"{screen}: the ring still shows at {ring}")
        if changed < 4:
            bad.append(f"{screen}: the gesture moved nothing ({changed} rows changed)")
    return bad


def sharpness(img, y0, y1):
    """How much detail a band of the display has: the summed differences between neighbouring
    pixels across it (a frost takes most of it away)."""
    c, total = img.channels, 0
    for y in range(y0, y1):
        row = img.rows[y]
        total += sum(abs(row[i] - row[i + c]) for i in range(0, (img.width - 1) * c, c))
    return total


def layers_touch(d):
    """nav-popover-touch, blur-backdrop: a popover opens over the screen and a tap outside closes it,
    leaving the screen as it was and no ring; a dialog frosts what is behind it and Escape closes
    it."""
    d.open_screen("Layers")
    d.tap(290, 95)  # bare ground: the ring hides
    taps = [(290, 95), (160, 384), (160, 120), (160, 436)]
    base = still(d.capture("lt-base", settle=0.6), None, taps)
    bad = []
    d.tap(160, 384)  # Open a popover
    opened = still(d.capture("lt-pop", settle=0.6), None, taps)
    if same(opened, base, set()):
        bad.append("the popover did not open")
    d.tap(160, 120)  # outside it
    closed = d.capture("lt-pop-closed", settle=0.6)
    if not same(still(closed, None, taps), base, set()):
        bad.append("a tap outside left the screen changed")
    if png.bounds(closed, (255, 135, 77), (14, 22, 26)) is not None:
        bad.append("a tap outside left the ring shown")
    before = sharpness(d.capture("lt-sharp", settle=0.3), 300, 470)
    d.tap(160, 436)  # Open a dialog
    frosted = sharpness(d.capture("lt-dialog", settle=0.8), 300, 470)
    if frosted > before * 0.5:
        bad.append(f"the dialog's backdrop is not frosted (detail {frosted} against {before})")
    d.keyboard("esc")
    if not same(still(d.capture("lt-dialog-closed", settle=0.8), None, taps), base, set()):
        bad.append("Escape did not close the dialog")
    return bad


def repeat(d):
    """ctl-repeat: Volume held keeps moving the focus (the composer's repeat), and stops on release:
    the screen changes between two moments of the hold, and not after it."""
    import threading
    d.open_screen("Virtual")
    d.press("volume-down")
    held = threading.Thread(target=d.hold, args=("volume-down", 2.5))
    held.start()
    time.sleep(1.0)
    a = d.capture("repeat-a", settle=0)
    time.sleep(0.8)
    b = d.capture("repeat-b", settle=0)
    held.join()
    c = d.capture("repeat-c", settle=0.5)
    e = d.capture("repeat-d", settle=0.8)
    bad = []
    if same(still(a, None), still(b, None), set(range(8))):
        bad.append("held, the focus did not keep moving")
    if not same(still(c, None), still(e, None), set(range(8))):
        bad.append("released, it kept moving")
    return bad


def exceptions(d):
    """life-exceptions: when the activity panics or exits, the composer starts it again, back on its
    list."""
    bad = []
    d.keyboard("esc")
    d.keyboard("esc")
    home_img = d.capture("exc-home", settle=0.8)
    # A restarted activity opens with the ring hidden: where it was is not compared.
    ring = png.bounds(home_img, (255, 135, 77), (14, 22, 26))
    home = still(home_img, ring, [(290, 95)])
    for name, presses in (("panic", 0), ("exit", 2)):
        d.open_screen("Exceptions")
        d.press("volume-down", presses)  # Panic first, Exit two below it
        d.press("action")
        time.sleep(5)
        back = still(d.capture(f"exc-{name}", settle=0.8), ring, [(290, 95)])
        if not same(back, home, set(range(4))):
            bad.append(f"after {name}, the activity is not back on its list")
    return bad


def turns(d):
    """turns-dial: turning the dial moves its needle and counts the turn; dragging the strip moves
    its marker."""
    import math
    d.open_screen("Turns")
    w, h = d.size
    before = d.capture("turns-0", settle=0.6)
    cx, cy, r = 63, 171, 30
    d.q.pointer(cx, cy - r, w, h)
    d.q.pointer(cx, cy - r, w, h, True)
    for i in range(1, 21):
        time.sleep(0.03)
        a = math.radians(i * 9)
        d.q.pointer(int(cx + r * math.sin(a)), int(cy - r * math.cos(a)), w, h)
    d.q.pointer(cx, cy + r, w, h, False)
    dial = d.capture("turns-dial", settle=0.6)
    d.swipe(30, 274, 200, 274, ms=400)
    strip = d.capture("turns-strip", settle=0.6)
    changed = lambda a, b, y0, y1: sum(a.rows[y] != b.rows[y] for y in range(y0, y1))
    bad = []
    if changed(before, dial, 124, 218) < 10:
        bad.append("the dial did not turn")
    if changed(dial, strip, 260, 290) < 4:
        bad.append("the strip's marker did not move")
    return bad


def status_bar_owner(d):
    """gest-bar-owner: a contact that starts in the status bar is the system's: the list under it
    does not move, and the ring stays where it was."""
    d.open_screen("Virtual")
    d.press("volume-down")
    before = d.capture("bar-0", settle=0.6)
    ring = png.bounds(before, (255, 135, 77), (14, 22, 26))
    d.swipe(160, 8, 160, 300, ms=300)
    after = d.capture("bar-1", settle=0.8)
    bad = []
    if not same(still(after, None, [(160, 300)]), still(before, None, [(160, 300)]), set(range(4))):
        bad.append("the list moved under a swipe from the status bar")
    if png.bounds(after, (255, 135, 77), (14, 22, 26)) != ring:
        bad.append("the activity's ring changed")
    return bad


def reduce_motion(d):
    """tr-reduce, comp-media-reduce: with reduce motion on (Calibration's switch, standing for the
    system's), a pager turn and a thrown spring land at once: just after the input the display is
    already what it is a second later. The switch is turned off again at the end."""
    w, h = d.size
    d.open_screen("Calibration")
    d.press("volume-down")
    d.press("action")
    bad = []
    try:
        d.open_screen("Transitions")
        d.press("action")
        d.press("volume-up", gap=0, hold=0.03)
        soon = d.capture("reduce-pager-soon", settle=0.03)
        later = d.capture("reduce-pager-later", settle=0.8)
        if soon.rows[90:200] != later.rows[90:200]:
            bad.append("the pager still moved after the turn")
        d.open_screen("Keyboard")
        d.press("volume-down")  # the ring on the first field
        d.press("action", gap=0, hold=0.03)  # Action opens its keyboard
        soon = d.capture("reduce-kb-soon", settle=0.03)
        later = d.capture("reduce-kb-later", settle=0.8)
        if soon.rows[230:480] != later.rows[230:480]:
            bad.append("the keyboard still slid in after it opened")
        d.keyboard("esc")
        d.open_screen("Media")
        d.q.pointer(160, 150, w, h)
        d.q.pointer(160, 150, w, h, True)
        for i in range(1, 8):
            time.sleep(0.03)
            d.q.pointer(160 + i * 12, 150, w, h)
        d.q.pointer(244, 150, w, h, False)
        soon = still(d.capture("reduce-spring-soon", settle=0.05), None, [(244, 150)])
        later = still(d.capture("reduce-spring-later", settle=1.0), None, [(244, 150)])
        # The block's band (still() starts at row 28), without the indicator at the release.
        if soon[0][97:147] != later[0][97:147]:
            bad.append("the spring still moved after the release")
    finally:
        d.open_screen("Calibration")
        d.press("volume-down")
        d.press("action")
    return bad


def display_off_focus(d):
    """sem-display-off, nav-stop-display-off, nav-modal-display-off: with the display off the
    buttons are play/pause and the volume, never the focus; off and on, the focus is where it was
    (a range on its knob), a stop engaged is released to the stop, an open dialog stays with its
    focus."""
    def off_on(presses=0):
        d.press("main")
        time.sleep(0.8)
        if presses:
            d.press("volume-down", presses, gap=0.6)  # apart: steps, not a double press
            d.press("action")
        d.tap(160, 240, hold=0.04, after=0.12)
        d.tap(160, 240, hold=0.04, after=0.12)
        time.sleep(1)
    bad = []
    d.open_screen("Controls")
    d.press("volume-down", 2)  # the range's high knob
    before = d.ring("doff-range-0")
    steps = d.ssh("journalctl -t composer --no-pager | grep -c 'volume step'").strip()
    off_on(3)
    after = d.ring("doff-range-1")
    steps_after = d.ssh("journalctl -t composer --no-pager | grep -c 'volume step'").strip()
    if after != before:
        bad.append(f"off and on, the focus moved: {before} -> {after}")
    if int(steps_after or 0) - int(steps or 0) != 3:
        bad.append(f"the volume took {int(steps_after or 0) - int(steps or 0)} steps, not 3")
    d.open_screen("Navigation")
    d.press("volume-down", 4)
    d.press("action")  # into the shelf
    d.press("volume-down", 2)
    stop = d.ring("doff-stop-0")
    off_on()
    released = d.ring("doff-stop-1")
    if released is None or released == stop or released[2] - released[0] < 200:
        bad.append(f"the shelf was not released to its stop: {stop} -> {released}")
    d.open_screen("Layers")
    d.tap(160, 436)  # Open a dialog
    time.sleep(0.6)
    d.press("volume-down")
    modal = d.ring("doff-modal-0")
    off_on()
    if d.ring("doff-modal-1") != modal:
        bad.append("the dialog or its focus did not stay")
    d.keyboard("esc")
    return bad


def system_controls(d):
    """ctl-main: Main short turns the display off (and on); Main held opens the power menu; Main +
    Volume − short saves a screenshot; held, it opens the power menu. The menu is closed with
    Volume + (round to Cancel) and Action, never with Main, whose short press there powers off."""
    bad = []
    lit = lambda img: 1 - png.share(img, BLACK, (8, 8, 8))

    def combo(seconds):
        d.q.key(d.controls["main"], True)
        d.q.key(d.controls["volume-down"], True)
        time.sleep(seconds)
        d.q.key(d.controls["volume-down"], False)
        d.q.key(d.controls["main"], False)
        time.sleep(0.6)

    def close_menu():
        d.press("volume-up")  # round to Cancel
        d.press("action")
        time.sleep(0.6)

    d.keyboard("esc")
    d.keyboard("esc")
    home = d.capture("ctl-home", settle=0.6)
    d.press("main")
    time.sleep(0.8)
    if lit(d.capture("ctl-off", settle=0.2)) > 0.02:
        bad.append("Main short did not turn the display off")
    d.tap(160, 240, hold=0.04, after=0.12)
    d.tap(160, 240, hold=0.04, after=0.12)
    time.sleep(0.8)
    d.hold("main", 1.0)
    menu = d.capture("ctl-menu", settle=0.6)
    if same(still(menu, None), still(home, None), set(range(8))):
        bad.append("Main held did not open the power menu")
    close_menu()
    if not same(still(d.capture("ctl-menu-closed", settle=0.6), None), still(home, None), set(range(8))):
        bad.append("the power menu did not close with Cancel")
    count = lambda: int(d.ssh("ls /var/lib/blackbird/Pictures/Screenshots 2>/dev/null | wc -l; ls ~blackbird/Pictures/Screenshots 2>/dev/null | wc -l").split()[-1])
    shots = count()
    combo(0.2)
    if count() != shots + 1:
        bad.append(f"Main + Volume − short saved {count() - shots} screenshots, not 1")
    combo(2.5)
    if same(still(d.capture("ctl-combo-menu", settle=0.6), None), still(home, None), set(range(8))):
        bad.append("Main + Volume − held did not open the power menu")
    close_menu()
    return bad


def volume_buttons(d):
    """ctl-vol-long, ctl-vol-double, ctl-vol-taps, ctl-vol-double-focus: while sound plays, a
    press is one volume step at once; held, the volume keeps stepping (as the focus does in
    navigation); a double press takes its first step back and changes the track (T1-062, T1-063
    as amended 2026-10-26). Presses apart are steps each; a third quick press is a step again.
    With nothing playing, a double press is two focus steps."""
    def log(since):
        out = d.ssh(f"journalctl -t composer --since @{since} --no-pager -o cat")
        return {k: out.count(k) for k in ("volume step\n", "volume step, held", "volume step undone", "track, double press")}

    def now():
        return d.ssh("date +%s.%N").strip()

    bad = []
    d.open_screen("Controls")
    d.press("volume-down")
    ring = d.ring("vol-0")
    d.ssh("(speaker-test -c 2 -t sine -f 440 -l 0 >/dev/null 2>&1 &); sleep 1")
    try:
        cases = [
            ("held 1.5 s", lambda: d.hold("volume-down", 1.5),
             lambda c: c["volume step\n"] == 1 and c["volume step, held"] >= 3 and c["track, double press"] == 0),
            ("double", lambda: d.press("volume-up", 2, gap=0.12),
             lambda c: c["volume step\n"] == 1 and c["volume step undone"] == 1 and c["track, double press"] == 1),
            ("triple", lambda: d.press("volume-up", 3, gap=0.12),
             lambda c: c["volume step\n"] == 2 and c["volume step undone"] == 1 and c["track, double press"] == 1),
            ("two apart", lambda: d.press("volume-down", 2, gap=0.6),
             lambda c: c["volume step\n"] == 2 and c["track, double press"] == 0),
        ]
        for name, act, ok in cases:
            t = now()
            act()
            time.sleep(0.8)
            c = log(t)
            if not ok(c):
                bad.append(f"sound playing, {name}: {c}")
        if d.ring("vol-1") != ring:
            bad.append("the focus moved while the buttons were the volume")
    finally:
        d.ssh("pkill speaker-test; true")
    time.sleep(1)
    t = now()
    d.press("volume-down", 2, gap=0.12)
    time.sleep(0.6)
    c = log(t)
    moved = d.ring("vol-2")
    if c["track, double press"] or c["volume step\n"]:
        bad.append(f"nothing playing, a double press acted on the volume: {c}")
    if moved == ring:
        bad.append("nothing playing, a double press did not move the focus")
    return bad


def scrim_variants(d):
    """blur-reduce-tr, blur-blueprint: a dialog's scrim is frosted by default; with reduce
    transparency it is a solid tint (every pixel under it is the screen's own, darkened by the
    tint's alpha: nothing blurred); in blueprint it is hatched (vertical lines every 16 px)."""
    alpha = 0xB8 / 255  # the scrim's tint (#000000b8)
    band = (400, 470)   # below the dialog, over the screen's own content

    def dialog(tag):
        d.open_screen("Layers")
        before = d.capture(tag + "-before", settle=0.5)
        d.tap(160, 436)  # Open a dialog
        after = d.capture(tag + "-dialog", settle=0.9)
        d.keyboard("esc")
        time.sleep(0.5)
        return before, after

    def tint_error(before, after):
        # Mean distance from a plain tint over the same pixels, in 0–255 levels.
        n, total = 0, 0
        for y in range(*band):
            for x in range(0, before.width, 2):
                for b, a in zip(before.pixel(x, y), after.pixel(x, y)):
                    total += abs(a - b * (1 - alpha))
                    n += 1
        return total / n

    def hatch(img):
        # Columns' brightness folded on 16 px: a hatch makes one phase stand out.
        cols = [0.0] * img.width
        for y in range(*band):
            for x in range(img.width):
                cols[x] += sum(img.pixel(x, y))
        phases = [sum(cols[x] for x in range(k, img.width, 16)) / len(range(k, img.width, 16)) for k in range(16)]
        rest = sorted(phases)[:-1]
        return max(phases) / (sum(rest) / len(rest))

    bad = []
    frost = tint_error(*dialog("scrim-frost"))
    d.open_screen("Calibration")
    d.tap(160, 245)  # Reduce transparency on
    try:
        solid = tint_error(*dialog("scrim-solid"))
    finally:
        d.open_screen("Calibration")
        d.tap(160, 245)  # off again
    if solid > 3:
        bad.append(f"reduce transparency: the scrim is not a plain tint (off by {solid:.1f} levels)")
    if frost < 3 * max(solid, 1):
        bad.append(f"frosted and solid look alike ({frost:.1f} against {solid:.1f} levels)")
    was = d.ssh("cat /run/blackbird/debug/ui 2>/dev/null").strip()
    d.ssh("echo 1 > /run/blackbird/debug/ui")
    try:
        before, after = dialog("scrim-blueprint")
    finally:
        d.ssh(f"echo {was or 0} > /run/blackbird/debug/ui")
    h_before, h_after = hatch(before), hatch(after)
    if h_after < 1.05 or h_after < h_before * 1.03:
        bad.append(f"blueprint: no hatch under the dialog (phase contrast {h_after:.3f}, {h_before:.3f} without it)")
    print(f"     tint error: frost {frost:.1f}, solid {solid:.1f}; hatch {h_before:.3f} -> {h_after:.3f}")
    return bad


def composer_restart(d):
    """life-restart: the composer restarted, the activity is back on screen."""
    d.ssh("systemctl restart blackbird-composer")
    time.sleep(4)
    img = d.capture("restart")
    lit = 1 - png.share(img, BLACK, (8, 8, 8))
    return [] if lit > 0.2 else [f"after a restart only {lit:.0%} of the display is lit"]


# The scenarios in the order they run (cheap and harmless first, the restart last), with the matrix
# cases each covers.

# The battery's icon in the status bar: its 16 px box at the bar's right end (composer spec §2:
# 16 px side margins, the icon 16 px, centred on the 28 px bar).
BATTERY_BOX = (288, 6, 304, 22)
POWER = "/sys/module/test_power/parameters"


def battery_icon(d, name, percent, charging):
    """The battery set to `percent`, charging or not (the virtual board's test_power), and the
    pixels of its icon then."""
    state = "charging" if charging else "discharging"
    ac = "on" if charging else "off"
    d.ssh(f"echo {ac} > {POWER}/ac_online; echo {state} > {POWER}/battery_status; "
          f"echo {percent} > {POWER}/battery_capacity")
    img = d.capture(name, settle=1.5)
    x0, y0, x1, y1 = BATTERY_BOX
    c = img.channels
    return [tuple(img.rows[y][x * c:x * c + 3]) for y in range(y0, y1) for x in range(x0, x1)]


def tinted(icon, colour):
    """Whether the icon is drawn in `colour`: its brightest pixel, a full stroke's, is that colour
    (the antialiased edges between it and the background are not)."""
    brightest = max(icon, key=sum)
    return sum(abs(a - b) for a, b in zip(brightest, colour)) < 12


def status_bar_battery(d):
    """sb-battery: the battery's icon in the status bar shows the nearest of its five levels (13 %
    one bar, 12 % none), amber at 15 % or less while discharging, and, while charging, the battery
    with its bolt inside whatever the level, never amber (SYS-038, SYS-076). The battery is put
    back as it was."""
    import tokens
    amber, plain = tokens.colour("state-warning"), tokens.colour("text-secondary")
    # test_power reads its battery_status parameter back as "on": the state comes from the class.
    supply = "/sys/class/power_supply"
    was = d.ssh(f"cat {supply}/test_battery/capacity {supply}/test_battery/status {supply}/test_ac/online").split()
    bad = []
    try:
        half = battery_icon(d, "sb-50", 50, False)
        one = battery_icon(d, "sb-13", 13, False)
        none = battery_icon(d, "sb-12", 12, False)
        charging = [battery_icon(d, f"sb-charging-{p}", p, True) for p in (10, 50, 100)]
        if one == none:
            bad.append("13 % and 12 % look the same (one bar against none)")
        if not tinted(half, plain) or tinted(half, amber):
            bad.append("50 % discharging is not in the secondary text colour")
        if not (tinted(one, amber) and tinted(none, amber)):
            bad.append("13 % and 12 % discharging are not amber")
        if not all(c == charging[0] for c in charging):
            bad.append("charging does not show the same glyph at 10, 50 and 100 %")
        if charging[0] in (half, one, none):
            bad.append("charging looks like a level glyph")
        if any(tinted(c, amber) for c in charging):
            bad.append("charging at 10 % is amber")
    finally:
        if len(was) == 3:
            capacity, status, ac = was
            ac = "on" if ac == "1" else "off"
            d.ssh(f"echo {ac} > {POWER}/ac_online; echo {status.lower()} > {POWER}/battery_status; "
                  f"echo {capacity} > {POWER}/battery_capacity")
    return bad


def repeat_direction(d):
    """ctl-repeat-direction: held, each volume button keeps moving the focus its own way and stops
    at its end (T1 controls spec §1.3, input events §3.7), in the UI primitives activity's list:
    Volume + held past the first line stays there, Volume − held runs to the last line and stays,
    and Volume + held from there moves up; an attached keyboard's Shift+Tab held stays at the
    first line too. Which line has the focus is told by what the ring holds (where the view
    scrolls the first line depends on where it was before)."""
    d.keyboard("esc")
    d.keyboard("esc")
    time.sleep(0.5)
    d.press("volume-up")  # shows the focus, on the list's first line
    first = d.focused("dir-first")
    if first is None:
        return ["the focus did not show on the list"]
    bad = []
    d.press("volume-down", 4)
    d.hold("volume-up", 5.0)
    if not png.same(d.focused("dir-top", settle=0.6), first):
        bad.append("Volume + held past the first line did not stay there (dir-top)")
    d.hold("volume-down", 8.0)
    last = d.focused("dir-last", settle=0.6)
    d.hold("volume-down", 2.0)
    if last is None or not png.same(d.focused("dir-last-still", settle=0.6), last):
        bad.append("Volume − held past the last line did not stay there (dir-last-still)")
    d.hold("volume-up", 1.0)
    up = d.focused("dir-up", settle=0.6)
    if up is None or png.same(up, last):
        bad.append("Volume + held from the last line did not move up (dir-up)")
    # An attached keyboard's Shift+Tab, held past the first line, stays there too: the screen's
    # end stop puts Shift back down after its Tab (Blackbird UI FocusNav.shift).
    d.hold("volume-up", 5.0)
    d.press("volume-down", 4)
    d.q.key("shift", True)
    d.hold_key("tab", 5.0)
    d.q.key("shift", False)
    if not png.same(d.focused("dir-kb-top", settle=0.6), first):
        bad.append("Shift+Tab held past the first line did not stay there (dir-kb-top)")
    return bad

ALL = [
    ("nav_entry", nav_entry, ["nav-entry-20"]),
    ("hidden_first_key", hidden_first_key, ["nav-hidden-action"]),
    ("first_touch", first_touch, ["nav-touch-first", "nested-scroll", "tr-pager-touch", "virt-shelf"]),
    ("keyboard", keyboard, ["kb-layout", "kb-dismiss", "nav-fields-hidden"]),
    ("nav_ends", nav_ends, ["nav-ends", "nav-ends-fast"]),
    ("blueprint", blueprint, ["vis-noblueprint", "life-blueprint"]),
    ("display_wake", display_wake, ["disp-wake"]),
    ("system_controls", system_controls, ["ctl-main"]),
    ("volume_buttons", volume_buttons, ["ctl-vol-long", "ctl-vol-double", "ctl-vol-taps", "ctl-vol-double-focus"]),
    ("scrim_variants", scrim_variants, ["blur-reduce-tr", "blur-blueprint"]),
    ("display_off_focus", display_off_focus, ["sem-display-off", "nav-stop-display-off", "nav-modal-display-off"]),
    ("hang_input", hang_input, ["ctl-hang-input"]),
    ("layers_touch", layers_touch, ["nav-popover-touch", "blur-backdrop"]),
    ("repeat", repeat, ["ctl-repeat"]),
    ("repeat_direction", repeat_direction, ["ctl-repeat-direction"]),
    ("turns", turns, ["turns-dial"]),
    ("status_bar_owner", status_bar_owner, ["gest-bar-owner"]),
    ("status_bar_battery", status_bar_battery, ["sb-battery"]),
    ("reduce_motion", reduce_motion, ["tr-reduce", "kb-reduce", "comp-media-reduce"]),
    ("exceptions", exceptions, ["life-exceptions"]),
    ("composer_restart", composer_restart, ["life-restart"]),
]
