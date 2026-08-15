#!/usr/bin/env python3
"""Item 3: cross-validate matched request/response pairs with RTBlint pair mode.

Pairs are matched by (site, requestId) from the capture stream. Reports the
pair-specific rules (impid/dealid/seat/currency/mtype coherence and
request-id match) that single-message validation cannot see.
"""

import json
import subprocess
import sys
import tempfile
from collections import Counter, defaultdict
from pathlib import Path
from urllib.parse import urlparse

RTBLINT = Path.home() / "Documents/workspace/vast-master/rtblint/target/release/rtblint"
VERSION = "2.6-202606"

# Rules that genuinely require the originating request. bid.mtype_missing is
# deliberately NOT here: it fires during ordinary single-message response
# validation, so counting it as a cross-message finding overstates what pair
# mode adds. An earlier version of this script made that mistake.
PAIR_RULES_PREFIXES = (
    "openrtb.pair.",
    "openrtb.bid.impid_unknown",
    "openrtb.bid.mtype_not_offered",
    "openrtb.bid.dealid_unknown",
    "openrtb.seatbid.seat_not_allowed",
    "openrtb.response.cur_not_allowed",
    "openrtb.response.request_id_mismatch",
    "openrtb.bid.adm.",
)


def main(capture_path):
    requests, responses = {}, []
    for line in open(capture_path):
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
    print(f"responses: {len(responses)}, matched to a captured request: {len(pairs)}")

    rule_counter = Counter()
    ep_stats = defaultdict(lambda: {"pairs": 0, "flagged": 0, "rules": Counter()})
    flagged = 0
    for req, resp in pairs:
        host = urlparse(resp["endpoint"]).netloc
        with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as fq:
            json.dump(req["body"], fq)
            qpath = fq.name
        with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as fr:
            json.dump(resp["body"], fr)
            rpath = fr.name
        r = subprocess.run(
            [str(RTBLINT), "validate", "--type", "response", "--version", VERSION,
             "--request", qpath, "--format", "json", rpath],
            capture_output=True, text=True,
        )
        Path(qpath).unlink(missing_ok=True)
        Path(rpath).unlink(missing_ok=True)
        try:
            rep = json.loads(r.stdout)
        except json.JSONDecodeError:
            continue
        pair_issues = [
            it for it in rep.get("issues", [])
            if any(str(it.get("id", "")).startswith(p) for p in PAIR_RULES_PREFIXES)
        ]
        s = ep_stats[host]
        s["pairs"] += 1
        if pair_issues:
            flagged += 1
            s["flagged"] += 1
        for it in pair_issues:
            rule_counter[it["id"]] += 1
            s["rules"][it["id"]] += 1

    n = sum(s["pairs"] for s in ep_stats.values())
    print(f"\nPAIR-MODE: {flagged}/{n} pairs with cross-validation findings ({100*flagged/max(n,1):.1f}%)")
    print("pair rules triggered:")
    for rule, cnt in rule_counter.most_common():
        print(f"  {cnt:5d}  {rule}")

    print("\nper-endpoint (>=10 pairs):")
    for host, s in sorted(ep_stats.items(), key=lambda kv: -kv[1]["pairs"]):
        if s["pairs"] < 10:
            continue
        top = s["rules"].most_common(1)
        top_s = f"{top[0][0]} ({top[0][1]})" if top else "-"
        print(f"  {host:42s} {s['pairs']:5d} pairs  {100*s['flagged']/s['pairs']:5.1f}% flagged  {top_s}")

    json.dump(
        {"matched_pairs": n, "flagged": flagged, "rules": dict(rule_counter),
         "endpoints": {h: {"pairs": s["pairs"], "flagged": s["flagged"], "rules": dict(s["rules"])}
                       for h, s in ep_stats.items()}},
        open("pairmode_results.json", "w"), indent=1,
    )
    print("\nwritten: pairmode_results.json")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "captures/full1.jsonl")
