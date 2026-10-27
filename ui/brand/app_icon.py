#!/usr/bin/env python3
"""Generate the OpenDAP app icon: the light mark on the graphite studio backdrop (the avatar's,
README.md), inside the macOS icon shape, for OpenDAP's own host applications (the virtual devices'
window, OpenDAP Desktop).

The mark comes from mark.py's own contours, rasterised here with supersampling, so no SVG renderer
is needed. The body follows Apple's icon grid: 824 px of a 1024 px canvas, a continuous-corner
squircle, with a soft shadow below it.

  app_icon.py OUT_DIR        writes opendap-app-icon.png (1024 px) and opendap-app.icns (macOS)
"""
import argparse, math, pathlib, shutil, subprocess, sys, tempfile

from PIL import Image, ImageDraw, ImageFilter

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import mark  # noqa: E402

CANVAS = 1024
BODY = 824                                    # Apple's icon grid
SQUIRCLE_N = 5.0                              # superellipse exponent, close to Apple's continuous corners
MARK_SHARE = 0.56                             # the mark's width in the body, as the avatar's lockup
SS = 4                                        # supersampling
LIGHT = (0xF5, 0xF6, 0xF4)                    # color.white
BASE = ((0x1D, 0x21, 0x25), (0x15, 0x18, 0x1B), (0x0D, 0x0F, 0x11))   # the avatar's backdrop
POOL = (0xAA, 0xB9, 0xCD)


def flatten(segs, steps=24):
    """A contour of (line-to, control, control, end) segments as a polygon."""
    pts = []
    for a, c1, c2, b in segs:
        pts.append(a)
        for i in range(1, steps + 1):
            t = i / steps
            m = 1 - t
            pts.append((m**3 * a[0] + 3 * m * m * t * c1[0] + 3 * m * t * t * c2[0] + t**3 * b[0],
                        m**3 * a[1] + 3 * m * m * t * c1[1] + 3 * m * t * t * c2[1] + t**3 * b[1]))
    return pts


def squircle(size, n=SQUIRCLE_N, steps=720):
    r = size / 2
    out = []
    for i in range(steps):
        a = 2 * math.pi * i / steps
        c, s = math.cos(a), math.sin(a)
        out.append((r + r * math.copysign(abs(c) ** (2 / n), c), r + r * math.copysign(abs(s) ** (2 / n), s)))
    return out


def lerp(a, b, t):
    return tuple(round(x + (y - x) * t) for x, y in zip(a, b))


def backdrop(size):
    """The radial graphite gradient with its soft cool light pool."""
    img = Image.new("RGB", (size, size))
    px = img.load()
    c = size / 2
    for y in range(size):
        for x in range(size):
            d = math.hypot(x + 0.5 - c, y + 0.5 - c) / (0.70 * size)
            col = lerp(BASE[0], BASE[1], d / 0.55) if d < 0.55 else lerp(BASE[1], BASE[2], min(1, (d - 0.55) / 0.45))
            p = math.hypot(x + 0.5 - c, y + 0.5 - 0.48 * size) / (0.45 * size)
            k = 0.13 * max(0.0, 1 - p / 0.72)
            px[x, y] = lerp(col, POOL, k)
    return img


def icon():
    big = CANVAS * SS
    body_px = BODY * SS
    off = (big - body_px) // 2
    body = [(x + off, y + off) for x, y in squircle(body_px)]

    # The body's shadow: a blurred, offset silhouette.
    shadow = Image.new("L", (big, big), 0)
    ImageDraw.Draw(shadow).polygon([(x, y + 10 * SS) for x, y in body], fill=int(255 * 0.32))
    shadow = shadow.filter(ImageFilter.GaussianBlur(12 * SS))

    # The body: the backdrop, drawn at full size, cut to the squircle.
    body_mask = Image.new("L", (big, big), 0)
    ImageDraw.Draw(body_mask).polygon(body, fill=255)
    fill = backdrop(CANVAS).resize((big, big), Image.BICUBIC)

    # The mark: outlines filled, counters cut, centred on its ink.
    cs = mark.contours()
    x0, y0, x1, y1 = mark.bounds(cs)
    k = MARK_SHARE * body_px / (x1 - x0)
    mx = off + (body_px - (x1 - x0) * k) / 2
    my = off + (body_px - (y1 - y0) * k) / 2
    mark_mask = Image.new("L", (big, big), 0)
    draw = ImageDraw.Draw(mark_mask)
    for i, segs in enumerate(cs):
        poly = [(mx + (x - x0) * k, my + (y - y0) * k) for x, y in flatten(segs)]
        draw.polygon(poly, fill=255 if i % 2 == 0 else 0)  # outline, then its counter

    out = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    out.paste((0, 0, 0, 255), (0, 0), shadow)
    out.paste(fill, (0, 0), body_mask)
    out.paste(LIGHT + (255,), (0, 0), mark_mask)
    return out.resize((CANVAS, CANVAS), Image.LANCZOS)


def icns(png, path):
    """macOS .icns through iconutil, which ships with macOS."""
    if shutil.which("iconutil") is None:
        print("app_icon: iconutil not found (macOS only): no .icns written", file=sys.stderr)
        return
    with tempfile.TemporaryDirectory() as tmp:
        iconset = pathlib.Path(tmp) / "opendap-app.iconset"
        iconset.mkdir()
        for size in (16, 32, 128, 256, 512):
            for scale in (1, 2):
                name = f"icon_{size}x{size}{'@2x' if scale == 2 else ''}.png"
                png.resize((size * scale, size * scale), Image.LANCZOS).save(iconset / name)
        subprocess.run(["iconutil", "-c", "icns", "-o", str(path), str(iconset)], check=True)


def main():
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("out", type=pathlib.Path)
    args = p.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    png = icon()
    png.save(args.out / "opendap-app-icon.png")
    icns(png, args.out / "opendap-app.icns")


if __name__ == "__main__":
    main()
