#!/usr/bin/env python3
"""
Extract normative statements from OpenRTB spec markdown sources.

Outputs (into data/):
  statements.csv       one row per normative sentence (prose + field descriptions)
  field_constraints.csv one row per field-level constraint from object tables
  summary.json         aggregate counts

Statement strata:
  prose       body prose sentence containing a normative marker
  field-desc  sentence from a field description cell containing a marker
  field-type  constraint encoded in the type column (type, required, default)

Auto-classification (pre-labels, reviewed by hand afterwards):
  A  JSON Schema expressible on a single document
  B  statically checkable on a single message, beyond practical JSON Schema
  C  requires cross-message state or runtime context
  D  no machine-decidable conformance criterion
"""

import csv
import json
import re
from pathlib import Path

SPECS = Path.home() / "Documents/workspace/vast-master/rtblint/.openrtb-specs"
OUT = Path(__file__).parent / "data"

SOURCES = {
    "2.6-202606": SPECS / "2.x/openrtb-2.6-202606.md",
    "3.0": SPECS / "3.0/openrtb-3.0-final.md",
}

NORMATIVE = re.compile(
    r"\b(must not|must|shall not|shall|should not|should|may not|may|"
    r"required|recommended|optional|cannot|prohibited|not permitted|not allowed)\b",
    re.IGNORECASE,
)

ABBREV = re.compile(r"\b(e\.g|i\.e|etc|vs|cf|no|v|approx)\.$", re.IGNORECASE)

START_MARKERS = ["# Getting Started", "## Getting Started", "# 1", "# OpenRTB Basics"]


def obligation_type(sentence: str) -> str:
    s = sentence.lower()
    if re.search(r"\b(must|shall|required|cannot|prohibited|not permitted|not allowed)\b", s):
        return "obligation"
    if re.search(r"\b(should|recommended|advised)\b", s):
        return "recommendation"
    return "permission"


def auto_class(sentence: str, stratum: str) -> tuple[str, str]:
    """Heuristic pre-label. Returns (class, rationale)."""
    s = sentence.lower()

    # C: cross-message / runtime context
    if re.search(
        r"\b(win notice|billing notice|loss notice|nurl|burl|lurl|"
        r"notif|substitut|macro|auction_price|timeout|within the (time|window)|"
        r"tmax|expire|expiry|frequency cap|prior (bid|request)|subsequent|"
        r"cookie match|sync|ads\.cert|signature|clear|settle|billable event)\b",
        s,
    ):
        return "C", "runtime/cross-message language"

    # D: bilateral / policy / no decidable criterion
    if re.search(
        r"\b(a priori|out.of.band|bilateral|between the parties|"
        r"business (agreement|relationship|logic)|policy|trust|"
        r"beyond the scope|exchange.specific|mutual agreement|"
        r"coordinated (between|with)|discretion|encouraged|"
        r"commonly|generally|typically|best effort)\b",
        s,
    ):
        return "D", "bilateral/policy language"

    # B: cross-field or external registry
    if re.search(
        r"\b(only one of|at most one|mutually exclusive|must not (also |)contain|"
        r"should not (also |)be (present|included)|one of the following|"
        r"iso.?4217|iso.?639|bcp ?47|iana|ad-id|taxonomy|"
        r"if .* (is|are) (present|specified|included|omitted)|when .* is (present|specified)|"
        r"depend(s|ing) on|instead of|either)\b",
        s,
    ):
        return "B", "cross-field/registry language"

    # A: plain type / presence / enum wording
    if stratum == "field-type":
        return "A", "type column constraint"
    if re.search(r"\bwhere \d+ ?=|\b(integer|string|float|boolean|array)\b.*\brequired\b", s):
        return "A", "type/enum wording"

    ot = obligation_type(sentence)
    if ot == "permission":
        return "D", "permission, no conformance test"
    return "", "needs manual coding"


def split_sentences(text: str):
    text = re.sub(r"\s+", " ", text).strip()
    if not text:
        return
    parts = re.split(r"(?<=[.!?])\s+(?=[A-Z0-9`\"])", text)
    buf = ""
    for p in parts:
        buf = (buf + " " + p).strip() if buf else p
        if ABBREV.search(buf.rstrip(".")):
            continue
        yield buf
        buf = ""
    if buf:
        yield buf


def strip_md(text: str) -> str:
    text = re.sub(r"<[^>]+>", " ", text)
    text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text)
    text = text.replace("**", "").replace("*", "")
    return text


def html_tables_to_md(text: str) -> str:
    """Convert simple HTML tables (OpenRTB 3.0 style) to pipe rows."""

    def convert(m):
        rows_out = []
        for row in re.findall(r"<tr[^>]*>(.*?)</tr>", m.group(0), re.S):
            cells = re.findall(r"<t[dh][^>]*>(.*?)</t[dh]>", row, re.S)
            cells = [re.sub(r"\s+", " ", re.sub(r"<[^>]+>", " ", c)).replace("&nbsp;", " ").strip() for c in cells]
            if cells:
                rows_out.append("| " + " | ".join(cells) + " |")
        if len(rows_out) > 1:
            sep = "| " + " | ".join(["---"] * (rows_out[0].count("|") - 1)) + " |"
            rows_out.insert(1, sep)
        return "\n".join(rows_out)

    return re.sub(r"<table[^>]*>.*?</table>", convert, text, flags=re.S)


