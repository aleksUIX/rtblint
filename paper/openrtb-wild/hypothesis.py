#!/usr/bin/env python3
"""Test the predictions the machine-checkability paper makes about live traffic.

Paper 2 (doi:10.13140/RG.2.2.27937.57448) argues three things that this
corpus can test directly:

  H1 (optionality). The required core stayed nearly flat (16 -> 22 fields)
      while the surface grew to 417, and the optional surface is where
      bilateral interpretation lives. Prediction: violations should almost
      never touch the required core.

  H2 (triviality). If validation were merely difficult, violations would
      cluster in the hard-to-check classes. Prediction under the "nobody
      validates" reading instead: the class a plain JSON Schema catches
      (paper 2 class A, only 14.5 percent of the spec) should still carry
      the bulk of violations.

  H3 (tolerant reader). Section 5.5 argues the mandated tolerance for
      unknown fields removes backpressure. Prediction: undefined fields,
      precisely what receivers are told to swallow silently, should be the
      largest single violation class.

Usage: python3 hypothesis.py captures/full1.jsonl
"""

import json
import subprocess
import sys
from collections import Counter, defaultdict
from pathlib import Path
from urllib.parse import urlparse

RTBLINT = Path.home() / "Documents/workspace/vast-master/rtblint/target/release/rtblint"
BESTFIT = Path(__file__).parent / (sys.argv[2] if len(sys.argv) > 2 else "bestfit_results.json")

# Rule -> paper 2 enforceability class, using that paper's Section 3.3 boundary:
# A = types, unconditional presence, enumerated values, structural shape;
# B = conditional requiredness, exclusivities, registry membership,
#     version-delta reasoning, markup coherence;
# C = cross-message / cross-artifact / runtime.
RULE_CLASS = {
    "openrtb.field.undefined": "A",
    "openrtb.type.mismatch": "A",
    "openrtb.field.required": "A",
    "openrtb.value.invalid": "A",
    "openrtb.payload.invalid_json": "A",
    "openrtb.payload.root_not_object": "A",
    "openrtb.imp.bidfloor_negative": "A",
    "openrtb.request.tmax_non_positive": "A",
    "openrtb.field.deprecated": "B",
    "openrtb.field.moved": "B",
    "openrtb.field.removed": "B",
    "openrtb.field.not_yet_available": "B",
    "openrtb.fields.mutually_exclusive": "B",
    "openrtb.imp.media_type.required": "B",
    "openrtb.response.seatbid_or_nbr.required": "B",
    "openrtb.field.requires_skippable_video": "B",
    "openrtb.imp.bidfloorcur_format_invalid": "B",
    "openrtb.request.cur_format_invalid": "B",
    "openrtb.request.tmax_implausible": "B",
    "openrtb.regs.gpp_sid_without_gpp": "B",
    "openrtb.regs.gpp_without_gpp_sid": "B",
    "openrtb.regs.us_privacy_malformed": "B",
    "openrtb.schain.duplicate_node": "B",
    "openrtb.schain.node.hp_missing": "B",
    "openrtb.schain.node.identifier_empty": "B",
    "openrtb.video.pod.rqddurs_empty": "B",
    "openrtb.video.pod.mincpmpersec_without_pod_context": "B",
    "openrtb.native.request.unparseable": "B",
    "openrtb.native.request.double_encoded": "B",
    "openrtb.native.request.legacy_wrapper": "B",
    "openrtb.bid.adm.native_not_json": "B",
    "openrtb.bid.adm.not_markup": "B",
    "openrtb.bid.adm.double_encoded": "B",
    "openrtb.bid.adm.vast_root_missing": "B",
    "openrtb.bid.mtype_missing": "B",
}
# everything cross-message resolves to C
C_PREFIXES = (
    "openrtb.pair.", "openrtb.bid.impid_unknown", "openrtb.bid.dealid_unknown",
    "openrtb.bid.mtype_not_offered", "openrtb.bid.adm.media_type_mismatch",
    "openrtb.seatbid.seat_not_allowed", "openrtb.response.cur_not_allowed",
    "openrtb.response.request_id_mismatch",
)


def rule_class(rule):
    if rule in RULE_CLASS:
        return RULE_CLASS[rule]
    if any(rule.startswith(p) for p in C_PREFIXES):
        return "C"
    return "?"


