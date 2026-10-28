"""instance: the runner tells toolkit-host's patched QEMU (odapkit qemu) from a stock one by its version line."""
import os
import pathlib
import stat
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent.parent / "lib"))
from odap_vdev import instance  # noqa: E402


def fake_qemu(folder, first_line):
    path = pathlib.Path(folder) / "qemu-system-aarch64"
    path.write_text(f"#!/bin/sh\necho '{first_line}'\necho 'Copyright (c) 2003-2026 Fabrice Bellard and the QEMU Project developers'\n")
    path.chmod(path.stat().st_mode | stat.S_IXUSR)
    return str(path)


class SoundPatches(unittest.TestCase):
    def test_the_patched_build_names_its_patch_set(self):
        with tempfile.TemporaryDirectory() as d:
            self.assertTrue(instance.carries_sound_patches(fake_qemu(d, "QEMU emulator version 11.1.2 (opendap1)")))

    def test_a_stock_qemu_is_not_patched(self):
        with tempfile.TemporaryDirectory() as d:
            self.assertFalse(instance.carries_sound_patches(fake_qemu(d, "QEMU emulator version 11.1.2")))

    def test_another_package_version_is_not_ours(self):
        with tempfile.TemporaryDirectory() as d:
            self.assertFalse(instance.carries_sound_patches(fake_qemu(d, "QEMU emulator version 11.1.2 (Debian 1:11.1.2+ds-1)")))

    def test_a_missing_qemu_is_not_patched(self):
        self.assertFalse(instance.carries_sound_patches(os.path.join(tempfile.gettempdir(), "no-such-qemu")))


if __name__ == "__main__":
    unittest.main()
