#!/usr/bin/env python3
"""Prevalence analysis over Sample A (the Tranco-derived random sample).

Reports the measurement funnel (sampled -> reachable -> header bidding ->
OpenRTB observed on the wire) with binomial confidence intervals, and the
endpoint census among detected sites. This is the only sample that supports
unbiased prevalence statements; Sample B (sites.txt) is purposive and is used
for per-endpoint conformance depth only.

Usage: python3 prevalence.py detect/tranco.jsonl
"""

import json
import math
import sys
from collections import Counter
from pathlib import Path

FRAME = Path(__file__).parent / "frame_tranco.json"


def wilson(k, n, z=1.96):
    """Wilson score interval: well behaved for small proportions."""
    if n == 0:
        return (0.0, 0.0)
    p = k / n
    d = 1 + z * z / n
    c = p + z * z / (2 * n)
    s = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n))
    return (100 * (c - s) / d, 100 * (c + s) / d)


def main(path):
    recs = [json.loads(l) for l in open(path)]
    by_site = {r["site"]: r for r in recs}
    recs = list(by_site.values())

    ranks = {}
    if FRAME.exists():
        ranks = {e["domain"]: e["rank"] for e in json.load(open(FRAME))["sample"]}

    n = len(recs)
    reachable = [r for r in recs if str(r.get("status", "")).startswith("ok")]
    pbjs = [r for r in reachable if r.get("hasPbjs")]
    ortb = [r for r in reachable if (r.get("ortbRequests") or 0) > 0]

    def line(label, k, denom):
        lo, hi = wilson(k, denom)
        print(f"  {label:44s} {k:5d} / {denom:5d}  = {100*k/denom:5.1f}%  95% CI [{lo:.1f}, {hi:.1f}]")

    print(f"Sample A funnel (Tranco random sample, n={n})")
    line("reachable (page loaded)", len(reachable), n)
    line("header bidding present (Prebid detected)", len(pbjs), len(reachable))
    line("OpenRTB observed on the wire", len(ortb), len(reachable))
    line("  ... among header-bidding sites", len([r for r in pbjs if (r.get("ortbRequests") or 0) > 0]), len(pbjs))

    fails = Counter()
    for r in recs:
        s = str(r.get("status", ""))
        if not s.startswith("ok"):
            fails[s.split(":")[0] + ":" + s.split("net::")[-1][:28]] += 1
    if fails:
        print(f"\nunreachable ({n-len(reachable)}), top reasons:")
        for reason, c in fails.most_common(6):
            print(f"  {c:4d}  {reason}")

    if ranks:
        print("\nprevalence by Tranco rank band (header bidding, among reachable):")
        bands = [(1, 5000), (5001, 15000), (15001, 30000), (30001, 50000)]
        for lo_r, hi_r in bands:
            band = [r for r in reachable if lo_r <= ranks.get(r["site"], 10**9) <= hi_r]
            if not band:
                continue
            k = len([r for r in band if r.get("hasPbjs")])
            lo, hi = wilson(k, len(band))
            print(f"  rank {lo_r:>6}-{hi_r:<6} {k:4d} / {len(band):4d} = {100*k/len(band):5.1f}%  95% CI [{lo:.1f}, {hi:.1f}]")

    eps = Counter()
    for r in recs:
        for e in r.get("endpoints", []):
            eps[e] += 1
    if eps:
        print(f"\nendpoint census: {len(eps)} distinct endpoints across {len(ortb)} sites")
        for host, c in eps.most_common(20):
            print(f"  {c:4d} sites  {host}")

    vers = Counter(r.get("pbjsVersion") for r in pbjs if r.get("pbjsVersion"))
    if vers:
        major = Counter(v.lstrip("v").split(".")[0] for v in vers)
        print(f"\nPrebid.js major versions among {len(pbjs)} detections: {dict(sorted(major.items()))}")

    json.dump(
        {
            "sample": "tranco-random",
            "n": n,
            "reachable": len(reachable),
            "header_bidding": len(pbjs),
            "ortb_observed": len(ortb),
            "endpoints": dict(eps),
            "prebid_versions": dict(vers),
        },
        open(Path(__file__).parent / "prevalence_results.json", "w"), indent=1,
    )
    print("\nwritten: prevalence_results.json")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "detect/tranco.jsonl")
