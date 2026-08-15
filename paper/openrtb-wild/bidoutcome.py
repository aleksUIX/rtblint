#!/usr/bin/env python3
"""Does a non-conforming bid request cost the publisher a bid?

The obvious objection to this study is that malformed requests must be
suppressing demand, and that the conformance rate is therefore already
priced in as lost revenue. This tests that directly by joining each
request to the response it solicited and comparing bid rates.

Two methodological constraints follow from the rest of the paper.

  First contact only. Bid outcome is fill, and Section 4.5 shows fill
  decays as the measuring profile ages. Only wave-0 captures are used.

  Stratify by endpoint. Endpoints differ enormously in both baseline bid
  rate and in the conformance of the adapters that address them, so the
  naive pooled comparison is a composition artifact: endpoints that bid on
  everything happen to receive many invalid requests, which drags the
  pooled invalid-request bid rate up. We report the naive figure to show
  the trap, then the endpoint-stratified Mantel-Haenszel risk difference,
  with a site-clustered bootstrap interval.

What this cannot do is establish causation. Conformance is a property of
the adapter, not of an individual request, so "valid" and "invalid"
requests at one endpoint largely come from different adapters rather than
from one adapter varying. Any difference is therefore confounded with
everything else that differs between adapters. Stated in the paper.

Usage: python3 bidoutcome.py
"""

import json
import random
import subprocess
from collections import defaultdict
from pathlib import Path
from urllib.parse import urlparse

RTBLINT = Path.home() / "Documents/workspace/vast-master/rtblint/target/release/rtblint"
HERE = Path(__file__).parent
SEED = 20260725
BOOTSTRAP = 5000
MIN_PAIRS = 15
MIN_STRATUM = 5

SAMPLES = [
    ("Sample A (random)", "captures/tranco-deep.jsonl", "bestfit_results_sampleA.json"),
    ("Sample B (purposive)", "captures/full1.jsonl", "bestfit_results_sampleB.json"),
]


def load_pairs(capture):
    requests, responses = {}, []
    for line in open(HERE / capture):
        rec = json.loads(line)
        if rec["kind"] == "ortb-request":
            requests[(rec["site"], rec["requestId"])] = rec
        elif rec["kind"] == "ortb-response":
            responses.append(rec)
    pairs = []
    for resp in responses:
        req = requests.get((resp["site"], resp["requestId"]))
        if req:
            pairs.append((req, resp))
    return pairs


def classify(pairs, bestfit):
    """-> list of (site, endpoint, request_valid, got_bid)."""
    by_host = defaultdict(list)
    for req, resp in pairs:
        by_host[urlparse(resp["endpoint"]).netloc].append((req, resp))

    out = []
    for host, items in by_host.items():
        version = bestfit.get(f"{host}|request")
        if not version:
            continue
        stdin_data = "\n".join(
            json.dumps(r["body"], separators=(",", ":")) for r, _ in items)
        proc = subprocess.run(
            [str(RTBLINT), "validate", "--batch", "--type", "request",
             "--version", version, "--format", "json"],
            input=stdin_data, capture_output=True, text=True,
        )
        lines = proc.stdout.splitlines()
        if len(lines) != len(items):
            print(f"  WARNING: {host} gave {len(lines)} results for {len(items)} payloads; skipped")
            continue
        for (req, resp), line in zip(items, lines):
            try:
                rep = json.loads(line)
            except json.JSONDecodeError:
                continue
            bid = sum(len(s.get("bid") or [])
                      for s in (resp["body"].get("seatbid") or [])) > 0
            out.append((req["site"], host, bool(rep.get("valid")), bid))
    return out


def strata(rows):
    """-> {endpoint: (n_valid, bid_valid, n_invalid, bid_invalid)}"""
    agg = defaultdict(lambda: [0, 0, 0, 0])
    for _site, host, valid, bid in rows:
        a = agg[host]
        if valid:
            a[0] += 1
            a[1] += bid
        else:
            a[2] += 1
            a[3] += bid
    return agg


def mh_risk_difference(agg):
    """Mantel-Haenszel pooled risk difference, weight = n1*n0/(n1+n0)."""
    num = den = 0.0
    for nv, bv, ni, bi in agg.values():
        if nv < MIN_STRATUM or ni < MIN_STRATUM:
            continue
        w = nv * ni / (nv + ni)
        num += w * (bv / nv - bi / ni)
        den += w
    return (num / den * 100) if den else float("nan")


def main():
    rng = random.Random(SEED)
    for label, capture, bestfit_file in SAMPLES:
        path = HERE / capture
        if not path.exists():
            print(f"{label}: capture missing, skipped\n")
            continue
        bestfit = json.load(open(HERE / bestfit_file))["best_fit_per_side"]
        pairs = load_pairs(capture)
        rows = classify(pairs, bestfit)
        agg = strata(rows)

        print(f"\n=== {label} ===")
        print(f"matched request/response pairs: {len(rows)}")

        nv = sum(a[0] for a in agg.values())
        bv = sum(a[1] for a in agg.values())
        ni = sum(a[2] for a in agg.values())
        bi = sum(a[3] for a in agg.values())
        if not (nv and ni):
            print("  one stratum empty; nothing to compare\n")
            continue
        print(f"naive pooled: conforming requests bid {100*bv/nv:.1f}% (n={nv}), "
              f"non-conforming {100*bi/ni:.1f}% (n={ni}), "
              f"difference {100*bv/nv - 100*bi/ni:+.1f} points")

        print(f"\nper endpoint (>={MIN_PAIRS} pairs, >={MIN_STRATUM} in each stratum):")
        print(f"  {'endpoint':38s} {'pairs':>6s} {'conf bid%':>10s} {'non-conf bid%':>14s} {'diff':>7s}")
        shown = 0
        for host, (nv_, bv_, ni_, bi_) in sorted(agg.items(), key=lambda kv: -sum(kv[1][::2])):
            if nv_ + ni_ < MIN_PAIRS or nv_ < MIN_STRATUM or ni_ < MIN_STRATUM:
                continue
            shown += 1
            rv, ri = 100 * bv_ / nv_, 100 * bi_ / ni_
            print(f"  {host:38s} {nv_+ni_:6d} {rv:9.1f}% {ri:13.1f}% {rv-ri:+6.1f}")
        if not shown:
            print("  (no endpoint has both strata populated)")

        rd = mh_risk_difference(agg)
        print(f"\nendpoint-stratified (Mantel-Haenszel) risk difference: {rd:+.1f} points")

        sites = sorted({r[0] for r in rows})
        by_site = defaultdict(list)
        for r in rows:
            by_site[r[0]].append(r)
        draws = []
        for _ in range(BOOTSTRAP):
            resampled = []
            for _ in sites:
                resampled.extend(by_site[rng.choice(sites)])
            d = mh_risk_difference(strata(resampled))
            if d == d:
                draws.append(d)
        draws.sort()
        if draws:
            lo = draws[int(0.025 * len(draws))]
            hi = draws[int(0.975 * len(draws))]
            print(f"site-clustered bootstrap 95% CI: [{lo:+.1f}, {hi:+.1f}] "
                  f"({BOOTSTRAP} draws over {len(sites)} sites)")
            print("interpretation:", "excludes zero" if lo > 0 or hi < 0 else "includes zero")
    print()


if __name__ == "__main__":
    main()
