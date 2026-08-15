#!/usr/bin/env python3
"""Does client-side conformance track the Prebid.js version a site runs?

Section 9b found the same SSP endpoints producing materially more violations
on the random sample than on the major-publisher sample. Since requests are
built client-side, the obvious candidate is the age of the builder: sites on
older Prebid releases should emit more legacy field placements.

Joins the per-site Prebid version recorded during capture to the per-site
request conformance rate from the published datasets.

Usage: python3 prebid_version.py
"""

import csv
import json
import statistics
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).parent
CAPTURES = [
    (HERE / "captures/full1.jsonl", "dataset_sampleB"),
    (HERE / "captures/tranco-deep.jsonl", "dataset_sampleA"),
]
LEGACY_RULES = {"openrtb.field.moved", "openrtb.field.deprecated"}


def main():
    site_version = {}
    for cap, _ in CAPTURES:
        for line in open(cap):
            rec = json.loads(line)
            if rec.get("kind") == "site-meta" and rec.get("pbjsVersion"):
                site_version[rec["site"]] = rec["pbjsVersion"].lstrip("v")

    stats = defaultdict(lambda: {"n": 0, "invalid": 0, "legacy": 0, "issues": 0})
    for _, ds in CAPTURES:
        for r in csv.DictReader(open(HERE / ds / "payloads.csv")):
            if r["side"] != "request":
                continue
            s = stats[r["site"]]
            s["n"] += 1
            s["issues"] += int(r["n_issues"])
            if r["valid"] == "False":
                s["invalid"] += 1
        for r in csv.DictReader(open(HERE / ds / "issues.csv")):
            if r["side"] == "request" and r["rule"] in LEGACY_RULES:
                stats[r["site"]]["legacy"] += 1

    rows = []
    for site, s in stats.items():
        v = site_version.get(site)
        if not v or s["n"] < 5:
            continue
        try:
            major = int(v.split(".")[0])
        except ValueError:
            continue
        rows.append({
            "site": site, "major": major, "n": s["n"],
            "invalid_pct": 100 * s["invalid"] / s["n"],
            "legacy_per_payload": s["legacy"] / s["n"],
        })

    print(f"sites with a detected Prebid version and >=5 requests: {len(rows)}\n")
    print(f"{'Prebid major':>13s} {'sites':>6s} {'requests':>9s} {'median invalid%':>16s} {'legacy fields/payload':>22s}")
    by_major = defaultdict(list)
    for r in rows:
        by_major[r["major"]].append(r)
    for major in sorted(by_major):
        g = by_major[major]
        print(f"{major:>13d} {len(g):>6d} {sum(x['n'] for x in g):>9d} "
              f"{statistics.median(x['invalid_pct'] for x in g):>15.1f}% "
              f"{statistics.mean(x['legacy_per_payload'] for x in g):>22.2f}")

    older = [r for r in rows if r["major"] <= 9]
    newer = [r for r in rows if r["major"] >= 10]
    if older and newer:
        print(f"\nPrebid <= 9  ({len(older)} sites): median invalid "
              f"{statistics.median(r['invalid_pct'] for r in older):.1f}%, "
              f"legacy {statistics.mean(r['legacy_per_payload'] for r in older):.2f}/payload")
        print(f"Prebid >= 10 ({len(newer)} sites): median invalid "
              f"{statistics.median(r['invalid_pct'] for r in newer):.1f}%, "
              f"legacy {statistics.mean(r['legacy_per_payload'] for r in newer):.2f}/payload")

    # Spearman rank correlation between version and legacy-field density
    n = len(rows)
    if n > 5:
        def ranks(vals):
            order = sorted(range(len(vals)), key=lambda i: vals[i])
            rk = [0.0] * len(vals)
            for pos, i in enumerate(order):
                rk[i] = pos + 1
            return rk
        rv = ranks([r["major"] for r in rows])
        rl = ranks([r["legacy_per_payload"] for r in rows])
        mv, ml = statistics.mean(rv), statistics.mean(rl)
        num = sum((a - mv) * (b - ml) for a, b in zip(rv, rl))
        den = (sum((a - mv) ** 2 for a in rv) * sum((b - ml) ** 2 for b in rl)) ** 0.5
        print(f"\nSpearman(Prebid major, legacy fields per payload) = {num/den:+.3f}  (n={n} sites)")


if __name__ == "__main__":
    main()
