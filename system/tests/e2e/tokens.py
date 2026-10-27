# SPDX-FileCopyrightText: 2026 The OpenDAP Project
# SPDX-License-Identifier: GPL-3.0-or-later
"""Other repositories' checkouts (ODAP_<NAME>_DIR, else the workspace's repos.tsv), and the design
tokens the scenarios look for on the display, read from Blackbird UI's published tokens
(export/css/tokens.css) as the composer reads them: never typed by hand."""

import os
import re
import sys
from pathlib import Path


def repo(name):
    """The checkout of the GitHub repository `name`, or exit with how to provide it."""
    var = "ODAP_" + name.upper().replace("-", "_") + "_DIR"
    if os.environ.get(var):
        return Path(os.environ[var])
    for up in Path(__file__).resolve().parents:
        tsv = up / "repos.tsv"
        if tsv.exists():
            for line in tsv.read_text().splitlines()[1:]:
                path, slug = line.split("\t")[:2]
                if slug.endswith("/" + name):
                    return up / path
    sys.exit(f"e2e: {name} not found ({var}, or a workspace checkout)")


def colour(name, css=None):
    """The token `--<name>` as an (r, g, b) tuple of 8-bit sRGB."""
    css = css or repo("blackbird-ui") / "export" / "css" / "tokens.css"
    m = re.search(rf"^\s*--{re.escape(name)}:\s*#([0-9A-Fa-f]{{6}});", css.read_text(), re.M)
    if not m:
        raise KeyError(f"{css} has no colour --{name}")
    return tuple(int(m.group(1)[i:i + 2], 16) for i in (0, 2, 4))
