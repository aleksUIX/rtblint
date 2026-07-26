#!/usr/bin/env python3
"""Per-version catalog stats from rtblint object catalogs + rule id census."""

import csv
import json
import re
from pathlib import Path

RTBLINT = Path.home() / "Documents/workspace/vast-master/rtblint"
SPECS = RTBLINT / "crates/rtblint-core/specs"
OUT = Path(__file__).parent / "data"

ORDER = [
    "2.0", "2.1", "2.2", "2.3", "2.3.1", "2.4", "2.5",
    "2.6-202204", "2.6-202210", "2.6-202211", "2.6-202303", "2.6-202309",
    "2.6-202402", "2.6-202409", "2.6-202501", "2.6-202505", "2.6-202606",
    "3.0",
]

rows = []
for v in ORDER:
    p = SPECS / f"openrtb-{v}-object-catalog.json"
    if not p.exists():
        print(f"missing: {p.name}")
        continue
    cat = json.load(open(p))
    objs = cat["objects"]
    fields = [f for o in objs for f in o["fields"]]
    req = [f for f in fields if "required" in f["type_spec"].lower()]
    rec = [f for f in fields if "recommended" in f["type_spec"].lower()]
    dep = [f for f in fields if "deprecated" in f["type_spec"].lower()]
    rows.append(
        {
            "version": v,
            "release_date": cat.get("release_date", ""),
            "objects": len(objs),
            "fields": len(fields),
            "required": len(req),
            "recommended": len(rec),
            "deprecated": len(dep),
        }
    )

with open(OUT / "catalog_versions.csv", "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
    w.writeheader()
    w.writerows(rows)

for r in rows:
    print(r)

# rule id census across core source
src = (RTBLINT / "crates/rtblint-core/src").glob("*.rs")
ids = set()
for p in src:
    ids |= set(re.findall(r'"(openrtb\.[a-z0-9_.]+)"', p.read_text()))
print(f"\nrule ids: {len(ids)}")

# AdCOM / enum lists
lists_src = (RTBLINT / "crates/rtblint-core/src/adcom_lists.rs").read_text()
list_names = re.findall(r'name:\s*"([^"]+)"', lists_src)
print(f"adcom lists: {len(set(list_names))}")
json.dump(
    {"rule_ids": sorted(ids), "adcom_lists": sorted(set(list_names))},
    open(OUT / "rtblint_rules.json", "w"),
    indent=1,
)
