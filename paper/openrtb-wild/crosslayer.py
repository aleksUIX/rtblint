#!/usr/bin/env python3
"""Cross-layer validation: the VAST creative against the OpenRTB contract.

OpenRTB carries video markup inside bid.adm but imposes no requirement that
the markup be valid, nor that it honour the constraints the request stated
(duration bounds, accepted MIME types, supported protocols and API
frameworks). This joins the two layers:

  1. VAST well-formedness and spec conformance, via vastlint.
  2. Contract coherence, per bid, against the originating imp.video object:
       Duration        vs  imp.video.minduration / maxduration
       MediaFile type  vs  imp.video.mimes
       VAST version    vs  imp.video.protocols
       VPAID usage     vs  imp.video.api

Nothing in either specification checks the second class, and no single-layer
validator can: it needs the request and the creative together.

Usage: python3 crosslayer.py captures/full1.jsonl
"""

import json
import re
import subprocess
import sys
import xml.etree.ElementTree as ET
from collections import Counter, defaultdict
from pathlib import Path
from urllib.parse import urlparse

VASTLINT = Path.home() / "Documents/workspace/vast-master/vastlint/target/release/vastlint"
HERE = Path(__file__).parent

# OpenRTB List: Protocols (AdCOM) -> VAST major version expressed by the tag
PROTOCOL_VAST = {
    1: "1.0", 2: "2.0", 3: "3.0", 4: "1.0", 5: "2.0", 6: "3.0",
    7: "4.0", 8: "4.0", 11: "4.1", 12: "4.1", 13: "4.2", 14: "4.2",
}
# OpenRTB List: API Frameworks
API_VPAID = {1, 2}  # VPAID 1.0, VPAID 2.0


def duration_seconds(text):
    m = re.match(r"^\s*(\d+):(\d{2}):(\d{2})(?:\.\d+)?\s*$", text or "")
    if not m:
        return None
    h, mnt, s = (int(g) for g in m.groups())
    return h * 3600 + mnt * 60 + s


def analyse_vast(xml_text):
    """Extract the creative facts the OpenRTB contract constrains."""
    facts = {"version": None, "durations": [], "mimes": [], "vpaid": False,
             "parse_ok": True, "wrapper": False, "inline": False, "flash": False}
    try:
        root = ET.fromstring(xml_text)
    except ET.ParseError:
        facts["parse_ok"] = False
        return facts
    facts["version"] = root.get("version")
    for el in root.iter():
        tag = el.tag.split("}")[-1]
        if tag == "Wrapper":
            facts["wrapper"] = True
        elif tag == "InLine":
            facts["inline"] = True
        if tag == "MediaFile" and "shockwave-flash" in (el.get("type") or ""):
            facts["flash"] = True
        if tag == "Duration":
            d = duration_seconds(el.text)
            if d is not None:
                facts["durations"].append(d)
        elif tag == "MediaFile":
            t = el.get("type")
            if t:
                facts["mimes"].append(t.lower())
            if (el.get("apiFramework") or "").lower() == "vpaid":
                facts["vpaid"] = True
        elif tag in ("Linear", "NonLinear") and (el.get("apiFramework") or "").lower() == "vpaid":
            facts["vpaid"] = True
    return facts


