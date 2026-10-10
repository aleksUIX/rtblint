"""Author supplier pod controls from the published Index multi-video rule."""
import copy
import json
from pathlib import Path

root = Path(__file__).resolve().parents[2]
target = root / "fixtures/exchange-depth/index-structured-pods/cases.json"
source = "https://kb.indexexchange.com/publishers/openrtb_integration/list_of_supported_openrtb_bid_request_fields_for_sellers.htm"
video = {"mimes": ["video/mp4"], "w": 640, "h": 360, "minduration": 1, "maxduration": 30, "protocols": [2], "podid": "pod-1"}
base = {"id": "r", "tmax": 100, "device": {"ip": "192.0.2.1"}, "site": {"domain": "publisher.example"}, "imp": [{"id": "1", "video": video}, {"id": "2", "video": copy.deepcopy(video)}]}
cases = []

def add(name, payload, valid, expected=(), forbidden=(), profile="index-exchange-seller"):
    cases.append({"id": name, "profile": profile, "version": "2.6-202606", "direction": "request", "input": payload, "valid": valid, "expected": [{"id": code, "path": path, "severity": "error"} for code, path in expected], "forbidden": list(forbidden), "source": source})

mismatch = "openrtb.profile.index.pod_id_mismatch"
required = "openrtb.profile.field_required"
add("same-pod", copy.deepcopy(base), True)
for value in ["", None]:
    p = copy.deepcopy(base); p["imp"][1]["video"]["podid"] = value
    add("unset-pod-" + str(value), p, False, [(required, "imp[1].video.podid")], [mismatch])
p = copy.deepcopy(base); del p["imp"][1]["video"]["podid"]
add("missing-one-pod", p, False, [(required, "imp[1].video.podid")], [mismatch])
p = copy.deepcopy(base)
for imp in p["imp"]: del imp["video"]["podid"]
add("missing-both-pods", p, False, [(required, "imp[0].video.podid"), (required, "imp[1].video.podid")], [mismatch])
p = copy.deepcopy(base); p["imp"][1]["video"]["podid"] = "pod-2"
add("different-pods", p, False, [(mismatch, "imp[1].video.podid")])
add("spec-allows-different-pods", copy.deepcopy(p), True, forbidden=[mismatch, required], profile="spec")
p = copy.deepcopy(base); p["imp"].append({"id": "3", "video": copy.deepcopy(video)})
add("three-same-pods", p, True)
p = copy.deepcopy(p); p["imp"][1]["video"]["podid"] = "pod-2"; p["imp"][2]["video"]["podid"] = "pod-3"
add("three-different-pods", p, False, [(mismatch, "imp[1].video.podid"), (mismatch, "imp[2].video.podid")])
p = copy.deepcopy(base); p["imp"].append({"id": "3", "banner": {"w": 300, "h": 250}})
add("banner-outside-video-pod", p, True)
p = copy.deepcopy(base); p["imp"] = p["imp"][:1]; del p["imp"][0]["video"]["podid"]
add("single-nonpodded-video", p, True, forbidden=[mismatch, required])
p = copy.deepcopy(p); p["imp"].append({"id": "2", "banner": {"w": 300, "h": 250}})
add("single-video-plus-banner", p, True, forbidden=[mismatch, required])
p = copy.deepcopy(base); p["imp"][1]["video"]["podid"] = 7
add("wrong-pod-type-defers-membership", p, False, [("openrtb.type.mismatch", "imp[1].video.podid")], [mismatch])
for shape in [7, [], "wrong", True]:
    p = copy.deepcopy(base); p["imp"][1]["video"]["podid"] = "pod-2"; p["imp"].append({"id": "3", "video": shape})
    add("malformed-video-defers-" + str(shape), p, False, [("openrtb.type.mismatch", "imp[2].video")], [mismatch])
p = copy.deepcopy(base); p["imp"][1]["video"]["podid"] = "pod-2"; p["imp"].append(None)
add("malformed-imp-defers-membership", p, False, forbidden=[mismatch])
target.parent.mkdir(parents=True, exist_ok=True)
target.write_text(json.dumps({"source": source, "cases": cases}, indent=2) + "\n")
print(len(cases))
