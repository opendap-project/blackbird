#!/usr/bin/env python3
"""Generate every OpenDAP brand SVG: the mark, the wordmark and the vertical lockup, in each colour,
and the product lockups and banner for each OpenDAP software product (Blackbird OS) in products/<slug>/.

The wordmark "OpenDAP" is set in IBM Plex Sans SemiBold and product names in Regular (SIL OFL 1.1), shaped
with HarfBuzz (kerning on) and converted to outlines, so no font is needed to display them. The fonts
are not stored here: pass the folder of OTFs from
https://github.com/IBM/plex (ibm-plex-sans, fonts/complete/otf).

Every output is flat (see mark.py): one filled <path>, absolute coordinates, cubic Béziers only, no
transforms, groups, strokes, text or styles, and a viewBox fitted to the ink with its origin at 0,0.

  logo.py --plex DIR [--out DIR]

Requires fontTools and uharfbuzz.
"""
import argparse, os
import uharfbuzz as hb
from fontTools.ttLib import TTFont
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.transformPen import TransformPen

import mark

S = mark.STROKE                                # every lockup measure is a multiple of the stroke
LOCKUPS = {
    # name: (cap height, gap between mark and wordmark). For horizontal spaces the wordmark stands alone.
    "vertical": (5 * S / 2, 3 * S / 2),        # 70 u caps, 42 u gap, wordmark centred under the mark
}
WORDMARK_CAP = 5 * S                           # the standalone wordmark's scale (it is scaled as a whole)
PRODUCTS = ["Blackbird OS"]                    # product lockups under the OpenDAP wordmark
NAME_SIZE = 1 / 2                            # product name cap height, in wordmark cap heights
NAME_GAP = 3 / 4                               # wordmark baseline to product name cap top, in wordmark cap heights
BANNER = (1280, 640)                           # product banner, px (release notes, repository preview)
COLOURS = {"light": "#F5F6F4", "dark": "#111315", "accent": "#FF6A2A", "current": "currentColor"}
NUM = lambda v: f"{round(v, 3):g}"


def wordmark(font_path, cap_u, x0, baseline, text="OpenDAP"):
    """Path data and ink bounds of `text` with its cap height at cap_u, starting at x0 (the left
    edge of its ink) on the given baseline."""
    face = hb.Face(hb.Blob.from_file_path(font_path)); font = hb.Font(face)
    buf = hb.Buffer(); buf.add_str(text); buf.guess_segment_properties()
    hb.shape(font, buf, {"kern": True, "liga": True})
    tt = TTFont(font_path); glyphs = tt.getGlyphSet(); order = tt.getGlyphOrder()
    s = cap_u / tt["OS/2"].sCapHeight
    placed, x = [], 0
    for info, pos in zip(buf.glyph_infos, buf.glyph_positions):
        placed.append((order[info.codepoint], x + pos.x_offset, pos.y_offset)); x += pos.x_advance
    bp = BoundsPen(glyphs)
    for g, gx, gy in placed:
        glyphs[g].draw(TransformPen(bp, (s, 0, 0, -s, gx * s, -gy * s)))
    left = bp.bounds[0]
    pen, bp = SVGPathPen(glyphs, ntos=NUM), BoundsPen(glyphs)
    for g, gx, gy in placed:
        t = (s, 0, 0, -s, gx * s + x0 - left, baseline - gy * s)
        glyphs[g].draw(TransformPen(pen, t)); glyphs[g].draw(TransformPen(bp, t))
    return pen.getCommands(), bp.bounds


def layouts(font):
    """Each output as a function of an offset (ox, oy) -> (path data, ink bounds); the bounds at
    offset 0 tell the offset that puts the ink at 0,0."""
    cs = mark.contours()
    mx0, my0, mx1, my1 = mark.bounds(cs)
    m = lambda ox, oy: mark.path_data(cs, -ox, -oy)
    mb = lambda ox, oy: (mx0 + ox, my0 + oy, mx1 + ox, my1 + oy)
    cap_v, gap_v = LOCKUPS["vertical"]
    probe = wordmark(font, cap_v, 0, 0)[1]
    width_v = probe[2] - probe[0]
    words = {
        "wordmark": lambda ox, oy: wordmark(font, WORDMARK_CAP, ox, WORDMARK_CAP + oy),
        "vertical": lambda ox, oy: wordmark(font, cap_v, (mx0 + mx1) / 2 - width_v / 2 + ox, my1 + gap_v + cap_v + oy),
    }
    out = {"mark": lambda ox, oy: (m(ox, oy), mb(ox, oy)), "wordmark": words["wordmark"]}
    for name in ("vertical",):
        def f(ox, oy, w=words[name]):
            wd, wb = w(ox, oy); b = mb(ox, oy)
            return m(ox, oy) + wd, (min(b[0], wb[0]), min(b[1], wb[1]), max(b[2], wb[2]), max(b[3], wb[3]))
        out[name] = f
    return out


