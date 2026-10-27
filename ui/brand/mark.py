#!/usr/bin/env python3
"""Generate the OpenDAP mark: two identical triangular pieces, the O pointing left and the D pointing
right, offset diagonally. See README.md for the construction table and the usage rules.

Output SVGs are flat on purpose, so they scale and convert without surprises: one filled <path>,
absolute coordinates, no transforms, groups, strokes, styles or arcs (corners are cubic Béziers),
outer contours clockwise and counters counter-clockwise (the same result under either fill rule),
and a viewBox fitted to the visible outline with its origin at 0,0.

  mark.py OUT.svg [--fill HEX|currentColor] [--grid]
"""
import argparse, math

STROKE = 28                                    # stem, u
SIDE = 8 * STROKE // 7                         # sloped stroke, measured vertically: 32 u
CHAMFER_OUTER, CHAMFER = 16, 8                 # one symmetric chamfer per corner, outside and in the counter
R = STROKE / 14                                # corner radius: 2 u
GAP = STROKE // 2                              # between the pieces, and each piece's offset: 14 u

# The D piece on the 1:1 grid: stem on x = 0, apex to the right; the sides run 4 u down per 7 u across.
STEM, DEPTH = 192, 168
TRI = [(0, 0), (DEPTH, STEM // 2), (0, STEM)]
COUNTER = [(STROKE, SIDE + 16), (DEPTH - 2 * STROKE, STEM // 2), (STROKE, STEM - SIDE - 16)]
APEX_IN = 14                                   # the 16 u apex face sits 14 u in from the tip


def chamfer(poly, length):
    """Cut every corner symmetrically with a face of `length`: each corner of angle t moves
    length / (2 sin(t/2)) along both of its edges."""
    out, n = [], len(poly)
    for i in range(n):
        prev, cur, nxt = poly[i - 1], poly[i], poly[(i + 1) % n]
        v1 = (prev[0] - cur[0], prev[1] - cur[1]); v2 = (nxt[0] - cur[0], nxt[1] - cur[1])
        d1, d2 = math.hypot(*v1), math.hypot(*v2)
        angle = math.acos((v1[0] * v2[0] + v1[1] * v2[1]) / (d1 * d2))
        leg = length / (2 * math.sin(angle / 2))
        out += [(cur[0] + v1[0] * leg / d1, cur[1] + v1[1] * leg / d1),
                (cur[0] + v2[0] * leg / d2, cur[1] + v2[1] * leg / d2)]
    return out


def area(poly):
    return sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(poly, poly[1:] + poly[:1])) / 2


def rounded(poly, r):
    """Round every corner with a circular arc of radius r, drawn as one cubic Bézier per corner
    (handle length 4/3 tan(phi/4) r, phi the turning angle). Returns segments for a closed path."""
    segs, n = [], len(poly)
    for i in range(n):
        prev, cur, nxt = poly[i - 1], poly[i], poly[(i + 1) % n]
        v1 = (prev[0] - cur[0], prev[1] - cur[1]); v2 = (nxt[0] - cur[0], nxt[1] - cur[1])
        d1, d2 = math.hypot(*v1), math.hypot(*v2)
        u1, u2 = (v1[0] / d1, v1[1] / d1), (v2[0] / d2, v2[1] / d2)
        angle = math.acos(max(-1.0, min(1.0, u1[0] * u2[0] + u1[1] * u2[1])))
        t = min(r / math.tan(angle / 2), d1 / 2, d2 / 2)
        h = 4 / 3 * math.tan((math.pi - angle) / 4) * t * math.tan(angle / 2)
        a = (cur[0] + u1[0] * t, cur[1] + u1[1] * t); b = (cur[0] + u2[0] * t, cur[1] + u2[1] * t)
        segs.append((a, (a[0] - u1[0] * h, a[1] - u1[1] * h), (b[0] - u2[0] * h, b[1] - u2[1] * h), b))
    return segs


def contours():
    """The mark's four contours in grid units: each piece's outline (clockwise) and counter
    (counter-clockwise). The O is the D turned 180°, lowered; the D is raised."""
    outer, counter = chamfer(TRI, CHAMFER_OUTER), chamfer(COUNTER, CHAMFER)
    d = lambda pts: [(x + DEPTH + GAP - APEX_IN, y) for x, y in pts]
    o = lambda pts: [(DEPTH - APEX_IN - x, STEM - y + 2 * GAP) for x, y in pts]
    out = []
    for f in (o, d):
        for pts, clockwise in ((f(outer), True), (f(counter), False)):
            if (area(pts) > 0) != clockwise:       # y points down: positive area is clockwise
                pts = pts[::-1]
            out.append(rounded(pts, R))
    return out


def bezier_bounds(segs):
    xs, ys = [], []
    for p0, p1, p2, p3 in segs:
        for t in [i / 64 for i in range(65)]:
            m = 1 - t
            xs.append(m**3 * p0[0] + 3 * m * m * t * p1[0] + 3 * m * t * t * p2[0] + t**3 * p3[0])
            ys.append(m**3 * p0[1] + 3 * m * m * t * p1[1] + 3 * m * t * t * p2[1] + t**3 * p3[1])
    return min(xs), min(ys), max(xs), max(ys)


def path_data(cs, dx=0, dy=0, scale=1, digits=3):
    f = lambda p: f"{round((p[0] - dx) * scale, digits):g} {round((p[1] - dy) * scale, digits):g}"
    d = []
    for segs in cs:
        d.append(f"M{f(segs[-1][3])}")
        for a, c1, c2, b in segs:
            d.append(f"L{f(a)}C{f(c1)} {f(c2)} {f(b)}")
        d.append("Z")
    return "".join(d)


def bounds(cs):
    bs = [bezier_bounds(s) for s in cs]
    return min(b[0] for b in bs), min(b[1] for b in bs), max(b[2] for b in bs), max(b[3] for b in bs)


def svg(fill):
    cs = contours()
    x0, y0, x1, y1 = bounds(cs)
    w, h = round(x1 - x0, 3), round(y1 - y0, 3)
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w:g} {h:g}" role="img">'
            f'<title>OpenDAP</title><path fill="{fill}" d="{path_data(cs, x0, y0)}"/></svg>\n')


def construction(u=4):
    """Construction drawing on the grid (a document, not a logo asset)."""
    W, H, pad = 2 * (DEPTH - APEX_IN) + GAP, STEM + 2 * GAP, 21
    out = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{-pad * u} {-pad * u} {(W + 2 * pad) * u} {(H + 2 * pad) * u}">',
           '<title>OpenDAP mark: construction</title>',
           f'<rect x="{-pad * u}" y="{-pad * u}" width="{(W + 2 * pad) * u}" height="{(H + 2 * pad) * u}" fill="#111315"/>']
    for step, colour, width in ((1, "#22272C", 0.04), (7, "#3A4148", 0.12)):
        g = [f'<line x1="{x * u}" y1="{-pad * u}" x2="{x * u}" y2="{(H + pad) * u}"/>' for x in range(0, W + 1, step)]
        g += [f'<line x1="{-pad * u}" y1="{y * u}" x2="{(W + pad) * u}" y2="{y * u}"/>' for y in range(0, H + 1, step)]
        out.append(f'<g stroke="{colour}" stroke-width="{u * width}">' + "".join(g) + '</g>')
    out.append(f'<path fill="#F5F6F4" opacity="0.85" d="{path_data(contours(), scale=u)}"/>')
    pts = lambda ps: " ".join(f"{x * u},{y * u}" for x, y in ps)
    frames = [lambda ps: [(DEPTH - APEX_IN - x, STEM - y + 2 * GAP) for x, y in ps],
              lambda ps: [(x + DEPTH + GAP - APEX_IN, y) for x, y in ps]]
    polys = "".join(f'<polygon points="{pts(f(p))}"/>' for f in frames for p in (TRI, COUNTER))
    out.append(f'<g fill="none" stroke="#FF6A2A" stroke-width="{u * 0.12}" stroke-dasharray="{u * 0.5},{u * 0.3}">{polys}</g>')
    x0 = DEPTH + GAP - APEX_IN
    out.append(f'<polygon fill="none" stroke="#69BFFF" stroke-width="{u * 0.12}" points="{pts([(x0, 0), (x0 + 28, 16), (x0, 16)])}"/>')
    out.append('</svg>')
    return "\n".join(out) + "\n"


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--fill", default="#F5F6F4")
    ap.add_argument("--grid", action="store_true", help="write the construction drawing instead")
    a = ap.parse_args()
    open(a.out, "w").write(construction() if a.grid else svg(a.fill))