def main(capture_path):
    requests, responses = {}, []
    for line in open(capture_path):
        rec = json.loads(line)
        if rec["kind"] == "ortb-request":
            requests[(rec["site"], rec["requestId"])] = rec
        elif rec["kind"] == "ortb-response":
            responses.append(rec)

    cases = []
    for resp in responses:
        req = requests.get((resp["site"], resp["requestId"]))
        imps = {i.get("id"): i for i in (req["body"].get("imp") or [])} if req else {}
        for sb in resp["body"].get("seatbid") or []:
            for bid in sb.get("bid") or []:
                adm = bid.get("adm")
                if not isinstance(adm, str) or "<vast" not in adm.lstrip()[:400].lower():
                    continue
                cases.append({
                    "endpoint": urlparse(resp["endpoint"]).netloc,
                    "site": resp["site"],
                    "adm": adm,
                    "imp": imps.get(bid.get("impid")),
                })

    print(f"VAST creatives found in bid.adm: {len(cases)}")
    if not cases:
        return

    # --- layer 1: VAST conformance -------------------------------------
    vast_rules = Counter()
    invalid = 0
    for c in cases:
        p = subprocess.run(
            [str(VASTLINT), "check", "--format", "json", "--no-fail",
             "--ignore-pattern", r"\$\{[^}]*\}|%%[^%]*%%|\[[A-Z_]+\]", "-"],
            input=c["adm"], capture_output=True, text=True,
        )
        try:
            rep = json.loads(p.stdout)
        except json.JSONDecodeError:
            continue
        items = rep if isinstance(rep, list) else [rep]
        errs = 0
        for it in items:
            for f in it.get("findings", it.get("issues", [])) or []:
                rid = f.get("rule") or f.get("id") or f.get("rule_id") or "?"
                sev = (f.get("severity") or "").lower()
                vast_rules[f"{sev}:{rid}"] += 1
                if sev == "error":
                    errs += 1
        c["vast_errors"] = errs
        if errs:
            invalid += 1

    print(f"\nLAYER 1 - VAST conformance (vastlint)")
    print(f"  creatives with >=1 VAST error: {invalid}/{len(cases)} ({100*invalid/len(cases):.1f}%)")
    for rule, n in vast_rules.most_common(12):
        print(f"    {n:4d}  {rule}")

    # --- layer 2: contract coherence -----------------------------------
    viol = Counter()
    checked = 0
    per_endpoint = defaultdict(Counter)
    for c in cases:
        imp = c["imp"]
        if not imp or not isinstance(imp.get("video"), dict):
            continue
        v = imp["video"]
        f = analyse_vast(c["adm"])
        if not f["parse_ok"]:
            viol["vast_unparseable"] += 1
            continue
        checked += 1
        ep = c["endpoint"]
        if f["wrapper"] and not f["durations"] and not f["mimes"]:
            viol["UNVERIFIABLE_wrapper_defers_creative_facts"] += 1
        if f["flash"]:
            viol["flash_mediafile_served"] += 1

        maxd, mind = v.get("maxduration"), v.get("minduration")
        for d in f["durations"]:
            if isinstance(maxd, int) and d > maxd:
                viol["duration_exceeds_maxduration"] += 1
                per_endpoint[ep]["duration_exceeds_maxduration"] += 1
                break
            if isinstance(mind, int) and d < mind:
                viol["duration_below_minduration"] += 1
                per_endpoint[ep]["duration_below_minduration"] += 1
                break

        mimes = [m.lower() for m in (v.get("mimes") or []) if isinstance(m, str)]
        if mimes and f["mimes"] and not (set(f["mimes"]) & set(mimes)):
            viol["mediafile_mime_not_offered"] += 1
            per_endpoint[ep]["mediafile_mime_not_offered"] += 1

        protos = [p for p in (v.get("protocols") or []) if isinstance(p, int)]
        if protos and f["version"]:
            allowed = {PROTOCOL_VAST.get(p) for p in protos} - {None}
            major = f["version"].split(".")[0]
            if allowed and not any(a.startswith(major) for a in allowed):
                viol["vast_version_not_in_protocols"] += 1
                per_endpoint[ep]["vast_version_not_in_protocols"] += 1

        apis = {a for a in (v.get("api") or []) if isinstance(a, int)}
        if f["vpaid"] and apis and not (apis & API_VPAID):
            viol["vpaid_without_api_support"] += 1
            per_endpoint[ep]["vpaid_without_api_support"] += 1

    print(f"\nLAYER 2 - OpenRTB contract coherence")
    print(f"  creatives checkable against their imp.video: {checked}")
    if viol:
        for k, n in viol.most_common():
            print(f"    {n:4d}  {k}")
    else:
        print("    no contract violations detected")
    if per_endpoint:
        print("\n  by endpoint:")
        for ep, c in sorted(per_endpoint.items(), key=lambda kv: -sum(kv[1].values())):
            print(f"    {ep:40s} {dict(c)}")

    json.dump(
        {"creatives": len(cases), "vast_invalid": invalid, "vast_rules": dict(vast_rules),
         "contract_checked": checked, "contract_violations": dict(viol),
         "by_endpoint": {k: dict(v) for k, v in per_endpoint.items()}},
        open(HERE / "crosslayer_results.json", "w"), indent=1,
    )
    print("\nwritten: crosslayer_results.json")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "captures/full1.jsonl")
