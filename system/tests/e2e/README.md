# End-to-end scenarios (L3)

The composer's primitives driven on the virtual T1 as a person drives them, and checked on what its display shows ([ADR 0014](../../docs/adr/0014-composition-and-test-levels.md), level L3). Each scenario names the cases of the [primitives matrix](../../docs/verification/primitives-matrix.md) it covers; a case seen only by hand (`L3m`) becomes `L3` once a scenario here runs it again on demand.

## Running

The virtual T1 running, with the UI primitives activity on screen:

```sh
odap-vdev start
odap-vdev ui activity odap-testbed
tests/e2e/run.py                       # every scenario
tests/e2e/run.py --only keyboard       # one or a few
```

Each run prints one line per scenario (`ok` or `FAIL`, the matrix cases, the time) and what failed, saves every capture in `--out` (default `target/e2e/`) with `report.json`, and exits 1 when a scenario failed.

## How it works

| File | Purpose |
|---|---|
| `device.py` | The device as a scenario sees it: the controls by role, touch, the display, `ssh`, over one QMP connection and virtual-devices' own library (found through `tokens.py`) |
| `tokens.py` | Other repositories' checkouts (`ODAP_<NAME>_DIR` or the workspace's `repos.tsv`), and the design tokens the scenarios look for on the display (the focus ring's colour), read from Blackbird UI's published `export/css/tokens.css`; `test_tokens.py` checks them |
| `png.py` | The display's captures read with the standard library only, and where a colour sits in them (the focus ring is the focus token's colour) |
| `scenarios.py` | The scenarios, each a function that returns what failed, and `ALL`: their order and the matrix cases each covers |
| `run.py` | Runs them and reports |

What a scenario checks is what the display shows (the ring's place, the share of the display lit or drawn as a blueprint), never the code's state: the same evidence a person verifying by hand would take.