def union(*bs):
    return min(b[0] for b in bs), min(b[1] for b in bs), max(b[2] for b in bs), max(b[3] for b in bs)


def product_layouts(font, regular, name):
    """Product lockups, every builder (ox, oy, k) -> (path data, ink bounds) at scale k:
    - stacked: "OpenDAP" (SemiBold) above the name (Regular, smaller), centred; no mark,
      like the wordmark;
    - vertical: the mark above the two lines, all centred, with the vertical lockup's measures."""
    cs = mark.contours()
    mx0, my0, mx1, my1 = mark.bounds(cs)
    width = lambda f, cap, text: (lambda b: b[2] - b[0])(wordmark(f, cap, 0, 0, text)[1])
    cap_v, gap = LOCKUPS["vertical"]

    def lines(x_of, top, cap, ox, oy, k):
        small = cap * NAME_SIZE
        a, ab = wordmark(font, cap * k, ox + x_of(font, cap, "OpenDAP") * k, oy + (top + cap) * k)
        b, bb = wordmark(regular, small * k, ox + x_of(regular, small, name) * k,
                         oy + (top + cap + cap * NAME_GAP + small) * k, name)
        return a + b, union(ab, bb)

    def stacked(ox, oy, k=1):
        w0 = width(font, WORDMARK_CAP, "OpenDAP")
        return lines(lambda f, cap, text: (w0 - width(f, cap, text)) / 2, 0, WORDMARK_CAP, ox, oy, k)

    cx = (mx0 + mx1) / 2

    def vertical(ox, oy, k=1):
        d, b = lines(lambda f, cap, text: cx - width(f, cap, text) / 2, my1 + gap, cap_v, ox, oy, k)
        m = mark.path_data(cs, -ox / k, -oy / k, k)
        return m + d, union((mx0 * k + ox, my0 * k + oy, mx1 * k + ox, my1 * k + oy), b)

    return {"stacked": stacked, "vertical": vertical}


def svg_file(path, build, fill, title):
    x0, y0, x1, y1 = build(0, 0)[1]
    d, _ = build(-x0, -y0)
    with open(path, "w") as f:
        f.write(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {NUM(x1 - x0)} {NUM(y1 - y0)}" role="img">'
                f'<title>{title}</title><path fill="{fill}" d="{d}"/></svg>\n')


def banner(path, build, title):
    """The vertical product lockup centred on graphite, its height half the banner's."""
    W, H = BANNER
    x0, y0, x1, y1 = build(0, 0)[1]
    k = H / 2 / (y1 - y0)
    d, _ = build((W - (x1 - x0) * k) / 2 - x0 * k, (H - (y1 - y0) * k) / 2 - y0 * k, k)
    with open(path, "w") as f:
        f.write(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" role="img"><title>{title}</title>'
                f'<rect width="{W}" height="{H}" fill="{COLOURS["dark"]}"/><path fill="{COLOURS["light"]}" d="{d}"/></svg>\n')


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--plex", required=True, help="the ibm-plex-sans fonts/complete/otf folder")
    ap.add_argument("--out", default=os.path.join(os.path.dirname(os.path.abspath(__file__)), "svg"))
    a = ap.parse_args()
    os.makedirs(a.out, exist_ok=True)
    bold = os.path.join(a.plex, "IBMPlexSans-SemiBold.otf")
    regular = os.path.join(a.plex, "IBMPlexSans-Regular.otf")
    for name, build in layouts(bold).items():
        for colour, fill in COLOURS.items():
            if colour == "accent" and name != "mark":
                continue                       # the orange is for the mark alone
            svg_file(os.path.join(a.out, f"opendap-{name}-{colour}.svg"), build, fill, "OpenDAP")
    here = os.path.dirname(os.path.abspath(__file__))
    for product in PRODUCTS:
        slug = product.lower().replace(" ", "-")
        out = os.path.join(here, "products", slug)
        os.makedirs(out, exist_ok=True)
        builds = product_layouts(bold, regular, product)
        for name, build in builds.items():
            for colour, fill in COLOURS.items():
                if colour != "accent":
                    svg_file(os.path.join(out, f"{slug}-{name}-{colour}.svg"), build, fill, product)
        banner(os.path.join(out, f"{slug}-banner.svg"), builds["vertical"], product)
    with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "opendap-mark-construction.svg"), "w") as f:
        f.write(mark.construction())


if __name__ == "__main__":
    main()
