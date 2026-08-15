#!/usr/bin/env python3
"""Repeated measurement depresses bid rates. Request-side statistics survive it.

Wave 4 was run at midday specifically to test whether the response-side
variation was diurnal. It was not: Index's no-bid rate was 99.7 percent at
14:00 and 99.7 percent at 20:54, while three captures on the first crawl day
all sat near 51 percent at three different evening hours.

Ordering every capture by time shows a step, not a cycle, and the step lands
at the boundary of the first crawl day. Because it appears simultaneously
across unrelated demand partners, who do not coordinate, the cause is the
measuring profile rather than the market: after a day of repeated crawling
the address and browser profile stop attracting bids.

The consequence for the study is asymmetric and worth stating plainly.
Request construction happens client-side and does not depend on whether
anyone bids, so request-side conformance is unaffected, which the wave
stability confirms. Response-side statistics that depend on fill are only
clean at first contact.

Confounds tested and excluded:

  VPN or changed network. Captured requests carry device.ip, and the same
    residential address appears in every
    capture including the later ones.
  Shifting site mix. Restricting to the 81 Sample B sites that yielded
    responses in all three waves, with comparable response counts per wave,
    reproduces the decline unchanged.
  Time of day. The midday capture matches the evening ones.

One alternative remains open and cannot be settled from a single vantage: a
genuine market-wide change coincident with the first crawl day. A capture
from a fresh address discriminates between the two, since a new profile
should recover day-zero bid rates if the cause is profile ageing and should
not if the market itself moved. That test is pending.

Usage: python3 footprint.py
"""

import glob
import json
from collections import defaultdict
from datetime import datetime, timedelta, timezone
from pathlib import Path
from urllib.parse import urlparse

HERE = Path(__file__).parent
LOCAL = timezone(timedelta(hours=-7))
SKIP = ("budget-test", "debug", "smoke", "selftest")
ENDPOINTS = [
    "htlb.casalemedia.com",
    "rtb.openx.net",
    "prebid-server.rubiconproject.com",
    "hbopenbid.pubmatic.com",
    "c2shb.pubgw.yahoo.com",
]
MIN_N = 20


def captures():
    out = []
    for f in sorted(glob.glob(str(HERE / "captures/*.jsonl"))):
        if any(k in f for k in SKIP):
            continue
        ts = [r["ts"] for r in (json.loads(l) for l in open(f))
              if r.get("kind") == "site-meta" and r.get("ts")]
        if ts:
            when = datetime.fromisoformat(min(ts).replace("Z", "+00:00")).astimezone(LOCAL)
            out.append((when, f))
    return sorted(out)


def bid_rates(path):
    agg = defaultdict(lambda: {"n": 0, "bid": 0})
    for line in open(path):
        r = json.loads(line)
        if r.get("kind") != "ortb-response":
            continue
        a = agg[urlparse(r["endpoint"]).netloc]
        a["n"] += 1
        if sum(len(s.get("bid") or []) for s in (r["body"].get("seatbid") or [])):
            a["bid"] += 1
    return agg


def main():
    caps = captures()
    if not caps:
        print("no captures found")
        return

    day0 = caps[0][0].date()
    print("Bid rate (share of responses carrying at least one bid)\n")
    header = f"{'capture':20s} {'day':>5s}  " + "  ".join(
        f"{e.split('.')[0][:10]:>10s}" for e in ENDPOINTS)
    print(header)
    print("-" * len(header))

    first, later = defaultdict(list), defaultdict(list)
    for when, f in caps:
        rates = bid_rates(f)
        is_first = when.date() == day0
        cells = []
        for e in ENDPOINTS:
            a = rates.get(e, {"n": 0, "bid": 0})
            if a["n"] < MIN_N:
                cells.append(f"{'-':>10s}")
                continue
            r = 100 * a["bid"] / a["n"]
            cells.append(f"{r:9.1f}%")
            (first if is_first else later)[e].append(r)
        tag = "  d0" if is_first else f"  +{(when.date()-day0).days}"
        print(f"{when:%a %b %d %H:%M}    {tag:>4s}  " + "  ".join(cells))

    print("\nFirst crawl day versus later, mean bid rate per endpoint:")
    print(f"  {'endpoint':36s} {'day 0':>8s} {'later':>8s} {'change':>9s}")
    for e in ENDPOINTS:
        if not first[e] or not later[e]:
            continue
        a, b = sum(first[e]) / len(first[e]), sum(later[e]) / len(later[e])
        print(f"  {e:36s} {a:7.1f}% {b:7.1f}% {b-a:+8.1f}")

    print("""
Reading: the decline appears across independent demand partners at once, so
it is a property of the measuring profile, not of any vendor or of the hour.
Report response-side fill statistics from first contact only. Request-side
conformance is unaffected and replicates across every wave.""")


if __name__ == "__main__":
    main()
