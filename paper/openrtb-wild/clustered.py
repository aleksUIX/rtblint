#!/usr/bin/env python3
"""Clustering-aware statistics for the conformance rates.

Payloads are not independent observations: one site fires many auctions and
each auction fans out to many endpoints, so a naive payload-level percentage
overstates precision (pseudo-replication). This computes site-clustered
estimates with a cluster bootstrap, resampling whole sites with replacement.

Usage: python3 clustered.py
"""

import csv
import random
import sys
import statistics
from collections import defaultdict
from pathlib import Path

DATA = Path(__file__).parent / (sys.argv[1] if len(sys.argv) > 1 else "dataset") / "payloads.csv"
B = 5000
SEED = 20260725


def load():
    by_site = defaultdict(lambda: defaultdict(lambda: {"n": 0, "invalid": 0}))
    for r in csv.DictReader(open(DATA)):
        s = by_site[r["site"]][r["side"]]
        s["n"] += 1
        if r["valid"] == "False":
            s["invalid"] += 1
    return by_site


def cluster_bootstrap(sites, side, rng):
    """Payload-weighted rate over a bootstrap resample of whole sites."""
    n = inv = 0
    for _ in range(len(sites)):
        s = sites[rng.randrange(len(sites))]
        d = s.get(side)
        if d:
            n += d["n"]
            inv += d["invalid"]
    return 100 * inv / n if n else None


def main():
    by_site = load()
    sites = list(by_site.values())
    rng = random.Random(SEED)

    print(f"sites: {len(sites)}  (cluster unit)\n")
    for side in ("request", "response"):
        present = [s for s in sites if side in s]
        n = sum(s[side]["n"] for s in present)
        inv = sum(s[side]["invalid"] for s in present)
        pooled = 100 * inv / n

        # per-site rates: each site counts once, regardless of traffic volume
        rates = [100 * s[side]["invalid"] / s[side]["n"] for s in present]
        boots = [b for b in (cluster_bootstrap(present, side, rng) for _ in range(B)) if b is not None]
        boots.sort()
        lo, hi = boots[int(0.025 * len(boots))], boots[int(0.975 * len(boots))]

        print(f"{side}s  ({len(present)} sites, {n:,} payloads)")
        print(f"  pooled payload-level rate : {pooled:5.1f}%  (overstates precision, do not quote a CI on this)")
        print(f"  site-clustered bootstrap  : {pooled:5.1f}%  95% CI [{lo:.1f}, {hi:.1f}]")
        print(f"  per-site rate mean        : {statistics.mean(rates):5.1f}%")
        print(f"  per-site rate median      : {statistics.median(rates):5.1f}%")
        q = sorted(rates)
        print(f"  per-site IQR              : [{q[len(q)//4]:.1f}, {q[3*len(q)//4]:.1f}]")
        print(f"  sites at 0% invalid       : {sum(1 for r in rates if r == 0)}/{len(rates)}")
        print(f"  sites at 100% invalid     : {sum(1 for r in rates if r == 100)}/{len(rates)}\n")


if __name__ == "__main__":
    main()
