# SPDX-FileCopyrightText: 2026 The OpenDAP Project
# SPDX-License-Identifier: GPL-3.0-or-later
"""tokens.py: the design tokens the scenarios look for, from Blackbird UI's published tokens."""
import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import tokens  # noqa: E402


class Tokens(unittest.TestCase):
    def test_the_focus_ring_keeps_its_colour(self):
        # The value device.py typed by hand before it read the token.
        self.assertEqual(tokens.colour("focus"), (255, 135, 77))

    def test_reads_a_colour_and_refuses_a_missing_one(self):
        with tempfile.NamedTemporaryFile("w", suffix=".css") as f:
            f.write(":root {\n  --focus: #FF874D;\n  --bg: #111315;\n}\n")
            f.flush()
            self.assertEqual(tokens.colour("bg", pathlib.Path(f.name)), (17, 19, 21))
            with self.assertRaises(KeyError):
                tokens.colour("nope", pathlib.Path(f.name))

    def test_finds_repositories_through_the_workspace(self):
        self.assertTrue((tokens.repo("virtual-devices") / "lib").is_dir())


if __name__ == "__main__":
    unittest.main()
