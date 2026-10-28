# SPDX-FileCopyrightText: 2026 The OpenDAP Project
# SPDX-License-Identifier: MPL-2.0
"""Tests of the status glyphs' pixel-grid alignment (pixelgrid.py). Run: python3 -m unittest icons/test_pixelgrid.py"""
import pathlib
import shutil
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import pixelgrid  # noqa: E402

BATTERY = ("M6 7h11a2 2 0 0 1 2 2v.5a.5 .5 0 0 0 .5 .5a.5 .5 0 0 1 .5 .5v3a.5 .5 0 0 1 -.5 .5"
           "a.5 .5 0 0 0 -.5 .5v.5a2 2 0 0 1 -2 2h-11a2 2 0 0 1 -2 -2v-6a2 2 0 0 1 2 -2")


class Strokes(unittest.TestCase):
    def test_battery_outline_and_bar(self):
        found = pixelgrid.strokes(BATTERY)
        self.assertIn(("h", 7.0, 11.0), found)
        self.assertIn(("h", 17.0, 11.0), found)
        self.assertIn(("v", 4.0, 6.0), found)
        self.assertIn(("v", 20.0, 3.0), found)
        self.assertNotIn(("v", 19.0, 0.5), found, "shorter than a unit: a dot, not a line")
        self.assertEqual(pixelgrid.strokes("M7 10l0 4"), [("v", 7.0, 4.0)])

    def test_curves_and_diagonals_do_not_count(self):
        self.assertEqual(pixelgrid.strokes("M7 8l10 8l-5 4"), [])
        self.assertEqual(pixelgrid.strokes("M15 8a5 5 0 0 1 0 8"), [])
        self.assertEqual(pixelgrid.strokes("M12 18l.01 0"), [], "Wi-Fi's dot")


class Offset(unittest.TestCase):
    def test_residual(self):
        self.assertAlmostEqual(pixelgrid.residual(0.75), 0)
        self.assertAlmostEqual(abs(pixelgrid.residual(4)), 1 / 6)

    def test_battery_bars_land_on_pixel_centres(self):
        for bars in (["M7 10l0 4"], ["M7 10l0 4", "M10 10l0 4", "M13 10l0 4", "M16 10l0 4"]):
            dx, _ = pixelgrid.offset([BATTERY] + bars)
            self.assertEqual(dx, -0.25)
            for x in (4, 7, 10, 13, 16, 19, 20):
                if x == 20:
                    continue  # the nub, a third of a pixel from the body's edge
                self.assertAlmostEqual(pixelgrid.residual(x + dx), 0, msg=x)

    def test_first_bar_leaves_an_empty_column_beside_the_outline(self):
        dx, _ = pixelgrid.offset([BATTERY, "M7 10l0 4"])
        outline = (4 + dx) * 16 / 24  # stroke centre, px
        bar = (7 + dx) * 16 / 24
        # 1 px strokes centred on pixel centres: their columns are int(centre); one column between.
        self.assertEqual(int(bar) - int(outline), 2)

    def test_a_glyph_without_straight_strokes_stays(self):
        self.assertEqual(pixelgrid.offset(["M7 8l10 8l-5 4l0 -16l5 4l-10 8"])[1], 0.0)


@unittest.skipUnless(shutil.which("rsvg-convert"), "needs rsvg-convert (librsvg)")
class Exported(unittest.TestCase):
    """The exported 16 px glyphs, rendered: what the status bar draws."""

    def cols(self, name):
        return pixelgrid.ink(pixelgrid_out() / f"{name}.svg")[0]

    def test_one_bar_is_a_sharp_column_apart_from_the_outline(self):
        empty, one = self.cols("status-battery-0"), self.cols("status-battery-1")
        changed = [x for x in range(16) if one[x] != empty[x]]
        # The bar adds ink to one column only (one sharp 1 px line), not two half-lit ones.
        self.assertEqual(len(changed), 1, (empty, one))
        bar = changed[0]
        outline = max(range(bar), key=lambda x: empty[x])
        self.assertGreaterEqual(bar - outline, 2, "an empty column between the outline and the bar")
        self.assertGreater(one[bar] - empty[bar], 300, "the bar's 1 px line is nearly full")

    def test_the_outline_edge_is_one_sharp_column(self):
        empty = self.cols("status-battery-0")
        edge = max(range(8), key=lambda x: empty[x])
        self.assertGreater(empty[edge], 2.5 * empty[edge + 1], empty)


def pixelgrid_out():
    return pathlib.Path(__file__).resolve().parent.parent / "export" / "icons" / "16"


if __name__ == "__main__":
    unittest.main()
