#!/usr/bin/env python3
"""Decide whether the current network is fit to measure from.

A datacenter or VPN exit yields zero auctions while every page still loads,
so the failure is silent and a whole wave looks successful while measuring
nothing. This gate therefore FAILS CLOSED: if it cannot establish what the
network is, it refuses.

The earlier inline guard grepped one provider for known hosting names. That
was wrong twice over: the provider rate-limited after repeated calls and
returned nothing, which made the grep match nothing and let the run proceed,
and a name blocklist misses any provider not on it. This queries several
sources and prefers positive proxy/hosting flags over name matching.

Exit 0 = fit to measure. Exit 1 = refuse.
Prints a one-line verdict either way, and reports mobile networks so the
hotspot wave can confirm it really is on a carrier.

Usage: python3 vantage.py [--quiet]
"""

import json
import sys
import urllib.request

TIMEOUT = 8
NAME_MARKERS = (
    "datacamp", "digitalocean", "linode", "ovh", "hetzner", "amazon", "aws",
    "google", "microsoft", "azure", "vultr", "choopa", "m247", "nordvpn",
    "mullvad", "surfshark", "expressvpn", "privateinternet", "cyberghost",
    "hosted", "hosting", "colo", "datacenter", "data center", "server",
)


def fetch(url, parse):
    try:
        with urllib.request.urlopen(url, timeout=TIMEOUT) as r:
            return parse(json.load(r))
    except Exception:
        return None


def probes():
    yield "ip-api", lambda: fetch(
        "http://ip-api.com/json?fields=status,isp,org,as,hosting,proxy,mobile",
        lambda d: None if d.get("status") != "success" else {
            "name": " ".join(filter(None, [d.get("isp"), d.get("org"), d.get("as")])),
            "proxy": bool(d.get("proxy")),
            "hosting": bool(d.get("hosting")),
            "mobile": bool(d.get("mobile")),
        },
    )
    yield "ipwho.is", lambda: fetch(
        "https://ipwho.is/",
        lambda d: None if not d.get("success", True) else {
            "name": " ".join(filter(None, [
                (d.get("connection") or {}).get("isp"),
                (d.get("connection") or {}).get("org"),
            ])),
            "proxy": bool((d.get("security") or {}).get("proxy")),
            "hosting": bool((d.get("security") or {}).get("hosting")),
            "mobile": False,
        },
    )
    yield "ipinfo", lambda: fetch(
        "https://ipinfo.io/json",
        lambda d: None if d.get("status") == 429 or not d.get("org") else {
            "name": d.get("org", ""),
            "proxy": False,
            "hosting": False,
            "mobile": False,
        },
    )


def main():
    quiet = "--quiet" in sys.argv
    verdicts = []
    for source, probe in probes():
        info = probe()
        if info:
            verdicts.append((source, info))

    if not verdicts:
        print("VANTAGE: REFUSE (no provider answered; failing closed rather "
              "than risk a silently void wave)")
        return 1

    bad = []
    mobile = False
    for source, info in verdicts:
        name = (info["name"] or "").lower()
        hits = [m for m in NAME_MARKERS if m in name]
        if info["proxy"]:
            bad.append(f"{source}: proxy flag")
        if info["hosting"]:
            bad.append(f"{source}: hosting flag")
        if hits:
            bad.append(f"{source}: name matches {hits[0]}")
        mobile = mobile or info["mobile"]

    label = verdicts[0][1]["name"] or "unknown"
    if bad:
        print(f"VANTAGE: REFUSE ({label}) :: " + "; ".join(sorted(set(bad))))
        return 1

    kind = "mobile carrier" if mobile else "residential/fixed"
    if not quiet:
        print(f"VANTAGE: OK ({label}) :: {kind}, {len(verdicts)} source(s) agree")
    return 0


if __name__ == "__main__":
    sys.exit(main())
