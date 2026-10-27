"""key_names.py: the KEY_NAMES table from the kernel header."""
import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import key_names  # noqa: E402

HEADER = """\
#define KEY_RESERVED		0
#define KEY_ESC			1
#define KEY_1			2
#define KEY_MUTE		113
#define KEY_MIN_INTERESTING	KEY_MUTE
#define KEY_BRIGHTNESS_ZERO	244
#define KEY_DISPLAY_OFF		245
#define KEY_WIMAX		246
#define KEY_WWAN		246
#define KEY_MAX			0x2ff
#define BTN_LEFT		0x110
"""


class KeyNames(unittest.TestCase):
    def test_first_name_per_code_and_no_limits_or_buttons(self):
        self.assertEqual(key_names.names(HEADER),
                         [(1, "KEY_ESC"), (2, "KEY_1"), (113, "KEY_MUTE"), (244, "KEY_BRIGHTNESS_ZERO"),
                          (245, "KEY_DISPLAY_OFF"), (246, "KEY_WIMAX")])

    def test_render_has_the_generated_header(self):
        out = key_names.render([(1, "KEY_ESC")])
        self.assertTrue(out.startswith("// @generated"))
        self.assertIn('    (1, "KEY_ESC"),\n', out)


if __name__ == "__main__":
    unittest.main()
