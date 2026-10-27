#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 The OpenDAP Project
# SPDX-License-Identifier: MPL-2.0
"""Pixel-grid alignment of the status bar's 16 px glyphs (icons README, Sizes).

At 16 px a unit of the 24 grid is 2/3 px, so Tabler's strokes on whole units land between pixels
(4 -> 2.67 px): a 1 px stroke spreads over two pixels, and the battery's first bar fuses with the
outline. Each status glyph is moved as a whole, never redrawn (ADR 0001): by the translation, in
grid units, that puts the most stroke length exactly on pixel centres (sharp 1 px lines), then,
among those, the least squared distance of the rest, weighted by length. Arcs, curves, diagonals
and dots (strokes shorter than one unit) do not take part; a glyph without straight axis-aligned
strokes is not moved. The move is at most half a pixel.

Run: python3 pixelgrid.py           the offset of each status glyph and its strokes' residuals, in px
     python3 pixelgrid.py --raster  each exported 16 px status glyph's ink per column and row, in
                                    percent of a full pixel (rendered with rsvg-convert, librsvg)
"""
import re

# Candidate translations, in grid units: eighths across one pixel period (1.5 units at 16 px).
STEP = 0.125
_TOKEN = re.compile(r"[A-Za-z]|-?(?:\d+\.?\d*|\.\d+)(?:e-?\d+)?")
_ARGS = {"M": 2, "L": 2, "H": 1, "V": 1, "C": 6, "S": 4, "Q": 4, "T": 2, "A": 7, "Z": 0}


def strokes(d):
    """The axis-aligned straight strokes of path data `d`: ("v", x, length) and ("h", y, length)."""
    tokens = _TOKEN.findall(d)
    out, i, cmd = [], 0, None
    x = y = start_x = start_y = 0.0
    while i < len(tokens):
        if tokens[i].isalpha():
            cmd = tokens[i]
            i += 1
            if cmd in "zZ":
                _line(out, x, y, start_x, start_y)
                x, y = start_x, start_y
                continue
        n = _ARGS[cmd.upper()]
        args = [float(t) for t in tokens[i:i + n]]
        i += n
        rel = cmd.islower()
        up = cmd.upper()
        if up == "H":
            nx, ny = (x + args[0] if rel else args[0]), y
        elif up == "V":
            nx, ny = x, (y + args[0] if rel else args[0])
        else:
            nx, ny = (x + args[-2], y + args[-1]) if rel else (args[-2], args[-1])
        if up in "LHV":
            _line(out, x, y, nx, ny)
        if up == "M":
            start_x, start_y = nx, ny
            # Further pairs after a moveto are linetos (SVG paths).
            cmd = "l" if rel else "L"
        x, y = nx, ny
    return out


# Shorter than this (grid units), a stroke is a dot, whose position does not sharpen a line.
MIN_LENGTH = 1.0


def _line(out, x0, y0, x1, y1):
    if x0 == x1 and abs(y1 - y0) >= MIN_LENGTH:
        out.append(("v", x0, abs(y1 - y0)))
    elif y0 == y1 and abs(x1 - x0) >= MIN_LENGTH:
        out.append(("h", y0, abs(x1 - x0)))


def residual(coord, size=16, grid=24):
    """How far, in px, a stroke centred at `coord` (grid units) is from the nearest pixel centre."""
    px = coord * size / grid
    return px - (round(px - 0.5) + 0.5)


def _best(coords, size, grid):
    if not coords:
        return 0.0
    period = grid / size
    steps = round(period / STEP)
    candidates = [(k - steps // 2) * STEP for k in range(steps)]

    def sharp(shift):
        return sum(w for c, w in coords if abs(residual(c + shift, size, grid)) < 1e-9)

    def cost(shift):
        return sum(w * residual(c + shift, size, grid) ** 2 for c, w in coords)

    # The most sharp length, then the least error, then the smaller move, then up or to the start.
    return min(candidates, key=lambda s: (-sharp(s), round(cost(s), 9), abs(s), s > 0))


def offset(paths, size=16, grid=24):
    """The translation (dx, dy), in grid units, that puts the glyph's straight strokes closest to
    pixel centres at `size` px."""
    found = [s for d in paths for s in strokes(d)]
    dx = _best([(c, w) for k, c, w in found if k == "v"], size, grid)
    dy = _best([(c, w) for k, c, w in found if k == "h"], size, grid)
    return dx, dy


def report():
    import icons

    _, _, status = icons.load_map()
    for name, _label in icons.STATUS:
        text = (icons.VENDOR / "outline" / f"{status[name]}.svg").read_text()
        paths = [d for d in re.findall(r'<path d="([^"]*)"', text) if d != "M0 0h24v24H0z"]
        dx, dy = offset(paths)
        found = [s for d in paths for s in strokes(d)]
        worst = max((abs(residual(c + (dx if k == "v" else dy))) for k, c, _ in found), default=0)
        print(f"{name:18} translate({dx:g} {dy:g})  strokes {len(found):2}  worst residual {worst:.3f} px")


def ink(svg_path, size=16):
    """Coverage per column and per row, in percent of a full pixel line, of `svg_path` at `size`."""
    import shutil
    import subprocess
    import zlib

    if not shutil.which("rsvg-convert"):
        raise SystemExit("--raster needs rsvg-convert (librsvg)")
    png = subprocess.run(["rsvg-convert", "-w", str(size), "-h", str(size), str(svg_path)],
                         check=True, capture_output=True).stdout
    alpha = _alpha(png, zlib)
    cols = [round(100 * sum(alpha[y][x] for y in range(size)) / 255) for x in range(size)]
    rows = [round(100 * sum(alpha[y]) / 255) for y in range(size)]
    return cols, rows


def _alpha(png, zlib):
    """The alpha channel of an 8-bit RGBA PNG, as rows (rsvg-convert's output)."""
    import struct

    pos, data, width = 8, b"", 0
    while pos < len(png):
        length, kind = struct.unpack(">I4s", png[pos:pos + 8])
        body = png[pos + 8:pos + 8 + length]
        if kind == b"IHDR":
            width, height, depth, colour = struct.unpack(">IIBB", body[:10])
            assert (depth, colour) == (8, 6), "RGBA 8-bit expected"
        elif kind == b"IDAT":
            data += body
        pos += 12 + length
    raw, stride, rows, prev = zlib.decompress(data), width * 4, [], bytearray(width * 4)
    for y in range(len(raw) // (stride + 1)):
        kind, line = raw[y * (stride + 1)], bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - 4] if i >= 4 else 0
            b, c = prev[i], prev[i - 4] if i >= 4 else 0
            if kind == 1:
                line[i] = (line[i] + a) & 255
            elif kind == 2:
                line[i] = (line[i] + b) & 255
            elif kind == 3:
                line[i] = (line[i] + (a + b) // 2) & 255
            elif kind == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(list(line[3::4]))
        prev = line
    return rows


def raster():
    import icons

    for name, _label in icons.STATUS:
        cols, rows = ink(icons.OUT / "16" / f"{name}.svg")
        print(f"{name:18} cols {' '.join(f'{c:3}' for c in cols)}")
        print(f"{'':18} rows {' '.join(f'{r:3}' for r in rows)}")


if __name__ == "__main__":
    import sys

    raster() if "--raster" in sys.argv else report()
