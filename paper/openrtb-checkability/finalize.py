#!/usr/bin/env python3
"""Merge hand-coded classes into the statement dataset and compute final stats.

Classes:
  A  enforceable by JSON Schema on a single document
  B  enforceable by stateless lint rule on a single message (beyond schema)
  C  requires cross-message state, another artifact, or runtime interaction
  D  no machine-decidable conformance criterion
  X  not a conformance statement (definitional, explanatory, example, changelog)
"""

import csv
import json
import re
from pathlib import Path

DATA = Path(__file__).parent / "data"

RUNS = [
    ("0-7", "X"), ("8", "D"), ("9-13", "X"), ("14-18", "C"), ("19", "X"),
    ("20", "C"), ("21", "D"), ("22", "X"), ("23", "D"), ("24-28", "C"),
    ("29", "X"), ("30", "C"), ("31-34", "D"), ("35", "X"), ("36", "D"),
    ("37", "A"), ("38", "B"), ("39", "A"), ("40", "B"), ("41", "A"),
    ("42", "B"), ("43", "D"), ("44", "A"), ("45-48", "B"), ("49", "D"),
    ("50", "B"), ("51", "D"), ("52", "B"), ("53", "D"), ("54-59", "B"),
    ("60", "C"), ("61-62", "D"), ("63", "A"), ("64", "C"), ("65-70", "B"),
    ("71-74", "D"), ("75", "B"), ("76", "X"), ("77-78", "A"), ("79", "C"),
    ("80-82", "B"), ("83-84", "X"), ("85-86", "B"), ("87", "A"), ("88", "C"),
    ("89-90", "B"), ("91-92", "X"), ("93-94", "B"), ("95", "X"), ("96-97", "B"),
    ("98-102", "X"), ("103", "B"), ("104", "X"), ("105", "A"), ("106", "C"),
    ("107-108", "B"), ("109", "X"), ("110-111", "B"), ("112-113", "X"),
    ("114", "B"), ("115", "X"), ("116", "D"), ("117", "A"), ("118", "C"),
    ("119-120", "B"), ("121", "X"), ("122", "D"), ("123", "A"), ("124", "D"),
    ("125", "A"), ("126", "X"), ("127-128", "B"), ("129", "X"), ("130", "D"),
    ("131-132", "B"), ("133-134", "C"), ("135-136", "B"), ("137", "X"),
    ("138", "D"), ("139-142", "B"), ("143-144", "C"), ("145", "X"),
    ("146", "C"), ("147", "D"), ("148-149", "X"), ("150-153", "B"),
    ("154-155", "X"), ("156", "A"), ("157", "B"), ("158", "D"), ("159", "X"),
    ("160", "D"), ("161", "X"), ("162-163", "B"), ("164", "D"), ("165", "A"),
    ("166", "X"), ("167-168", "D"), ("169-170", "B"), ("171", "X"),
    ("172", "D"), ("173", "X"), ("174", "D"), ("175-176", "B"), ("177", "X"),
    ("178-179", "A"), ("180", "X"), ("181-182", "D"), ("183", "X"),
    ("184", "B"), ("185", "X"), ("186", "B"), ("187", "X"), ("188", "C"),
    ("189", "D"), ("190", "A"), ("191-192", "C"), ("193", "A"), ("194", "C"),
    ("195", "D"), ("196", "X"), ("197", "C"), ("198", "X"), ("199", "B"),
    ("200", "D"), ("201", "B"), ("202", "D"), ("203", "X"), ("204-209", "D"),
    ("210-212", "X"), ("213-214", "B"), ("215", "X"), ("216", "D"),
    ("217-218", "B"), ("219", "X"), ("220-221", "A"), ("222-224", "X"),
    ("225", "B"), ("226", "X"), ("227", "B"), ("228", "X"), ("229-231", "A"),
    ("232", "D"), ("233", "C"), ("234-236", "A"), ("237-238", "C"),
    ("239", "A"), ("240-241", "B"), ("242-243", "D"), ("244", "A"),
    ("245-248", "B"), ("249-250", "C"), ("251", "X"), ("252", "D"),
    ("253", "C"), ("254", "D"), ("255-256", "X"), ("257", "C"), ("258", "X"),
    ("259", "C"), ("260", "X"), ("261", "D"), ("262", "X"), ("263", "A"),
    ("264", "D"), ("265-269", "X"), ("270", "C"), ("271-274", "X"),
    ("275", "D"), ("276-279", "C"), ("280", "X"), ("281-283", "C"),
    ("284", "X"), ("285", "D"), ("286-287", "A"), ("288-289", "C"),
    ("290-293", "X"), ("294-301", "C"), ("302", "D"), ("303", "X"),
    ("304", "D"), ("305-310", "C"), ("311", "D"), ("312-313", "C"),
    ("314-319", "X"), ("320", "D"), ("321", "B"), ("322", "D"),
    ("323-324", "B"), ("325-326", "X"), ("327", "A"), ("328", "C"),
    ("329-330", "D"), ("331-332", "A"), ("333-334", "X"), ("335-336", "C"),
    ("337-338", "X"), ("339", "A"), ("340", "X"), ("341", "B"), ("342", "X"),
    ("343", "B"), ("344", "X"), ("345", "A"), ("346", "X"), ("347", "A"),
    ("348", "D"), ("349", "X"), ("350-351", "D"), ("352", "B"),
    ("353-354", "X"), ("355", "B"), ("356", "X"), ("357", "C"), ("358", "A"),
    ("359", "B"), ("360-361", "X"), ("362-363", "D"), ("364", "C"),
    ("365-367", "A"), ("368", "C"), ("369-370", "X"), ("371", "C"),
    ("372-373", "D"), ("374", "C"), ("375-376", "X"), ("377", "A"),
    ("378", "D"), ("379", "X"), ("380", "C"), ("381", "X"), ("382-383", "D"),
    ("384-389", "C"), ("390", "X"), ("391", "C"), ("392", "X"), ("393", "D"),
    ("394-396", "X"), ("397", "C"), ("398", "X"), ("399-400", "C"),
    ("401", "D"), ("402", "C"), ("403", "X"), ("404-406", "C"), ("407", "B"),
    ("408-410", "X"), ("411-412", "A"), ("413-416", "X"),
]

