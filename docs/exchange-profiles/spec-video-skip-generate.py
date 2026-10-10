"""Preserve the IAB Video.skip binary-flag boundary in each corrected snapshot."""
import json
from pathlib import Path

root = Path(__file__).resolve().parents[2]
versions = ["202210", "202211", "202303", "202309", "202402", "202409", "202501", "202505", "202606"]
cases = []
for release in versions:
    for flag in [0, 1, 2, 16, 500]:
        cases.append({
            "id": f"{release}-skip-{flag}", "profile": "spec", "direction": "request",
            "version": "2.6-" + release,
            "input": {"id": "r", "imp": [{"id": "i", "video": {"mimes": ["video/mp4"], "skip": flag}}]},
            "valid": flag in [0, 1],
            "expected": [] if flag in [0, 1] else [{"id": "openrtb.value.invalid", "path": "imp[0].video.skip", "severity": "error"}],
            "source": "IAB OpenRTB 2.6 section 3.2.7, Video.skip: binary flag. Per-release source archives are under .openrtb-specs/2.x.",
        })
target = root / "fixtures/exchange-depth/spec-video-skip/cases.json"
target.parent.mkdir(parents=True, exist_ok=True)
target.write_text(json.dumps({"cases": cases}, indent=2) + "\n")
print(len(cases))
