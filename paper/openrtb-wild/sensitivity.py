#!/usr/bin/env python3
"""Sensitivity of the headline conformance rate to the two disputable classes.

OpenRTB says extensions "should" be named `ext` (a recommendation, not a
requirement) and separately requires receivers to tolerate unexpected
fields. A reader can therefore argue that unknown non-`ext` fields, and
legacy placements of fields that later moved, are ecosystem practice rather
than violations. This recomputes the invalid rate with those classes
removed, so the headline can be read under the most hostile definition.

Usage: python3 sensitivity.py
"""

import csv
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).parent
DISPUTABLE = ["openrtb.field.undefined", "openrtb.field.moved"]


def main():
    print(f"{'sample':10s} {'side':9s} {'n':>6s} {'as reported':>12s} "
          f"{'excl. undefined':>16s} {'excl. undef+moved':>18s}")
    for ds, label in (("dataset_sampleA", "A random"), ("dataset_sampleB", "B purposive")):
        sides = defaultdict(list)
        for r in csv.DictReader(open(HERE / ds / "payloads.csv")):
            sides[r["side"]].append(r["payload_id"])
        errs = defaultdict(set)
        for r in csv.DictReader(open(HERE / ds / "issues.csv")):
            if r["severity"] == "error":
                errs[r["payload_id"]].add(r["rule"])
        valid = {r["payload_id"]: r["valid"]
                 for r in csv.DictReader(open(HERE / ds / "payloads.csv"))}
        for side in ("request", "response"):
            ids = sides[side]
            n = len(ids)
            base = sum(1 for p in ids if valid[p] == "False")
            no_und = sum(1 for p in ids if errs[p] - {DISPUTABLE[0]})
            core = sum(1 for p in ids if errs[p] - set(DISPUTABLE))
            print(f"{label:10s} {side:9s} {n:6d} {100*base/n:11.1f}% "
                  f"{100*no_und/n:15.1f}% {100*core/n:17.1f}%")


if __name__ == "__main__":
    main()
