#!/usr/bin/env python3
"""Generate the lockups of each OpenDAP software product (Blackbird OS) in products/<slug>/: the stacked and
vertical lockups in each colour and the banner.

"OpenDAP" is IBM Plex Sans SemiBold and the product name Regular, converted to outlines (outline.py). The
colours are the design tokens' (colours.py). The OpenDAP marks themselves are logo.py's.

  products.py --plex DIR [--root DIR] [--check]

Requires fontTools and uharfbuzz.
"""
import colours
import mark
import outline

PRODUCTS = ["Blackbird OS"]                    # product lockups under the OpenDAP wordmark
NAME_SIZE = 1 / 2                              # product name cap height, in wordmark cap heights
NAME_GAP = 3 / 4                               # wordmark baseline to product name cap top, in wordmark cap heights
BANNER = (1280, 640)                           # product banner, px (release notes, repository preview)


def product_layouts(font, regular, name):
    """Product lockups, every builder (ox, oy, k) -> (path data, ink bounds) at scale k:
    - stacked: "OpenDAP" (SemiBold) above the name (Regular, smaller), centred; no mark,
      like the wordmark;
    - vertical: the mark above the two lines, all centred, with the vertical lockup's measures."""
    cs = mark.contours()
    mx0, my0, mx1, my1 = mark.bounds(cs)
    width = lambda f, cap, text: (lambda b: b[2] - b[0])(outline.wordmark(f, cap, 0, 0, text)[1])
    cap_v, gap = outline.LOCKUPS["vertical"]

    def lines(x_of, top, cap, ox, oy, k):
        small = cap * NAME_SIZE
        a, ab = outline.wordmark(font, cap * k, ox + x_of(font, cap, "OpenDAP") * k, oy + (top + cap) * k)
        b, bb = outline.wordmark(regular, small * k, ox + x_of(regular, small, name) * k,
                                 oy + (top + cap + cap * NAME_GAP + small) * k, name)
        return a + b, outline.union(ab, bb)

    def stacked(ox, oy, k=1):
        w0 = width(font, outline.WORDMARK_CAP, "OpenDAP")
        return lines(lambda f, cap, text: (w0 - width(f, cap, text)) / 2, 0, outline.WORDMARK_CAP, ox, oy, k)

    cx = (mx0 + mx1) / 2

    def vertical(ox, oy, k=1):
        d, b = lines(lambda f, cap, text: cx - width(f, cap, text) / 2, my1 + gap, cap_v, ox, oy, k)
        m = mark.path_data(cs, -ox / k, -oy / k, k)
        return m + d, outline.union((mx0 * k + ox, my0 * k + oy, mx1 * k + ox, my1 * k + oy), b)

    return {"stacked": stacked, "vertical": vertical}


def banner(build, title):
    """The vertical product lockup centred on graphite, its height half the banner's."""
    W, H = BANNER
    x0, y0, x1, y1 = build(0, 0)[1]
    k = H / 2 / (y1 - y0)
    d, _ = build((W - (x1 - x0) * k) / 2 - x0 * k, (H - (y1 - y0) * k) / 2 - y0 * k, k)
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" role="img"><title>{title}</title>'
            f'<rect width="{W}" height="{H}" fill="{colours.LOGO["dark"]}"/><path fill="{colours.LOGO["light"]}" d="{d}"/></svg>\n')


def outputs(plex):
    """{path relative to the brand folder: file text} for every product lockup."""
    bold, regular = outline.fonts(plex)
    files = {}
    for product in PRODUCTS:
        slug = product.lower().replace(" ", "-")
        builds = product_layouts(bold, regular, product)
        for name, build in builds.items():
            for colour, fill in colours.LOGO.items():
                if colour != "accent":         # product lockups are never orange
                    files[f"products/{slug}/{slug}-{name}-{colour}.svg"] = outline.svg_text(build, fill, product)
        files[f"products/{slug}/{slug}-banner.svg"] = banner(builds["vertical"], product)
    return files


if __name__ == "__main__":
    outline.run(outputs, __doc__)
