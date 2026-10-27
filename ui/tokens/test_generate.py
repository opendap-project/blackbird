# SPDX-FileCopyrightText: 2026 The OpenDAP Project
# SPDX-License-Identifier: MPL-2.0
"""Tests of the token generator's reference output (generate.py). Run: python3 -m unittest tokens/test_generate.py"""
import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import generate  # noqa: E402

REFERENCE = generate.HERE.parent / "docs" / "reference" / "tokens.md"


class Reference(unittest.TestCase):
    def setUp(self):
        self.outputs = generate.generate()
        self.text = self.outputs[REFERENCE]

    def test_reference_is_an_output(self):
        self.assertIn(REFERENCE, self.outputs)
        self.assertIn("Do not edit", self.text.splitlines()[0])

    def test_every_token_is_listed_once(self):
        for path, _kind, _value in generate.flatten(generate.load()):
            self.assertEqual(self.text.count(f"| `{'.'.join(path)}` |"), 1, ".".join(path))

    def test_alias_shows_value_and_source(self):
        self.assertIn("| `semantic.bg` | `#111315` (= `color.graphite.950`) | `--bg` | `Tokens.bg` |", self.text)

    def test_value_without_slint_type_is_marked(self):
        line = next(row for row in self.text.splitlines() if row.startswith("| `plasma.gradient.field` |"))
        self.assertIn("| — | — |", line)  # neither CSS nor Slint: drawn by components
        self.assertIn("`#FFFDF9` 98 % at 0 %", line)

    def test_one_section_per_group(self):
        for group in generate.load():
            if not group.startswith("$"):
                self.assertIn(f"\n## `{group}`\n", self.text)


if __name__ == "__main__":
    unittest.main()
