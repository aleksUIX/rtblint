#!/usr/bin/env python3
"""Item 2: inspect what the type.mismatch violations actually are, per endpoint.

For each endpoint with heavy type.mismatch counts, validate its payloads and
tabulate mismatches by JSON path with a sample message and observed value,
so real violations can be separated from detector artifacts or vendor
conventions before anyone is named in print.
"""

import json
import subprocess
import sys
from collections import Counter, defaultdict
from pathlib import Path
from urllib.parse import urlparse

RTBLINT = Path.home() / "Documents/workspace/vast-master/rtblint/target/release/rtblint"
VERSION = "2.6-202606"


def get_at_path(body, path):
    """Resolve a dotted/indexed rtblint path like imp[0].video.w against the body."""
    cur = body
    token = ""
    tokens = []
    for ch in path:
        if ch == ".":
            if token:
                tokens.append(token)
            token = ""
        elif ch == "[":
            if token:
                tokens.append(token)
            token = "["
        elif ch == "]":
            tokens.append(token + "]")
            token = ""
        else:
            token += ch
    if token:
        tokens.append(token)
    try:
        for t in tokens:
            if t.startswith("["):
                cur = cur[int(t[1:-1])]
            else:
                cur = cur[t]
        return cur
    except Exception:
        return "<unresolved>"


def main(capture_path):
    by_host = defaultdict(list)
    for line in open(capture_path):
        rec = json.loads(line)
        if rec["kind"] in ("ortb-request", "ortb-response"):
            host = urlparse(rec["endpoint"]).netloc
            kind = "request" if rec["kind"] == "ortb-request" else "response"
            by_host[(host, kind)].append(rec["body"])

    # rank sides by mismatch volume first
    tallies = []
    for (host, kind), bodies in by_host.items():
        if len(bodies) < 10:
            continue
        stdin_data = "\n".join(json.dumps(b, separators=(",", ":")) for b in bodies)
        r = subprocess.run(
            [str(RTBLINT), "validate", "--batch", "--type", kind, "--version", VERSION, "--format", "json"],
            input=stdin_data, capture_output=True, text=True,
        )
        paths = Counter()
        examples = {}
        for line, body in zip(r.stdout.splitlines(), bodies):
            try:
                rep = json.loads(line)
            except json.JSONDecodeError:
                continue
            for it in rep.get("issues", []):
                if it.get("id") != "openrtb.type.mismatch":
                    continue
                # normalize indices so paths aggregate
                norm = []
                depth = 0
                for ch in it["path"]:
                    if ch == "[":
                        depth += 1
                        norm.append("[")
                    elif ch == "]":
                        depth -= 1
                        norm.append("*]")
                    elif depth == 0 or ch not in "0123456789":
                        norm.append(ch)
                key = "".join(norm)
                paths[key] += 1
                if key not in examples:
                    val = get_at_path(body, it["path"])
                    examples[key] = (it["message"][:90], json.dumps(val)[:60])
        if paths:
            tallies.append((host, kind, len(bodies), paths, examples))

    tallies.sort(key=lambda t: -sum(t[3].values()))
    for host, kind, n, paths, examples in tallies[:12]:
        print(f"\n{host} ({kind}s, n={n}, {sum(paths.values())} mismatches)")
        for path, cnt in paths.most_common(6):
            msg, val = examples[path]
            print(f"  {cnt:5d}  {path}")
            print(f"         spec: {msg}")
            print(f"         observed value: {val}")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "captures/full1.jsonl")
