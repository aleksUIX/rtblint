#!/usr/bin/env python3
"""Separate adapter-origin defects from publisher-config-origin defects.

A request-side defect can come from two places: the SSP's Prebid adapter
(shared code, same defect wherever that adapter runs) or the publisher's own
configuration and page context (same defect across all of that publisher's
endpoints). The two are distinguishable by how a defect spreads.

For each (rule, path) defect signature we compute:
  endpoint_concentration = share of the defect's occurrences at its top endpoint
  site_spread            = number of distinct sites exhibiting it

An adapter defect appears at ONE endpoint across MANY sites.
A publisher defect appears at MANY endpoints on FEW sites.

Usage: python3 origin.py
"""

import csv
from collections import defaultdict
from pathlib import Path

DATA = Path(__file__).parent / "dataset" / "issues.csv"


def main():
    sig = defaultdict(lambda: {"n": 0, "sites": set(), "endpoints": defaultdict(int)})
    for r in csv.DictReader(open(DATA)):
        if r["side"] != "request":
            continue
        # normalise array indices so imp[0].x and imp[3].x are one signature
        path = "".join("*" if ch.isdigit() else ch for ch in r["path"])
        s = sig[(r["rule"], path)]
        s["n"] += 1
        s["sites"].add(r["site"])
        s["endpoints"][r["endpoint"]] += 1

    rows = []
    for (rule, path), s in sig.items():
        if s["n"] < 20:
            continue
        top_ep, top_n = max(s["endpoints"].items(), key=lambda kv: kv[1])
        rows.append({
            "rule": rule.replace("openrtb.", ""),
            "path": path,
            "n": s["n"],
            "sites": len(s["sites"]),
            "endpoints": len(s["endpoints"]),
            "top_endpoint": top_ep,
            "ep_conc": top_n / s["n"],
        })

    adapter = [r for r in rows if r["ep_conc"] >= 0.9 and r["sites"] >= 5]
    publisher = [r for r in rows if r["endpoints"] >= 5 and r["sites"] <= 3]
    mixed = [r for r in rows if r not in adapter and r not in publisher]

    def show(title, rs, note):
        print(f"\n{title}  ({len(rs)} signatures)")
        print(f"  {note}")
        print(f"  {'defect':58s} {'n':>5s} {'sites':>6s} {'eps':>4s} {'conc':>5s}  top endpoint")
        for r in sorted(rs, key=lambda x: -x["n"])[:12]:
            label = f"{r['rule']} @ {r['path']}"
            print(f"  {label[:58]:58s} {r['n']:5d} {r['sites']:6d} {r['endpoints']:4d} "
                  f"{r['ep_conc']:5.2f}  {r['top_endpoint'][:34]}")

    show("ADAPTER-ORIGIN", adapter,
         "one endpoint, many publishers: the SSP's shared adapter code emits it")
    show("PUBLISHER-ORIGIN", publisher,
         "many endpoints, few publishers: the page or its config emits it")
    show("MIXED / ECOSYSTEM-WIDE", mixed,
         "spread across both axes: convention or shared upstream builder")

    tot = sum(r["n"] for r in rows)
    print(f"\nsignatures >=20 occurrences: {len(rows)} covering {tot} request-side findings")
    print(f"  adapter-origin:   {sum(r['n'] for r in adapter):6d} ({100*sum(r['n'] for r in adapter)/tot:.1f}%)")
    print(f"  publisher-origin: {sum(r['n'] for r in publisher):6d} ({100*sum(r['n'] for r in publisher)/tot:.1f}%)")
    print(f"  mixed:            {sum(r['n'] for r in mixed):6d} ({100*sum(r['n'] for r in mixed)/tot:.1f}%)")


if __name__ == "__main__":
    main()
