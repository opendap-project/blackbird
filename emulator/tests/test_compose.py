"""compose: the device description carries the device's controls only; the window's host keys stay runner-side."""
import pathlib
import sys
import tomllib
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent.parent / "lib"))
from odap_vdev import compose  # noqa: E402


class VirtualT1(unittest.TestCase):
    def setUp(self):
        self.model = compose.load(compose.model_path("virtual-t1"))

    def test_description_has_one_control_per_physical_key(self):
        d = tomllib.loads(compose.description(self.model))
        roles = [c["role"] for c in d["controls"]]
        self.assertEqual(sorted(roles), ["action", "main", "volume-down", "volume-up"])
        self.assertFalse({"KEY_SPACE", "KEY_UP", "KEY_DOWN", "KEY_P"} & {c["code"] for c in d["controls"]})
        self.assertFalse(any("qcode" in c for c in d["controls"]))

    def test_frame_gives_the_window_its_host_keys(self):
        f = compose.frame(self.model)
        self.assertEqual(sorted((k["key"], k["role"]) for k in f["host_keys"]),
                         [("down", "volume-down"), ("p", "main"), ("spc", "action"), ("up", "volume-up")])

    def test_injection_still_finds_one_qcode_per_role(self):
        by_role = {c["role"]: c["qcode"] for c in compose.controls(self.model)}
        self.assertEqual(by_role, {"action": "audioplay", "volume-up": "volumeup", "volume-down": "volumedown", "main": "power"})


    def test_frame_gives_the_window_the_layers_probes(self):
        f = compose.frame(self.model)
        names = [p["name"] for p in f["probes"]]
        self.assertEqual(len(names), len(set(names)), "probe names are unique")
        self.assertTrue({"soc-dac", "mic-adc", "status-led", "ambient-light", "battery"} <= set(names))
        for p in f["probes"]:
            self.assertIn(p["kind"], compose.PROBE_KINDS)
            self.assertIn(p["source"], compose.PROBE_SOURCES)
            self.assertTrue(p["what"])

    def test_probes_stay_out_of_the_device_description(self):
        self.assertNotIn("probes", tomllib.loads(compose.description(self.model)))


if __name__ == "__main__":
    unittest.main()
