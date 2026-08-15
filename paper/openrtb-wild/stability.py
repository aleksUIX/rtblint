#!/usr/bin/env python3
"""Cross-wave stability of the conformance rates.

Every headline number in the study comes from one capture wave. This
compares waves to answer two questions the paper must answer before it can
claim anything about a population rather than an afternoon:

  1. Is the site-level rate stable across days?
  2. Are the per-endpoint rates stable, or is the aggregate an average over
     endpoints that swing independently?

Treats the original captures as wave 0 so the first repeat is immediately
comparable.

Usage: python3 stability.py
"""

import csv
import statistics
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).parent
WAVES = HERE / "waves"

BASELINE = {"A": HERE / "dataset_sampleA", "B": HERE / "dataset_sampleB"}


def rates(dataset_dir):
    """Returns (per-side pooled rate, per-endpoint request rates, site count)."""
    sides = defaultdict(lambda: {"n": 0, "inv": 0})
    eps = defaultdict(lambda: {"n": 0, "inv": 0})
    sites = set()
    path = Path(dataset_dir) / "payloads.csv"
    if not path.exists():
        return None
    for r in csv.DictReader(open(path)):
        sites.add(r["site"])
        s = sides[r["side"]]
        s["n"] += 1
        if r["valid"] == "False":
            s["inv"] += 1
        if r["side"] == "request":
            e = eps[r["endpoint"]]
            e["n"] += 1
            if r["valid"] == "False":
                e["inv"] += 1
    return {
        "sides": {k: 100 * v["inv"] / v["n"] for k, v in sides.items() if v["n"]},
        "n": {k: v["n"] for k, v in sides.items()},
        "endpoints": {k: 100 * v["inv"] / v["n"] for k, v in eps.items() if v["n"] >= 25},
        "sites": len(sites),
    }


def main():
    for sample in ("A", "B"):
        waves = [("wave0 (paper)", rates(BASELINE[sample]))]
        for d in sorted(WAVES.glob(f"*-sample{sample}-dataset")):
            waves.append((d.name.split("-sample")[0], rates(d)))
        waves = [(n, r) for n, r in waves if r]
        if len(waves) < 2:
            print(f"Sample {sample}: only {len(waves)} wave(s); run ./repeat.sh for more.\n")
            continue

        print(f"=== Sample {sample}: {len(waves)} waves ===")
        print(f"{'wave':22s} {'sites':>6s} {'req n':>7s} {'req inv%':>9s} {'resp n':>7s} {'resp inv%':>10s}")
        for name, r in waves:
            print(f"{name:22s} {r['sites']:6d} {r['n'].get('request',0):7d} "
                  f"{r['sides'].get('request',float('nan')):8.1f}% "
                  f"{r['n'].get('response',0):7d} {r['sides'].get('response',float('nan')):9.1f}%")

        for side in ("request", "response"):
            vals = [r["sides"][side] for _, r in waves if side in r["sides"]]
            if len(vals) >= 2:
                spread = max(vals) - min(vals)
                sd = statistics.stdev(vals)
                print(f"  {side}s: mean {statistics.mean(vals):.1f}%, sd {sd:.1f}, "
                      f"range {min(vals):.1f}-{max(vals):.1f} (spread {spread:.1f} points)")

        # endpoint-level drift: endpoints present in every wave
        common = set(waves[0][1]["endpoints"])
        for _, r in waves[1:]:
            common &= set(r["endpoints"])
        if common:
            print(f"\n  endpoints with >=25 requests in all waves: {len(common)}")
            print(f"  {'endpoint':40s} " + " ".join(f"{n[:8]:>9s}" for n, _ in waves) + "   spread")
            drifts = []
            for e in sorted(common):
                vals = [r["endpoints"][e] for _, r in waves]
                spread = max(vals) - min(vals)
                drifts.append(spread)
                print(f"  {e:40s} " + " ".join(f"{v:8.1f}%" for v in vals) + f" {spread:8.1f}")
            print(f"  median per-endpoint spread across waves: {statistics.median(drifts):.1f} points")
        print()


if __name__ == "__main__":
    main()
