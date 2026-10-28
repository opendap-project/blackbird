"""Text to outlines and flat SVG files, shared by logo.py (the OpenDAP marks) and products.py (the product
lockups).

Text is set in IBM Plex Sans (SIL OFL 1.1), shaped with HarfBuzz (kerning on) and converted to outlines,
so no font is needed to display it. Every file is flat (see mark.py): one filled <path>, absolute
coordinates, cubic Béziers only, no transforms, groups, strokes, text or styles, and a viewBox fitted
to the ink with its origin at 0,0.

Requires fontTools and uharfbuzz.
"""
import argparse
import os
import pathlib
import sys

import uharfbuzz as hb
from fontTools.ttLib import TTFont
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.transformPen import TransformPen

import mark

HERE = pathlib.Path(__file__).resolve().parent
S = mark.STROKE                                # every lockup measure is a multiple of the stroke
LOCKUPS = {
    # name: (cap height, gap between mark and wordmark). For horizontal spaces the wordmark stands alone.
    "vertical": (5 * S / 2, 3 * S / 2),        # 70 u caps, 42 u gap, wordmark centred under the mark
}
WORDMARK_CAP = 5 * S                           # the standalone wordmark's scale (it is scaled as a whole)
NUM = lambda v: f"{round(v, 3):g}"


def fonts(plex):
    """The SemiBold (the wordmark) and Regular (product names) OTFs in the Plex folder."""
    return os.path.join(plex, "IBMPlexSans-SemiBold.otf"), os.path.join(plex, "IBMPlexSans-Regular.otf")


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


def union(*bs):
    return min(b[0] for b in bs), min(b[1] for b in bs), max(b[2] for b in bs), max(b[3] for b in bs)


def svg_text(build, fill, title):
    """One flat SVG: the builder's ink moved to 0,0, filled with one colour."""
    x0, y0, x1, y1 = build(0, 0)[1]
    d, _ = build(-x0, -y0)
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {NUM(x1 - x0)} {NUM(y1 - y0)}" role="img">'
            f'<title>{title}</title><path fill="{fill}" d="{d}"/></svg>\n')


def run(outputs, doc):
    """The generators' command line: write `outputs(plex)` under the brand folder, or check it."""
    ap = argparse.ArgumentParser(description=doc.splitlines()[0])
    ap.add_argument("--plex", required=True, help="the ibm-plex-sans fonts/complete/otf folder (IBM Plex Sans 1.1.0)")
    ap.add_argument("--root", type=pathlib.Path, default=HERE, help="the brand folder to write into (default: this one)")
    ap.add_argument("--check", action="store_true", help="exit 1 if a committed file differs from what would be written")
    a = ap.parse_args()
    files = outputs(a.plex)
    stale = [rel for rel, text in files.items() if not (a.root / rel).exists() or (a.root / rel).read_text() != text]
    if a.check:
        if stale:
            print("out of date: " + ", ".join(stale), file=sys.stderr)
            sys.exit(1)
        return
    for rel, text in files.items():
        (a.root / rel).parent.mkdir(parents=True, exist_ok=True)
        (a.root / rel).write_text(text)
        print(rel)
