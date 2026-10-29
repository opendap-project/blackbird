# SPDX-FileCopyrightText: 2026 The OpenDAP Project
# SPDX-License-Identifier: GPL-3.0-or-later
"""Other repositories' checkouts (ODAP_<NAME>_DIR, else the workspace's repos.tsv; a repository merged
into the blackbird root is only its folder there), and the design tokens the scenarios look for on the
display, read from Blackbird UI's published tokens (export/css/tokens.css) as the composer reads
them: never typed by hand."""

import os
import re
import sys
from pathlib import Path


# Repositories merged into a root (RFC 0017 §7): name → (root, folder).
MERGED = {"blackbird-os": ("blackbird", "os"), "blackbird-ui": ("blackbird", "ui"),
          "system": ("blackbird", "system"), "virtual-devices": ("blackbird", "emulator")}


def _row(start, name):
    """The checkout repos.tsv lists for `name` above `start`, or None."""
    for up in (start, *start.parents):
        tsv = up / "repos.tsv"
        if tsv.exists():
            for line in tsv.read_text().splitlines()[1:]:
                path, slug = line.split("\t")[:2]
                if slug.endswith("/" + name) and (up / path).is_dir():
                    return up / path
            return None
    return None


def repo(name, start=None):
    """The checkout of the GitHub repository `name`, or exit with how to provide it. A merged repository is
    only its folder in the checkout that holds `start`, else in its root's checkout (ODAP_BLACKBIRD_DIR or
    the root's row); its old variable and row are not read any more (HL-91)."""
    start = Path(start or Path(__file__).resolve().parent)
    if name in MERGED:
        root, folder = MERGED[name]
        var = "ODAP_" + root.upper() + "_DIR"
        top = next((d for d in (start, *start.parents) if (d / ".git").exists()), None)
        base, row = os.environ.get(var), _row(start, root)
        for cand in (top and top / folder, base and Path(base) / folder, row and row / folder):
            if cand and cand.is_dir():
                return cand
        sys.exit(f"e2e: {name} not found (the {folder}/ folder of {root}: {var}, or a workspace checkout)")
    var = "ODAP_" + name.upper().replace("-", "_") + "_DIR"
    if os.environ.get(var):
        return Path(os.environ[var])
    found = _row(start, name)
    if found is None:
        sys.exit(f"e2e: {name} not found ({var}, or a workspace checkout)")
    return found


def colour(name, css=None):
    """The token `--<name>` as an (r, g, b) tuple of 8-bit sRGB."""
    css = css or repo("blackbird-ui") / "export" / "css" / "tokens.css"
    m = re.search(rf"^\s*--{re.escape(name)}:\s*#([0-9A-Fa-f]{{6}});", css.read_text(), re.M)
    if not m:
        raise KeyError(f"{css} has no colour --{name}")
    return tuple(int(m.group(1)[i:i + 2], 16) for i in (0, 2, 4))
