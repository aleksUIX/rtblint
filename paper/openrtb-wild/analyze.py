#!/usr/bin/env python3
"""Analyze a capture run: crawl stats + RTBlint validation of every payload.

Usage: python3 analyze.py captures/run1.jsonl [more.jsonl ...]
"""

import json
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path
from urllib.parse import urlparse

RTBLINT = Path.home() / "Documents/workspace/vast-master/rtblint/target/release/rtblint"
import os
DEFAULT_VERSION = os.environ.get("RTBLINT_VERSION", "2.6-202606")
MAX_VALIDATIONS = 2000

KNOWN_VERSIONS = {"2.0", "2.1", "2.2", "2.3", "2.3.1", "2.4", "2.5", "2.6", "3.0"}


def pick_version(rec):
    v = rec.get("xOpenrtbVersion")
    if v in KNOWN_VERSIONS:
        return DEFAULT_VERSION if v == "2.6" else v
    return DEFAULT_VERSION


def validate(kind, body, version):
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
        json.dump(body, f)
        path = f.name
    r = subprocess.run(
        [str(RTBLINT), "validate", "--type", kind, "--version", version, "--format", "json", path],
        capture_output=True,
        text=True,
    )
    Path(path).unlink(missing_ok=True)
    try:
        return json.loads(r.stdout)
    except json.JSONDecodeError:
        return {"valid": None, "issues": [], "cli_error": r.stderr[:200]}


def main(paths):
    metas, requests, responses = [], [], []
    for p in paths:
        for line in open(p):
            rec = json.loads(line)
            {"site-meta": metas, "ortb-request": requests, "ortb-response": responses}.get(
                rec["kind"], []
            ).append(rec)

    print(f"sites: {len(metas)}  ortb-requests: {len(requests)}  ortb-responses: {len(responses)}")
    ok = [m for m in metas if str(m.get('status', '')).startswith('ok')]
    pb = [m for m in metas if m.get("hasPbjs")]
    cap = [m for m in metas if (m.get("ortbRequests") or 0) > 0]
    print(f"loaded ok: {len(ok)}  pbjs detected: {len(pb)}  sites with ortb traffic: {len(cap)}")

    endpoints = Counter(urlparse(r["endpoint"]).netloc for r in requests)
    if endpoints:
        print("\ntop ORTB endpoints:")
        for host, n in endpoints.most_common(15):
            print(f"  {n:5d}  {host}")

    versions = Counter(r.get("xOpenrtbVersion") or "(none)" for r in requests)
    print(f"\nx-openrtb-version header: {dict(versions)}")

    if not RTBLINT.exists():
        print("\nrtblint release binary missing; skipping validation")
        return

    rule_counter, sev_counter = Counter(), Counter()
    invalid = 0
    todo = [("request", r) for r in requests] + [("response", r) for r in responses]
    todo = todo[:MAX_VALIDATIONS]
    for kind, rec in todo:
        rep = validate(kind, rec["body"], pick_version(rec))
        if rep.get("valid") is False:
            invalid += 1
        for it in rep.get("issues", []):
            rule_counter[it.get("id") or "?"] += 1
            sev_counter[it.get("severity", "?")] += 1

    n = len(todo)
    print(f"\nvalidated {n} payloads against RTBlint: {invalid} invalid ({100*invalid/max(n,1):.1f}%)")
    print(f"issues by severity: {dict(sev_counter)}")
    print("\ntop rules triggered:")
    for rule, cnt in rule_counter.most_common(20):
        print(f"  {cnt:5d}  {rule}")


if __name__ == "__main__":
    main(sys.argv[1:] or ["captures/smoke.jsonl"])