def parse_spec(version: str, path: Path):
    lines = html_tables_to_md(path.read_text()).split("\n")

    # find start of normative body (skip front matter, TOC, license)
    start = 0
    for i, ln in enumerate(lines):
        if re.match(r"^#{1,2}\s+.*(Getting Started|OpenRTB Basics|OVERVIEW)", ln):
            start = i
            break

    statements = []
    field_rows = []
    section = ""
    obj = ""
    in_code = False

    i = start
    while i < len(lines):
        ln = lines[i]

        if ln.strip().startswith("```"):
            in_code = not in_code
            i += 1
            continue
        if in_code:
            i += 1
            continue

        h = re.match(r"^#{1,4}\s+(.*)", ln)
        if h:
            section = strip_md(h.group(1)).strip()
            m = re.search(r"Object:\s*(\w+)", section)
            obj = m.group(1) if m else ""
            i += 1
            continue

        # table row
        if ln.strip().startswith("|"):
            cells = [c.strip() for c in ln.strip().strip("|").split("|")]
            if len(cells) >= 3 and not re.match(r"^[-: ]+$", cells[0]):
                attr = strip_md(cells[0]).strip("` ")
                type_spec = strip_md(cells[1]).strip()
                desc = strip_md(" ".join(cells[2:])).replace("<br>", " ")
                if attr.lower() not in ("attribute", "field", "property", "value", "object"):
                    field_rows.append(
                        {
                            "spec": version,
                            "section": section,
                            "object": obj,
                            "field": attr,
                            "type_spec": type_spec,
                            "required": int("required" in type_spec.lower()),
                            "recommended": int("recommended" in type_spec.lower()),
                            "has_default": int("default" in type_spec.lower()),
                            "enum_inline": int(bool(re.search(r"where \d+ ?=|\b0 ?= ?", desc))),
                            "registry_ref": int(
                                bool(re.search(r"ISO.?4217|ISO.?639|BCP ?47|IANA|Ad-ID|[Tt]axonomy", desc))
                            ),
                        }
                    )
                    # normative sentences inside the description cell
                    for sent in split_sentences(desc):
                        if NORMATIVE.search(sent):
                            cls, why = auto_class(sent, "field-desc")
                            statements.append(
                                {
                                    "spec": version,
                                    "stratum": "field-desc",
                                    "section": section,
                                    "object": obj,
                                    "field": attr,
                                    "text": sent,
                                    "obligation": obligation_type(sent),
                                    "auto_class": cls,
                                    "auto_rationale": why,
                                }
                            )
            i += 1
            continue

        # prose paragraph: collect continuation lines
        para = ln.strip()
        while (
            para
            and i + 1 < len(lines)
            and lines[i + 1].strip()
            and not lines[i + 1].strip().startswith(("|", "#", "```", "- ", "* "))
        ):
            i += 1
            para += " " + lines[i].strip()

        para = strip_md(para)
        for sent in split_sentences(para):
            if NORMATIVE.search(sent) and len(sent) > 25:
                cls, why = auto_class(sent, "prose")
                statements.append(
                    {
                        "spec": version,
                        "stratum": "prose",
                        "section": section,
                        "object": obj,
                        "field": "",
                        "text": sent,
                        "obligation": obligation_type(sent),
                        "auto_class": cls,
                        "auto_rationale": why,
                    }
                )
        i += 1

    return statements, field_rows


def main():
    OUT.mkdir(exist_ok=True)
    all_statements, all_fields = [], []
    for version, path in SOURCES.items():
        st, fr = parse_spec(version, path)
        all_statements.extend(st)
        all_fields.extend(fr)
        print(f"{version}: {len(st)} normative sentences, {len(fr)} field rows")

    with open(OUT / "statements.csv", "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(all_statements[0].keys()))
        w.writeheader()
        w.writerows(all_statements)

    with open(OUT / "field_constraints.csv", "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(all_fields[0].keys()))
        w.writeheader()
        w.writerows(all_fields)

    summary = {}
    for v in SOURCES:
        sts = [s for s in all_statements if s["spec"] == v]
        frs = [r for r in all_fields if r["spec"] == v]
        summary[v] = {
            "normative_sentences": len(sts),
            "by_stratum": {
                k: len([s for s in sts if s["stratum"] == k]) for k in ("prose", "field-desc")
            },
            "by_obligation": {
                k: len([s for s in sts if s["obligation"] == k])
                for k in ("obligation", "recommendation", "permission")
            },
            "auto_class": {
                k: len([s for s in sts if s["auto_class"] == k]) for k in ("A", "B", "C", "D", "")
            },
            "field_rows": len(frs),
            "fields_required": sum(r["required"] for r in frs),
            "fields_recommended": sum(r["recommended"] for r in frs),
            "fields_with_default": sum(r["has_default"] for r in frs),
            "fields_enum_inline": sum(r["enum_inline"] for r in frs),
            "fields_registry_ref": sum(r["registry_ref"] for r in frs),
        }
    with open(OUT / "summary.json", "w") as f:
        json.dump(summary, f, indent=2)
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
