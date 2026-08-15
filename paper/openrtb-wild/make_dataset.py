#!/usr/bin/env python3
"""Build the publishable, PII-free dataset from the private capture corpus.

The raw corpus cannot be released: bid requests carry user identifiers,
extended IDs, IP-derived geolocation, and commercial terms (floor prices,
deal IDs). This script emits only structural findings, which reproduce every
table in the paper without exposing a single payload value.

Emitted (dataset/):
  issues.csv    one row per validation finding: site, endpoint, side,
                best-fit version, rule id, severity, JSON path, spec section.
                Paths are structural locators; no values are ever written.
  payloads.csv  one row per payload: site, endpoint, side, version, issue
                counts, validity. Enables clustering-aware statistics.
  endpoints.csv per (endpoint, side) aggregate.
  README.md     provenance and schema.

Usage: python3 make_dataset.py captures/full1.jsonl
"""

import csv
import json
import re
import subprocess
import sys
from collections import Counter, defaultdict
from pathlib import Path
from urllib.parse import urlparse

RTBLINT = Path.home() / "Documents/workspace/vast-master/rtblint/target/release/rtblint"
HERE = Path(__file__).parent
OUT = HERE / (sys.argv[2] if len(sys.argv) > 2 else "dataset")
BESTFIT = HERE / (sys.argv[3] if len(sys.argv) > 3 else "bestfit_results.json")

# Defence in depth: a path should never carry a value, but if a future rule
# ever embeds one, drop anything that looks like an identifier or coordinate.
SUSPICIOUS = re.compile(r"[0-9a-f]{16,}|@|\d+\.\d+\.\d+\.\d+")


def validator_version():
    r = subprocess.run([str(RTBLINT), "--version"], capture_output=True, text=True)
    return r.stdout.strip() or "unknown"


def main(capture_path):
    OUT.mkdir(exist_ok=True)
    bestfit = json.load(open(BESTFIT))["best_fit_per_side"]
    ver = validator_version()

    groups = defaultdict(list)
    for line in open(capture_path):
        rec = json.loads(line)
        if rec["kind"] not in ("ortb-request", "ortb-response"):
            continue
        side = "request" if rec["kind"] == "ortb-request" else "response"
        host = urlparse(rec["endpoint"]).netloc
        groups[(host, side)].append(rec)

    issue_rows, payload_rows = [], []
    payload_seq = 0
    dropped_paths = 0

    for (host, side), recs in sorted(groups.items()):
        version = bestfit.get(f"{host}|{side}")
        if not version:
            continue
        stdin_data = "\n".join(json.dumps(r["body"], separators=(",", ":")) for r in recs)
        proc = subprocess.run(
            [str(RTBLINT), "validate", "--batch", "--type", side,
             "--version", version, "--format", "json"],
            input=stdin_data, capture_output=True, text=True,
        )
        lines = proc.stdout.splitlines()
        if len(lines) != len(recs):
            print(f"WARNING: {host}|{side} produced {len(lines)} results for {len(recs)} payloads")
        for rec, line in zip(recs, lines):
            payload_seq += 1
            pid = f"p{payload_seq:06d}"
            try:
                rep = json.loads(line)
            except json.JSONDecodeError:
                continue
            issues = rep.get("issues", [])
            sev = Counter(i.get("severity", "?") for i in issues)
            payload_rows.append({
                "payload_id": pid,
                "site": rec["site"],
                "endpoint": host,
                "side": side,
                "best_fit_version": version,
                "valid": rep.get("valid"),
                "n_issues": len(issues),
                "n_errors": sev.get("error", 0),
                "n_warnings": sev.get("warning", 0),
            })
            for it in issues:
                path = it.get("path", "")
                if SUSPICIOUS.search(path):
                    dropped_paths += 1
                    path = "<redacted>"
                issue_rows.append({
                    "payload_id": pid,
                    "site": rec["site"],
                    "endpoint": host,
                    "side": side,
                    "best_fit_version": version,
                    "rule": it.get("id"),
                    "severity": it.get("severity"),
                    "path": path,
                    "section": it.get("section", ""),
                })

    with open(OUT / "issues.csv", "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(issue_rows[0].keys()))
        w.writeheader()
        w.writerows(issue_rows)
    with open(OUT / "payloads.csv", "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(payload_rows[0].keys()))
        w.writeheader()
        w.writerows(payload_rows)

    agg = defaultdict(lambda: {"n": 0, "invalid": 0, "issues": 0})
    for p in payload_rows:
        a = agg[(p["endpoint"], p["side"], p["best_fit_version"])]
        a["n"] += 1
        a["issues"] += p["n_issues"]
        if p["valid"] is False:
            a["invalid"] += 1
    with open(OUT / "endpoints.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["endpoint", "side", "best_fit_version", "payloads", "invalid", "invalid_pct", "issues"])
        for (ep, side, v), a in sorted(agg.items(), key=lambda kv: -kv[1]["n"]):
            w.writerow([ep, side, v, a["n"], a["invalid"], round(100 * a["invalid"] / a["n"], 1), a["issues"]])

    sites = len({p["site"] for p in payload_rows})
    (OUT / "README.md").write_text(f"""# OpenRTB in-the-wild conformance dataset

Structural findings from live OpenRTB traffic captured at a residential
browser vantage point. Derived from a private raw corpus that is **not**
released: bid payloads contain user identifiers, extended IDs, geolocation,
and commercial terms (floor prices, deal IDs). This dataset contains only
rule identifiers and JSON paths, never payload values, and reproduces every
table and figure in the paper.

- Validator: {ver} (pinned; catalogs ship with the crate)
- Payloads: {len(payload_rows):,} across {sites} sites and {len(agg)} endpoint/side pairs
- Findings: {len(issue_rows):,}
- Redacted paths (defence-in-depth filter): {dropped_paths}

## Files

`payloads.csv` one row per captured payload
: `payload_id`, `site`, `endpoint`, `side` (request = built by the Prebid
  adapter and publisher config; response = built by the SSP server),
  `best_fit_version`, `valid`, `n_issues`, `n_errors`, `n_warnings`.
  Payloads cluster by site and by auction: use `site` for clustering-aware
  statistics rather than treating rows as independent.

`issues.csv` one row per validation finding
: `payload_id` (joins to payloads.csv), `site`, `endpoint`, `side`,
  `best_fit_version`, `rule` (stable RTBlint rule id), `severity`, `path`
  (structural JSON locator), `section` (OpenRTB spec section).

`endpoints.csv` per endpoint and side aggregate
: `payloads`, `invalid`, `invalid_pct`, `issues`.

## Method notes

Best-fit version: each (endpoint, side) is validated against all 16
cataloged OpenRTB versions and assigned the one minimizing its error count,
ties to newest. This is deliberately charitable to implementers. The
selection is only as sound as the weakest catalog in the candidate set, so
the validator version is pinned above and must be reported with any reuse.

Capture and analysis code accompanies this dataset.
""")

    print(f"validator: {ver}")
    print(f"payloads: {len(payload_rows):,}  issues: {len(issue_rows):,}  "
          f"sites: {sites}  endpoint/side pairs: {len(agg)}")
    print(f"redacted paths: {dropped_paths}")
    print(f"written: {OUT}/issues.csv, payloads.csv, endpoints.csv, README.md")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "captures/full1.jsonl")
