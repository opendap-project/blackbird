# SPDX-FileCopyrightText: 2026 The OpenDAP Project
# SPDX-License-Identifier: GPL-3.0-or-later
"""tokens.py: the design tokens the scenarios look for, from Blackbird UI's published tokens."""
import os
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

    def test_finds_a_merged_repository_in_the_same_checkout(self):
        with tempfile.TemporaryDirectory() as d:
            top = pathlib.Path(d) / "blackbird"
            for sub in (".git", "ui", "system/tests/e2e"):
                (top / sub).mkdir(parents=True)
            self.assertEqual(tokens.repo("blackbird-ui", top / "system/tests/e2e"), top / "ui")

    def test_finds_a_merged_repository_through_its_root_row(self):
        with tempfile.TemporaryDirectory() as d:
            ws = pathlib.Path(d)
            (ws / "public/blackbird/emulator").mkdir(parents=True)
            (ws / "public/other").mkdir()
            (ws / "repos.tsv").write_text("path\trepository\npublic/blackbird\topendap-project/blackbird\n")
            self.assertEqual(tokens.repo("virtual-devices", ws / "public/other"), ws / "public/blackbird/emulator")


    def test_a_merged_repository_ignores_its_old_variable_and_row(self):
        with tempfile.TemporaryDirectory() as d:
            ws = pathlib.Path(d)
            (ws / "public/virtual-devices").mkdir(parents=True)
            (ws / "public/other").mkdir()
            (ws / "repos.tsv").write_text("path\trepository\npublic/virtual-devices\topendap-project/virtual-devices\n")
            old = os.environ.pop("ODAP_VIRTUAL_DEVICES_DIR", None), os.environ.pop("ODAP_BLACKBIRD_DIR", None)
            os.environ["ODAP_VIRTUAL_DEVICES_DIR"] = str(ws / "public/virtual-devices")
            try:
                with self.assertRaises(SystemExit):
                    tokens.repo("virtual-devices", ws / "public/other")
            finally:
                os.environ.pop("ODAP_VIRTUAL_DEVICES_DIR")
                for k, v in zip(("ODAP_VIRTUAL_DEVICES_DIR", "ODAP_BLACKBIRD_DIR"), old):
                    if v is not None:
                        os.environ[k] = v

if __name__ == "__main__":
    unittest.main()
