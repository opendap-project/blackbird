# SPDX-FileCopyrightText: 2026 The OpenDAP Project
# SPDX-License-Identifier: GPL-3.0-or-later
"""Just enough PNG for the display's captures (8-bit RGB or RGBA, not interlaced), with the
standard library only, and where a colour sits in it."""

import struct
import zlib


class Image:
    def __init__(self, width, height, channels, rows):
        self.width, self.height, self.channels, self.rows = width, height, channels, rows

    def pixel(self, x, y):
        i = x * self.channels
        return tuple(self.rows[y][i:i + 3])


def read(path):
    data = open(path, "rb").read()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"{path}: not a PNG")
    pos, idat, header = 8, b"", None
    while pos < len(data):
        length, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + length]
        if kind == b"IHDR":
            header = struct.unpack(">IIBBBBB", body)
        elif kind == b"IDAT":
            idat += body
        pos += 12 + length
    width, height, depth, colour, _, _, interlace = header
    if depth != 8 or colour not in (2, 6) or interlace:
        raise ValueError(f"{path}: only 8-bit RGB or RGBA, not interlaced")
    channels = 3 if colour == 2 else 4
    raw = zlib.decompress(idat)
    stride = width * channels
    rows, prev = [], bytearray(stride)
    for y in range(height):
        start = y * (stride + 1)
        kind, line = raw[start], bytearray(raw[start + 1:start + 1 + stride])
        for i in range(stride):
            a = line[i - channels] if i >= channels else 0
            b = prev[i]
            c = prev[i - channels] if i >= channels else 0
            if kind == 1:
                line[i] = (line[i] + a) & 0xFF
            elif kind == 2:
                line[i] = (line[i] + b) & 0xFF
            elif kind == 3:
                line[i] = (line[i] + (a + b) // 2) & 0xFF
            elif kind == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 0xFF
        rows.append(line)
        prev = line
    return Image(width, height, channels, rows)


def bounds(img, colour, tolerance=(10, 10, 10), least=30):
    """The box around the pixels of `colour` (within `tolerance` per channel), or None when fewer
    than `least` have it."""
    xs, ys = [], []
    for y in range(img.height):
        row = img.rows[y]
        for x in range(img.width):
            i = x * img.channels
            if all(abs(row[i + k] - colour[k]) <= tolerance[k] for k in range(3)):
                xs.append(x)
                ys.append(y)
    if len(xs) < least:
        return None
    return (min(xs), min(ys), max(xs), max(ys))


def share(img, colour, tolerance=(10, 10, 10)):
    """The share of the display's pixels that have `colour`."""
    n = 0
    for y in range(img.height):
        row = img.rows[y]
        for x in range(img.width):
            i = x * img.channels
            if all(abs(row[i + k] - colour[k]) <= tolerance[k] for k in range(3)):
                n += 1
    return n / (img.width * img.height)


def inside(img, box, inset=4):
    """The pixels within `box` (x0, y0, x1, y1), `inset` in from its edges: what a focus ring holds,
    to tell which element it is on wherever the view has scrolled it."""
    x0, y0, x1, y1 = box
    out = bytearray()
    for y in range(y0 + inset, y1 - inset + 1):
        row = img.rows[y]
        for x in range(x0 + inset, x1 - inset + 1):
            i = x * img.channels
            out += row[i:i + 3]
    return bytes(out)


def same(a, b, tolerance=2.0):
    """Whether two pixel runs from `inside` show the same thing: same size, and a mean difference
    per channel within `tolerance`."""
    if len(a) != len(b) or not a:
        return False
    return sum(abs(p - q) for p, q in zip(a, b)) / len(a) <= tolerance