def main(capture_path):
    bestfit = json.load(open(BESTFIT))["best_fit_per_side"]

    groups = defaultdict(list)
    for line in open(capture_path):
        rec = json.loads(line)
        if rec["kind"] not in ("ortb-request", "ortb-response"):
            continue
        kind = "request" if rec["kind"] == "ortb-request" else "response"
        groups[(urlparse(rec["endpoint"]).netloc, kind)].append(rec["body"])

    rule_counter = Counter()
    class_counter = Counter()
    payloads_with_class = Counter()
    payloads_total = 0
    payloads_with_required_violation = 0
    payloads_with_any = 0

    for (host, kind), bodies in groups.items():
        version = bestfit.get(f"{host}|{kind}")
        if not version:
            continue
        stdin_data = "\n".join(json.dumps(b, separators=(",", ":")) for b in bodies)
        r = subprocess.run(
            [str(RTBLINT), "validate", "--batch", "--type", kind,
             "--version", version, "--format", "json"],
            input=stdin_data, capture_output=True, text=True,
        )
        for line in r.stdout.splitlines():
            try:
                rep = json.loads(line)
            except json.JSONDecodeError:
                continue
            payloads_total += 1
            seen = set()
            has_required = False
            for it in rep.get("issues", []):
                rule = it.get("id", "?")
                cls = rule_class(rule)
                rule_counter[rule] += 1
                class_counter[cls] += 1
                seen.add(cls)
                if rule == "openrtb.field.required":
                    has_required = True
            if seen:
                payloads_with_any += 1
            for c in seen:
                payloads_with_class[c] += 1
            if has_required:
                payloads_with_required_violation += 1

    total_issues = sum(class_counter.values())
    print(f"payloads validated at best-fit version: {payloads_total}")
    print(f"payloads with >=1 issue: {payloads_with_any} ({100*payloads_with_any/payloads_total:.1f}%)")
    print(f"total issues: {total_issues}\n")

    print("H2: issues by paper-2 enforceability class")
    share = {
        "A": "14.5% of spec (plain JSON Schema)",
        "B": "39.0% of spec (lint rules)",
        "C": "20.5% of spec (cross-message)",
    }
    for cls in ("A", "B", "C", "?"):
        n = class_counter.get(cls, 0)
        if not n:
            continue
        pay = payloads_with_class.get(cls, 0)
        print(f"  class {cls}: {n:6d} issues ({100*n/total_issues:5.1f}% of all issues), "
              f"in {pay:5d} payloads ({100*pay/payloads_total:5.1f}%)   {share.get(cls,'')}")

    print(f"\nH1: required core vs optional surface")
    print(f"  required-field violations (openrtb.field.required): "
          f"{rule_counter.get('openrtb.field.required', 0)} issues in "
          f"{payloads_with_required_violation} payloads "
          f"({100*payloads_with_required_violation/payloads_total:.1f}% of payloads)")
    print(f"  all other violations: {total_issues - rule_counter.get('openrtb.field.required', 0)} issues")
    print(f"  the required core is 22 of 417 fields (5.3% of the 2.6 surface)")

    print(f"\nH3: tolerant-reader class")
    und = rule_counter.get("openrtb.field.undefined", 0)
    print(f"  openrtb.field.undefined: {und} issues ({100*und/total_issues:.1f}% of all issues), "
          f"largest single class: {und == max(rule_counter.values())}")

    print("\nall rules:")
    for rule, cnt in rule_counter.most_common():
        print(f"  {cnt:6d}  [{rule_class(rule)}]  {rule}")

    json.dump(
        {
            "payloads": payloads_total,
            "payloads_with_issue": payloads_with_any,
            "issues_by_class": dict(class_counter),
            "payloads_by_class": dict(payloads_with_class),
            "rules": dict(rule_counter),
            "rule_class_map": {r: rule_class(r) for r in rule_counter},
            "required_violation_payloads": payloads_with_required_violation,
        },
        open(Path(__file__).parent / (sys.argv[3] if len(sys.argv) > 3 else "hypothesis_results.json"), "w"), indent=1,
    )
    print("\nwritten: hypothesis_results.json")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "captures/full1.jsonl")
