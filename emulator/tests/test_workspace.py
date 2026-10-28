"""workspace: another repository's checkout; one merged into the blackbird root is its folder there (RFC 0017)."""
import os
import pathlib
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent.parent / "lib"))
from odap_vdev import workspace  # noqa: E402

CLEAN = {k: v for k, v in os.environ.items() if not (k.startswith("ODAP_") and k.endswith("_DIR"))}


class RepoDir(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.ws = pathlib.Path(self.tmp.name)
        patcher = mock.patch.dict(os.environ, CLEAN, clear=True)
        patcher.start()
        self.addCleanup(patcher.stop)

    def at(self, repo):
        p = mock.patch.object(workspace, "REPO", self.ws / repo)
        p.start()
        self.addCleanup(p.stop)

    def test_a_repository_listed_in_repos_tsv(self):
        (self.ws / "public/blackbird-os").mkdir(parents=True)
        (self.ws / "public/virtual-devices").mkdir(parents=True)
        (self.ws / "repos.tsv").write_text("path\trepository\npublic/blackbird-os\topendap-project/blackbird-os\n")
        self.at("public/virtual-devices")
        self.assertEqual(workspace.repo_dir("blackbird-os"), self.ws / "public/blackbird-os")

    def test_a_merged_repository_in_the_same_checkout(self):
        for sub in (".git", "os", "emulator"):
            (self.ws / "blackbird" / sub).mkdir(parents=True)
        self.at("blackbird/emulator")
        self.assertEqual(workspace.repo_dir("blackbird-os"), self.ws / "blackbird/os")

    def test_a_merged_repository_through_its_root_row(self):
        (self.ws / "public/blackbird/os").mkdir(parents=True)
        (self.ws / "elsewhere").mkdir()
        (self.ws / "repos.tsv").write_text("path\trepository\npublic/blackbird\topendap-project/blackbird\n")
        self.at("elsewhere")
        self.assertEqual(workspace.repo_dir("blackbird-os"), self.ws / "public/blackbird/os")

    def test_an_unknown_repository_is_not_guessed(self):
        (self.ws / "repos.tsv").write_text("path\trepository\n")
        self.at("elsewhere")
        with self.assertRaises(workspace.NotFound):
            workspace.repo_dir("nothing")


if __name__ == "__main__":
    unittest.main()
