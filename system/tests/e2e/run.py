#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 The OpenDAP Project
# SPDX-License-Identifier: GPL-3.0-or-later
"""Runs the end-to-end scenarios on a running virtual device (ADR 0014, level L3), with the UI
primitives activity on screen (odap-vdev ui activity ui-primitives). Prints each scenario's result
and the matrix cases it covers, saves the captures, report.json and check-result.json (an
opendap.check-result/v1 document, one result per scenario, id e2e/<scenario>, level V3, for the
verification matrix to collect) in --out, and exits 1 when one failed.

  tests/e2e/run.py [--only NAME ...] [--out DIR] [--model MODEL]
"""

import argparse
import datetime
import json
import os
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from device import Device  # noqa: E402
from scenarios import ALL  # noqa: E402

# Never inherit a hook's GIT_* variables (HL-64): under pre-commit, git exports GIT_DIR, GIT_INDEX_FILE and
# others, and the git that names the tested commit would read the repository being committed.
for _k in [k for k in os.environ if k.startswith("GIT_")]:
    del os.environ[_k]


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--only", nargs="*", help="scenarios to run (default: all)")
    p.add_argument("--out", default="target/e2e", help="captures and report (default: target/e2e)")
    p.add_argument("--model", default="virtual-t1")
    a = p.parse_args()
    chosen = [s for s in ALL if not a.only or s[0] in a.only]
    if a.only and len(chosen) != len(a.only):
        sys.exit(f"e2e: unknown scenario; known: {', '.join(s[0] for s in ALL)}")
    d = Device(a.model, Path(a.out))
    report = []
    for name, run, cases in chosen:
        t = time.monotonic()
        try:
            failed = run(d)
        except Exception as e:  # a scenario that breaks is a failure, the next ones still run
            failed = [f"{type(e).__name__}: {e}"]
        report.append({"scenario": name, "cases": cases, "failed": failed, "seconds": round(time.monotonic() - t, 1)})
        print(f"{'ok  ' if not failed else 'FAIL'} {name} ({', '.join(cases)}) {report[-1]['seconds']} s")
        for f in failed:
            print(f"     {f}")
    (Path(a.out) / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    commit = subprocess.run(["git", "-C", str(Path(__file__).parent), "rev-parse", "--short=12", "HEAD"],
                            capture_output=True, text=True).stdout.strip()
    result = {
        "schema": "opendap.check-result/v1",
        "tool": {"name": "system tests/e2e"},
        "subject": {"repo": "system", "path": "tests/e2e", **({"commit": commit} if commit else {}), "variant": a.model},
        "generated": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "results": [
            {
                "id": f"e2e/{r['scenario']}",
                "level": "V3",
                "verdict": "fail" if r["failed"] else "pass",
                "message": "; ".join(r["failed"]) if r["failed"] else f"{r['seconds']} s, as expected",
                "evidence": [str(Path(a.out).resolve())],
            }
            for r in report
        ],
    }
    (Path(a.out) / "check-result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(f"evidence: {Path(a.out).resolve()}")
    return 1 if any(r["failed"] for r in report) else 0


if __name__ == "__main__":
    sys.exit(main())
