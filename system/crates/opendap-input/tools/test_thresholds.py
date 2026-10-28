"""thresholds.py: thresholds.rs and the input-events §7 table from thresholds.toml."""
import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import thresholds  # noqa: E402

DATA = {
    "module_doc": "\nThe thresholds.",
    "constant": [
        {"name": "SLOP_PX", "type": "f32", "value": 10.0, "doc": "\nHow far, in\ndesign pixels."},
        {"name": "HOLD_MS", "type": "u64", "value": 2000},
        {"name": "FACTOR", "type": "f64", "value": 0.8},
        {"name": "EVERY_MS", "type": "u64", "value": 40},
        {"name": "AGAIN_MS", "type": "u64", "same_as": "HOLD_MS", "doc": "The same."},
    ],
    "row": [{"threshold": "Slop", "default": "{SLOP_PX} px, {HOLD_MS|s} s, {EVERY_MS|per_s}/s × {FACTOR}",
             "from": "Android"}],
}


class Thresholds(unittest.TestCase):
    def test_rust_constants_keep_their_docs_and_types(self):
        rs = thresholds.render_rust(DATA)
        self.assertTrue(rs.startswith("// @generated"))
        self.assertIn("//! The thresholds.\n", rs)
        self.assertIn("/// How far, in\n/// design pixels.\npub const SLOP_PX: f32 = 10.0;\n", rs)
        self.assertIn("\npub const HOLD_MS: u64 = 2000;\n", rs)
        self.assertIn("\npub const FACTOR: f64 = 0.8;\n", rs)
        self.assertIn("/// The same.\npub const AGAIN_MS: u64 = HOLD_MS;\n", rs)

    def test_table_formats_values(self):
        table = thresholds.render_table(DATA)
        self.assertEqual(table.splitlines()[-1], "| Slop | 10 px, 2 s, 25/s × 0.8 | Android |")

    def test_unknown_names_and_bad_types_fail(self):
        with self.assertRaises(KeyError):
            thresholds.render_table({**DATA, "row": [{"threshold": "x", "default": "{NOPE}", "from": "y"}]})
        with self.assertRaises(ValueError):
            thresholds.render_rust({**DATA, "constant": [{"name": "X", "type": "u64", "value": 1.5}]})

    def test_splice_replaces_only_between_markers(self):
        doc = f"a\n{thresholds.BEGIN}\nold\n{thresholds.END}\nb\n"
        self.assertEqual(thresholds.splice(doc, "new\n"), f"a\n{thresholds.BEGIN}\nnew\n{thresholds.END}\nb\n")

    def test_committed_files_are_current(self):
        self.assertEqual(thresholds.stale(), [])


if __name__ == "__main__":
    unittest.main()
