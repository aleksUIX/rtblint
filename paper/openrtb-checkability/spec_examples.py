#!/usr/bin/env python3
"""Validate the example payloads embedded in the OpenRTB 2.6 spec text
against the spec's own object tables, using rtblint.

Extracts every fenced JSON code block that parses as a bid request or bid
response from the archived 2.6-202606 markdown, runs rtblint against the
matching payload type and version, and reports per-example results. Also
greps older 2.6 monthly releases for the two defective example fragments
to establish how long they have been present.
"""

import json
import re
import subprocess
from pathlib import Path

RTBLINT = Path.home() / "Documents/workspace/vast-master/rtblint"
SPECS = RTBLINT / ".openrtb-specs/2.x"
CLI = RTBLINT / "target/release/rtblint"
OUT = Path(__file__).parent / "data"

SPEC = SPECS / "openrtb-2.6-202606.md"
VERSION = "2.6-202606"


def extract_examples(text):
    blocks = re.findall(r"```[a-z]*\s*\n(\{.*?\})\s*\n```", text, re.S)
    for i, b in enumerate(blocks):
        try:
            j = json.loads(b)
        except ValueError:
            continue
        if not isinstance(j, dict):
            continue
        if "imp" in j:
            yield i, "request", b
        elif "seatbid" in j or "nbr" in j:
            yield i, "response", b


def main():
    results = []
    for i, kind, raw in extract_examples(SPEC.read_text()):
        p = Path(f"/tmp/ortb-ex{i:02d}.json")
        p.write_text(raw)
        r = subprocess.run(
            [str(CLI), "validate", "--type", kind, "--version", VERSION,
             "--format", "json", str(p)],
            capture_output=True, text=True,
        )
        rep = json.loads(r.stdout)
        results.append(
            {
                "example_index": i,
                "type": kind,
                "valid": rep.get("valid"),
                "issues": [
                    {k: it.get(k) for k in ("severity", "rule", "rule_id", "path", "message")}
                    for it in rep.get("issues", [])
                ],
            }
        )

    n_bad = sum(1 for r in results if not r["valid"])
    print(f"{len(results)} example payloads extracted from {SPEC.name}; {n_bad} invalid")
    for r in results:
        flag = "INVALID" if not r["valid"] else "valid"
        print(f"  ex{r['example_index']:02d} {r['type']:8s} {flag}")
        for it in r["issues"]:
            print(f"      {it['severity']}: {it['path']}: {str(it['message'])[:90]}")

    # persistence of the two defects across the 2.6 line
    persistence = {}
    for md in sorted(SPECS.glob("openrtb-2.6-*.md")):
        t = md.read_text()
        persistence[md.stem] = {
            "data_value_example": '"value": "30-40"' in t or '"value":"30-40"' in t,
            "video_apis_example": bool(re.search(r'"apis"\s*:', t)),
        }
    print("\ndefect persistence across archived 2.6 releases:")
    for k, v in persistence.items():
        print(f"  {k}: data.value={v['data_value_example']} video.apis={v['video_apis_example']}")

    json.dump(
        {"spec": SPEC.name, "results": results, "persistence": persistence},
        open(OUT / "spec_examples.json", "w"),
        indent=1,
    )


if __name__ == "__main__":
    main()
