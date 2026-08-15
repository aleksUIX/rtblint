#!/usr/bin/env python3
"""Best-fit version attribution for captured OpenRTB traffic.

Validates every captured payload against every cataloged OpenRTB version
using RTBlint batch mode, assigns each endpoint the version that minimizes
its total error count (ties go to the newest version), and reports
violations under that best-fit baseline.

Usage: python3 bestfit.py captures/full1.jsonl
"""

import json
import subprocess
import sys
from collections import Counter, defaultdict
from pathlib import Path
from urllib.parse import urlparse

RTBLINT = Path.home() / "Documents/workspace/vast-master/rtblint/target/release/rtblint"

VERSIONS = [
    "2.0", "2.1", "2.2", "2.3", "2.3.1", "2.4", "2.5",
    "2.6-202210", "2.6-202211", "2.6-202303", "2.6-202309",
    "2.6-202402", "2.6-202409", "2.6-202501", "2.6-202505", "2.6-202606",
]
NEWEST_RANK = {v: i for i, v in enumerate(VERSIONS)}


def load(path):
    payloads = {"request": [], "response": []}
    for line in open(path):
        rec = json.loads(line)
        if rec["kind"] == "ortb-request":
            payloads["request"].append(rec)
        elif rec["kind"] == "ortb-response":
            payloads["response"].append(rec)
    return payloads


def batch_validate(kind, records, version):
    """Returns list of (error_count, issue_ids, valid) aligned with records."""
    stdin_data = "\n".join(json.dumps(r["body"], separators=(",", ":")) for r in records)
    r = subprocess.run(
        [str(RTBLINT), "validate", "--batch", "--type", kind, "--version", version, "--format", "json"],
        input=stdin_data,
        capture_output=True,
        text=True,
    )
    out = []
    for line in r.stdout.splitlines():
        try:
            rep = json.loads(line)
        except json.JSONDecodeError:
            out.append((0, [], None))
            continue
        issues = rep.get("issues", [])
        errs = [i for i in issues if i.get("severity") == "error"]
        out.append((len(errs), [i.get("id") for i in issues], rep.get("valid")))
    while len(out) < len(records):
        out.append((0, [], None))
    return out


def main(path):
    payloads = load(path)
    n_req, n_resp = len(payloads["request"]), len(payloads["response"])
    print(f"payloads: {n_req} requests, {n_resp} responses; versions: {len(VERSIONS)}")

    # errors[kind][version] = list of error counts aligned with payloads[kind]
    results = {k: {} for k in payloads}
    for kind, records in payloads.items():
        if not records:
            continue
        for v in VERSIONS:
            results[kind][v] = batch_validate(kind, records, v)
        print(f"  validated all {len(records)} {kind}s across {len(VERSIONS)} versions")

    # per-side (endpoint, kind) best-fit: adapter-built requests and
    # server-built responses can speak different dialects
    side_errors = defaultdict(Counter)
    for kind, records in payloads.items():
        for idx, rec in enumerate(records):
            host = urlparse(rec["endpoint"]).netloc
            for v in VERSIONS:
                side_errors[(host, kind)][v] += results[kind][v][idx][0]

    best_fit = {}
    for key, per_v in side_errors.items():
        best_fit[key] = min(per_v.items(), key=lambda kv: (kv[1], -NEWEST_RANK[kv[0]]))[0]

    side_stats = defaultdict(lambda: {"n": 0, "invalid": 0, "rules": Counter()})
    overall = {k: {"n": 0, "invalid": 0, "rules": Counter()} for k in ("request", "response")}
    for kind, records in payloads.items():
        for idx, rec in enumerate(records):
            host = urlparse(rec["endpoint"]).netloc
            v = best_fit[(host, kind)]
            errs, ids, valid = results[kind][v][idx]
            s = side_stats[(host, kind)]
            s["n"] += 1
            overall[kind]["n"] += 1
            if valid is False:
                s["invalid"] += 1
                overall[kind]["invalid"] += 1
            for i in ids:
                s["rules"][i] += 1
                overall[kind]["rules"][i] += 1

    print("\nATTRIBUTION SPLIT (per-side best-fit):")
    for kind in ("request", "response"):
        o = overall[kind]
        who = "adapter/publisher-built" if kind == "request" else "server-built"
        print(f"  {kind}s ({who}): {o['invalid']}/{o['n']} invalid ({100*o['invalid']/max(o['n'],1):.1f}%)")
        for rule, cnt in o["rules"].most_common(6):
            print(f"      {cnt:5d}  {rule}")

    hosts = sorted({h for h, _ in list(side_stats)}, key=lambda h: -(side_stats[(h, "request")]["n"] + side_stats[(h, "response")]["n"]))
    print(f"\nper-endpoint, both sides (>=20 total payloads):")
    print(f"{'endpoint':40s} {'reqN':>5s} {'req-fit':>11s} {'reqInv%':>8s} {'respN':>6s} {'resp-fit':>11s} {'respInv%':>9s}")
    for h in hosts:
        rq = side_stats.get((h, "request"), {"n": 0, "invalid": 0, "rules": Counter()})
        rs = side_stats.get((h, "response"), {"n": 0, "invalid": 0, "rules": Counter()})
        if rq["n"] + rs["n"] < 20:
            continue
        rqf = best_fit.get((h, "request"), "-")
        rsf = best_fit.get((h, "response"), "-")
        rqi = f"{100*rq['invalid']/rq['n']:.1f}%" if rq["n"] else "-"
        rsi = f"{100*rs['invalid']/rs['n']:.1f}%" if rs["n"] else "-"
        print(f"{h:40s} {rq['n']:5d} {rqf:>11s} {rqi:>8s} {rs['n']:6d} {rsf:>11s} {rsi:>9s}")

    json.dump(
        {
            "best_fit_per_side": {f"{h}|{k}": v for (h, k), v in best_fit.items()},
            "overall_by_side": {
                k: {"n": o["n"], "invalid": o["invalid"], "rules": dict(o["rules"])}
                for k, o in overall.items()
            },
            "endpoints_by_side": {
                f"{h}|{k}": {"n": s["n"], "invalid": s["invalid"], "best_fit": best_fit.get((h, k)), "rules": dict(s["rules"])}
                for (h, k), s in side_stats.items()
                if s["n"] > 0
            },
        },
        open("bestfit_results.json", "w"),
        indent=1,
    )
    print("\nwritten: bestfit_results.json")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "captures/full1.jsonl")
