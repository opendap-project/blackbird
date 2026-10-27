#!/usr/bin/env python3
"""Generate the OpenDAP project's marks: the mark, the wordmark and the vertical lockup in each colour
(svg/opendap-<form>-<colour>.svg), and the construction drawing (opendap-mark-construction.svg).

The wordmark "OpenDAP" is IBM Plex Sans SemiBold converted to outlines (outline.py). The colours are the
design tokens' (colours.py). Product lockups (Blackbird OS) are products.py's.

  logo.py --plex DIR [--root DIR] [--check]

Requires fontTools and uharfbuzz.
"""
import colours
import mark
import outline


def layouts(font):
    """Each output as a function of an offset (ox, oy) -> (path data, ink bounds); the bounds at
    offset 0 tell the offset that puts the ink at 0,0."""
    cs = mark.contours()
    mx0, my0, mx1, my1 = mark.bounds(cs)
    m = lambda ox, oy: mark.path_data(cs, -ox, -oy)
    mb = lambda ox, oy: (mx0 + ox, my0 + oy, mx1 + ox, my1 + oy)
    cap_v, gap_v = outline.LOCKUPS["vertical"]
    probe = outline.wordmark(font, cap_v, 0, 0)[1]
    width_v = probe[2] - probe[0]
    words = {
        "wordmark": lambda ox, oy: outline.wordmark(font, outline.WORDMARK_CAP, ox, outline.WORDMARK_CAP + oy),
        "vertical": lambda ox, oy: outline.wordmark(font, cap_v, (mx0 + mx1) / 2 - width_v / 2 + ox, my1 + gap_v + cap_v + oy),
    }
    out = {"mark": lambda ox, oy: (m(ox, oy), mb(ox, oy)), "wordmark": words["wordmark"]}
    for name in ("vertical",):
        def f(ox, oy, w=words[name]):
            wd, wb = w(ox, oy); b = mb(ox, oy)
            return m(ox, oy) + wd, (min(b[0], wb[0]), min(b[1], wb[1]), max(b[2], wb[2]), max(b[3], wb[3]))
        out[name] = f
    return out


def outputs(plex):
    """{path relative to the brand folder: file text} for every OpenDAP mark."""
    bold, _ = outline.fonts(plex)
    files = {}
    for name, build in layouts(bold).items():
        for colour, fill in colours.LOGO.items():
            if colour == "accent" and name != "mark":
                continue                       # the orange is for the mark alone
            files[f"svg/opendap-{name}-{colour}.svg"] = outline.svg_text(build, fill, "OpenDAP")
    files["opendap-mark-construction.svg"] = mark.construction()
    return files


if __name__ == "__main__":
    outline.run(outputs, __doc__)
