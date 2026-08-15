#!/usr/bin/env python3
"""Response-side violations are a deterministic property of no-bids.

Wave 2 showed the aggregate response-side rate moving from 14.9 to 31.6
percent while every request-side number held. The movement is entirely two
endpoints, and it is not a change in their behaviour: both emit a
specification-violating response every time they decline to bid, so their
observed failure rate is exactly their no-bid rate, which varies with
demand.

This checks that identity directly across every capture: for each endpoint,
what fraction of responses carry no bids, and what fraction fail validation.

Usage: python3 nobid.py
"""

import csv
import json
from collections import defaultdict
from pathlib import Path
from urllib.parse import urlparse

HERE = Path(__file__).parent
WAVES = [
    ("wave0 B", "captures/full1.jsonl", "dataset_sampleB"),
    ("wave2 B", "captures/wave-wave2-sampleB.jsonl", "waves/wave2-sampleB-dataset"),
    ("wave4 B", "captures/wave-wave4-sampleB.jsonl", "waves/wave4-sampleB-dataset"),
    ("wave0 A", "captures/tranco-deep.jsonl", "dataset_sampleA"),
    ("wave1 A", "captures/wave-wave1-sampleA.jsonl", "waves/wave1-sampleA-dataset"),
    ("wave3 A", "captures/wave-wave3-sampleA.jsonl", "waves/wave3-sampleA-dataset"),
]
MIN_N = 25


def nobid_rates(capture):
    agg = defaultdict(lambda: {"n": 0, "nobid": 0})
    for line in open(HERE / capture):
        r = json.loads(line)
        if r.get("kind") != "ortb-response":
            continue
        host = urlparse(r["endpoint"]).netloc
        a = agg[host]
        a["n"] += 1
        bids = sum(len(s.get("bid") or []) for s in (r["body"].get("seatbid") or []))
        if bids == 0:
            a["nobid"] += 1
    return agg


def invalid_rates(dataset):
    agg = defaultdict(lambda: {"n": 0, "inv": 0})
    for r in csv.DictReader(open(HERE / dataset / "payloads.csv")):
        if r["side"] != "response":
            continue
        a = agg[r["endpoint"]]
        a["n"] += 1
        if r["valid"] == "False":
            a["inv"] += 1
    return agg


def main():
    print(f"{'wave':9s} {'endpoint':36s} {'resp':>5s} {'no-bid%':>8s} {'invalid%':>9s} {'delta':>7s}")
    offenders = []
    for label, cap, ds in WAVES:
        if not (HERE / cap).exists() or not (HERE / ds).exists():
            continue
        nb, iv = nobid_rates(cap), invalid_rates(ds)
        for host in sorted(set(nb) & set(iv)):
            if iv[host]["n"] < MIN_N:
                continue
            nbr = 100 * nb[host]["nobid"] / nb[host]["n"]
            ivr = 100 * iv[host]["inv"] / iv[host]["n"]
            if ivr < 1:
                continue
            print(f"{label:9s} {host:36s} {iv[host]['n']:5d} {nbr:7.1f}% {ivr:8.1f}% {ivr-nbr:+7.1f}")
            offenders.append((label, host, nbr, ivr))

    print("\nEndpoints whose response failures track their no-bid rate exactly")
    print("(|invalid% - no-bid%| <= 2 points):")
    for label, host, nbr, ivr in offenders:
        if abs(ivr - nbr) <= 2:
            print(f"  {label:9s} {host:36s} no-bid {nbr:5.1f}%  invalid {ivr:5.1f}%")
    print("\nReading: for these endpoints the violation is deterministic per no-bid.")
    print("The aggregate response-side rate is therefore a function of fill rate,")
    print("not a stable property, and must be reported as demand-dependent.")


if __name__ == "__main__":
    main()
