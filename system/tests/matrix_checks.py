#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 The OpenDAP Project
# SPDX-License-Identifier: GPL-3.0-or-later
"""Names, in the primitives matrix, the checks that prove each case again (opendap.verification/v1
`checks`), from the suites themselves: an end-to-end scenario (tests/e2e/scenarios.py) lists the
cases it covers, and an interface check (crates/odap-testbed/tests/interface.rs) whose name begins
with a case's id and a colon covers that case. `odapkit verify collect` then joins their results.

  tests/matrix_checks.py [--check]

Without --check it writes the `checks` of every case it finds checks for (other cases keep theirs);
with --check it only says whether the matrix names them all, and exits 1 when not.
"""

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MATRIX = ROOT / "docs/verification/primitives-matrix.toml"
INTERFACE = ROOT / "crates/odap-testbed/tests/interface.rs"


def e2e_checks():
    sys.path.insert(0, str(ROOT / "tests/e2e"))
    from scenarios import ALL  # noqa: E402

    out = {}
    for name, _, cases in ALL:
        for case in cases:
            out.setdefault(case, []).append(f"e2e/{name}")
    return out


def interface_checks(case_ids):
    # The checks' names as written: run.check("name", …) or run.check(&format!("name…", …), …).
    names = re.findall(r'run\.check\(\s*(?:&format!\()?"([^"]+)"', INTERFACE.read_text())
    out = {}
    for n in names:
        head = n.split(":", 1)[0]
        if ":" in n and head in case_ids and "{" not in n:
            out.setdefault(head, []).append(f"interface/{n}")
    return out


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--check", action="store_true")
    a = p.parse_args()
    text = MATRIX.read_text()
    head, *blocks = text.split("[[case]]")
    ids = [re.search(r'^id = "([^"]+)"$', b, re.M).group(1) for b in blocks]
    found = {}
    for source in (e2e_checks(), interface_checks(set(ids))):
        for case, checks in source.items():
            found.setdefault(case, []).extend(c for c in checks if c not in found.get(case, []))
    stale, out = [], []
    for case_id, b in zip(ids, blocks):
        want = sorted(found.get(case_id, []))
        have = re.search(r'^checks = (\[.*\])$', b, re.M)
        line = "checks = [" + ", ".join(f'"{c}"' for c in want) + "]"
        if want and (not have or have.group(0) != line):
            stale.append(case_id)
            b = re.sub(r'^checks = \[.*\]\n', "", b, flags=re.M)
            b = re.sub(r'^(levels = .*)$', r"\1\n" + line.replace("\\", "\\\\"), b, count=1, flags=re.M)
        out.append(b)
    if a.check:
        if stale:
            print(f"matrix_checks: {len(stale)} cases do not name their checks: {', '.join(stale)}")
            return 1
        return 0
    MATRIX.write_text("[[case]]".join([head, *out]))
    print(f"matrix_checks: {sum(1 for i in ids if i in found)} cases name their checks ({len(stale)} updated)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
