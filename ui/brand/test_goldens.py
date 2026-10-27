# SPDX-FileCopyrightText: 2026 The OpenDAP Project
# SPDX-License-Identifier: GPL-3.0-or-later
"""Golden tests of the brand generators: every generated file is byte-identical to the committed one.

The mark, the construction drawing and the app icon need nothing else. The wordmark, the lockups and the
product lockups need IBM Plex Sans 1.1.0 SemiBold and Regular OTFs (ibm-plex-sans, fonts/complete/otf,
https://github.com/IBM/plex); set ODAP_PLEX_SANS_OTF to that folder, or those tests are skipped.

Run: python3 -m unittest brand/test_goldens.py
"""
import os
import pathlib
import sys
import unittest

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import app_icon  # noqa: E402
import colours  # noqa: E402
import logo  # noqa: E402
import mark  # noqa: E402

PLEX = os.environ.get("ODAP_PLEX_SANS_OTF")
needs_plex = unittest.skipUnless(PLEX and pathlib.Path(PLEX, "IBMPlexSans-SemiBold.otf").exists(),
                                 "set ODAP_PLEX_SANS_OTF to the IBM Plex Sans 1.1.0 OTF folder")


class Goldens(unittest.TestCase):
    def assertGolden(self, outputs):
        self.assertTrue(outputs)
        for rel, text in outputs.items():
            with self.subTest(rel):
                self.assertEqual(text, (HERE / rel).read_text(), rel)

    def test_mark_in_each_colour(self):
        for colour, fill in colours.LOGO.items():
            with self.subTest(colour):
                self.assertEqual(mark.svg(fill), (HERE / "svg" / f"opendap-mark-{colour}.svg").read_text())

    def test_construction_drawing(self):
        self.assertEqual(mark.construction(), (HERE / "opendap-mark-construction.svg").read_text())

    def test_app_icon(self):
        import io
        buf = io.BytesIO()
        app_icon.icon().save(buf, format="PNG")
        self.assertEqual(buf.getvalue(), (HERE.parent / "export" / "brand" / "opendap-app-icon.png").read_bytes())

    @needs_plex
    def test_opendap_marks(self):
        self.assertGolden(logo.outputs(PLEX))

    @needs_plex
    def test_product_lockups(self):
        import products
        self.assertGolden(products.outputs(PLEX))


class Colours(unittest.TestCase):
    def test_logo_colours_come_from_the_tokens(self):
        self.assertEqual(colours.LOGO, {"light": "#F5F6F4", "dark": "#111315", "accent": "#FF6A2A", "current": "currentColor"})

    def test_alias_resolves(self):
        self.assertEqual(colours.token("semantic.bg"), colours.token("color.graphite.950"))


if __name__ == "__main__":
    unittest.main()