label_map = {}
for span, lab in RUNS:
    if "-" in span:
        a, b = span.split("-")
        ids = range(int(a), int(b) + 1)
    else:
        ids = [int(span)]
    for i in ids:
        assert i not in label_map, f"duplicate id {i}"
        label_map[i] = lab

assert sorted(label_map) == list(range(417)), (
    f"coverage gap: {sorted(set(range(417)) - set(label_map))[:10]}"
)
LABELS = [label_map[i] for i in range(417)]

rows = list(csv.DictReader(open(DATA / "statements.csv")))
assert len(rows) == len(LABELS)

for r, lab in zip(rows, LABELS):
    r["final_class"] = lab

with open(DATA / "statements_final.csv", "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
    w.writeheader()
    w.writerows(rows)

stats = {}
for spec in ("2.6-202606", "3.0"):
    sub = [r for r in rows if r["spec"] == spec]
    conf = [r for r in sub if r["final_class"] != "X"]
    by_class = {c: len([r for r in conf if r["final_class"] == c]) for c in "ABCD"}
    n = len(conf)
    stats[spec] = {
        "keyword_sentences": len(sub),
        "excluded_non_conformance": len(sub) - n,
        "conformance_statements": n,
        "by_class": by_class,
        "by_class_pct": {c: round(100 * v / n, 1) for c, v in by_class.items()},
        "static_checkable_pct": round(100 * (by_class["A"] + by_class["B"]) / n, 1),
        "by_obligation": {
            k: len([r for r in conf if r["obligation"] == k])
            for k in ("obligation", "recommendation", "permission")
        },
        "class_by_obligation": {
            k: {c: len([r for r in conf if r["obligation"] == k and r["final_class"] == c]) for c in "ABCD"}
            for k in ("obligation", "recommendation", "permission")
        },
    }

# deduplicated by normalized text (boilerplate repeated across objects counts once)
def norm(t):
    t = re.sub(r"`[^`]*`", "F", t.lower())
    return re.sub(r"[^a-z0-9 ]", "", t)


for spec in ("2.6-202606", "3.0"):
    seen = {}
    for r in rows:
        if r["spec"] == spec and r["final_class"] != "X":
            seen.setdefault(norm(r["text"]), r)
    uniq = list(seen.values())
    by_class = {c: len([r for r in uniq if r["final_class"] == c]) for c in "ABCD"}
    n = len(uniq)
    stats[f"{spec}_deduplicated"] = {
        "conformance_statements": n,
        "by_class": by_class,
        "by_class_pct": {c: round(100 * v / n, 1) for c, v in by_class.items()},
        "static_checkable_pct": round(100 * (by_class["A"] + by_class["B"]) / n, 1),
    }

# combined
conf_all = [r for r in rows if r["final_class"] != "X"]
stats["combined"] = {
    "keyword_sentences": len(rows),
    "conformance_statements": len(conf_all),
    "by_class": {c: len([r for r in conf_all if r["final_class"] == c]) for c in "ABCD"},
}

# field-level composition, object tables only (2.6-202606)
frows = [r for r in csv.DictReader(open(DATA / "field_constraints.csv")) if r["object"]]
for spec in ("2.6-202606", "3.0"):
    sub = [r for r in frows if r["spec"] == spec]
    stats[f"fields_{spec}"] = {
        "object_table_rows": len(sub),
        "required": sum(int(r["required"]) for r in sub),
        "recommended": sum(int(r["recommended"]) for r in sub),
        "default": sum(int(r["has_default"]) for r in sub),
        "enum_inline": sum(int(r["enum_inline"]) for r in sub),
        "registry_ref": sum(int(r["registry_ref"]) for r in sub),
    }

json.dump(stats, open(DATA / "final_stats.json", "w"), indent=2)
print(json.dumps(stats, indent=2))
