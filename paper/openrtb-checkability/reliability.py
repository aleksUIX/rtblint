#!/usr/bin/env python3
"""Inter-rater reliability: author labels vs two blind recoding runs.

The 60-statement sample (data/reliability_sample.csv, seed 42) was recoded
by two independent runs of a large language model given only the Section 3.3
codebook, blind to the author's labels (data/coder1_labels.csv,
data/coder2_labels.csv). Computes percent agreement and Cohen's kappa for
each pair and lists disagreements.
"""

import csv
import json
from collections import Counter
from pathlib import Path

DATA = Path(__file__).parent / "data"
CATS = ["A", "B", "C", "D", "X"]


def load_labels(path, key, val):
    return {int(r[key]): r[val] for r in csv.DictReader(open(path))}


def kappa(a, b, ids):
    n = len(ids)
    po = sum(1 for i in ids if a[i] == b[i]) / n
    ca, cb = Counter(a[i] for i in ids), Counter(b[i] for i in ids)
    pe = sum((ca[c] / n) * (cb[c] / n) for c in CATS)
    return po, (po - pe) / (1 - pe)


author_all = {
    int(i): r["final_class"]
    for i, r in enumerate(csv.DictReader(open(DATA / "statements_final.csv")))
}
sample_ids = [int(r["id"]) for r in csv.DictReader(open(DATA / "reliability_sample.csv"))]
author = {i: author_all[i] for i in sample_ids}
c1 = load_labels(DATA / "coder1_labels.csv", "id", "code")
c2 = load_labels(DATA / "coder2_labels.csv", "id", "code")

out = {}
for name, other in (("author_vs_run1", c1), ("author_vs_run2", c2)):
    po, k = kappa(author, other, sample_ids)
    out[name] = {"percent_agreement": round(100 * po, 1), "kappa": round(k, 3)}
po, k = kappa(c1, c2, sample_ids)
out["run1_vs_run2"] = {"percent_agreement": round(100 * po, 1), "kappa": round(k, 3)}

disagreements = [
    {"id": i, "author": author[i], "run1": c1[i], "run2": c2[i]}
    for i in sample_ids
    if not (author[i] == c1[i] == c2[i])
]
out["disagreements"] = disagreements

json.dump(out, open(DATA / "reliability_results.json", "w"), indent=1)
print(json.dumps(out, indent=1))
