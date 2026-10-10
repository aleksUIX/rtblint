"""Build synthetic type cases from pinned vendor descriptors, without reading Rust.

The descriptor inventory comes from Magnite's public proto2 declarations.
Semantic and ambiguity controls below are separately authored from vendor prose.
"""
import copy
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[5]
DESCRIPTORS = json.loads((ROOT / "docs/exchange-profiles/xandr-magnite-descriptors.json").read_text())["messages"]
CASES = []


def request():
    return {"id": "request-1", "imp": [{"id": "imp-1", "banner": {"w": 300, "h": 250}}]}


def response():
    return {"id": "request-1", "seatbid": [{"seat": "buyer-code", "bid": [{"id": "bid-1", "impid": "imp-1", "price": 1.5}]}]}


def set_path(value, parts, child):
    for index, key in enumerate(parts[:-1]):
        next_key = parts[index + 1]
        if isinstance(key, int):
            while len(value) <= key:
                value.append([] if isinstance(next_key, int) else {})
            value = value[key]
        else:
            value = value.setdefault(key, [] if isinstance(next_key, int) else {})
    if isinstance(parts[-1], int):
        while len(value) <= parts[-1]:
            value.append(None)
    value[parts[-1]] = child


def display_path(parts):
    text = ""
    for part in parts:
        text += f"[{part}]" if isinstance(part, int) else ("." if text else "") + part
    return text


PLACEMENTS = {
    "BidRequestExt": ("request", []), "ImpExt": ("request", ["imp", 0]),
    "VideoExt": ("request", ["imp", 0, "video"]), "BannerExt": ("request", ["imp", 0, "banner"]),
    "AppExt": ("request", ["app"]), "SiteExt": ("request", ["site"]), "DoohExt": ("request", ["dooh"]),
    "PublisherExt": ("request", ["site", "publisher"]), "DeviceExt": ("request", ["device"]),
    "GeoExt": ("request", ["device", "geo"]), "UserExt": ("request", ["user"]),
    "RegsExt": ("request", ["regs"]), "SourceExt": ("request", ["source"]),
    "NativeExt": ("request", ["imp", 0, "native"]), "AudioExt": ("request", ["imp", 0, "audio"]),
    "BidResponseExt": ("response", []), "SeatBidExt": ("response", ["seatbid", 0]),
    "BidExt": ("response", ["seatbid", 0, "bid", 0]),
    "NativeRequestExt": ("native_request", []), "EventTrackerExt": ("native_response", ["eventtrackers", 0]),
}


def base_for(root):
    mode, place = PLACEMENTS[root]
    if mode == "native_request":
        return {"ver": "1.2", "assets": [{"id": 1, "title": {"len": 80}}]}, mode, place
    if mode == "native_response":
        return {"ver": "1.2", "assets": [{"id": 1, "title": {"text": "Example"}}],
                "link": {"url": "https://advertiser.example/"},
                "eventtrackers": [{"event": 1, "method": 1, "url": "https://tracker.example/i"}]}, mode, place
    value = request() if mode == "request" else response()
    if root == "VideoExt":
        value["imp"][0] = {"id": "imp-1", "video": {"mimes": ["video/mp4"], "protocols": [2]}}
    if root == "AudioExt":
        value["imp"][0] = {"id": "imp-1", "audio": {"mimes": ["audio/mpeg"], "protocols": [2]}}
    if root == "NativeExt":
        value["imp"][0] = {"id": "imp-1", "native": {"ver": "1.2", "request": json.dumps({"ver": "1.2", "assets": [{"id": 1, "title": {"len": 80}}]})}}
    if root == "AppExt":
        value["app"] = {"id": "app-1", "bundle": "com.example.app"}
    if root in ("SiteExt", "PublisherExt"):
        value["site"] = {"id": "site-1", "domain": "publisher.example", "publisher": {"id": "publisher-1"}}
    return value, mode, place


def add(name, profile, mode, payload, issues=(), valid=None, source="vendor-prose", request_payload=None, warnings=(), required=()):
    if mode.startswith("native_"):
        native = payload
        if mode == "native_request":
            payload = request()
            payload["imp"][0] = {"id": "imp-1", "native": {"ver": "1.2", "request": json.dumps(native)}}
            mode = "request"
        else:
            payload = response()
            payload["seatbid"][0]["bid"][0]["adm"] = json.dumps(native)
            mode = "response"
    case = {"name": name, "profile": profile, "mode": mode, "payload": copy.deepcopy(payload),
            "expected_profile_errors": [{"id": identifier, "path": path} for identifier, path in issues],
            "source": source}
    if valid is not None:
        case["valid"] = valid
    if warnings:
        case["expected_profile_warnings"] = [{"id": identifier, "path": path} for identifier, path in warnings]
    if required:
        case["required_issues"] = [{"id": identifier, "path": path} for identifier, path in required]
    if request_payload is not None:
        case["request"] = copy.deepcopy(request_payload)
    CASES.append(case)


def resolve(field):
    if field["type"] in {"int32", "uint32", "uint64", "double", "string", "bool"}:
        return field["type"]
    scope = field["scope"].split(".")
    for n in range(len(scope), -1, -1):
        name = ".".join(scope[:n] + [field["type"]])
        if name in DESCRIPTORS:
            return name
    raise ValueError(field)


def paths(message, prefix):
    for field in DESCRIPTORS[message]:
        path = prefix + [field["name"]]
        kind = resolve(field)
        yield field, path, kind
        if kind in DESCRIPTORS:
            yield from paths(kind, path + ([0] if field["repeated"] else []))


for root in PLACEMENTS:
    base, mode, place = base_for(root)
    add("magnite-" + root + "-optional-absence", "magnite", mode, base, valid=True, source="proto2-optional")
    for field, path, kind in paths(root, place + ["ext"]):
        good = {"int32": 1, "uint32": 1, "uint64": 1, "double": 1.25, "string": "example", "bool": True}.get(kind, {})
        if path[-1] == "orientation":
            good = "h"
        if path[-1] == "mime":
            good = "text/html"
        if path[-1] == "sensitivity":
            good = "high"
        if root == "BidResponseExt" and path[-1] == "time":
            good = 20
        if root == "BidExt" and path[-1] == "adtype":
            good = "banner"
        if field["repeated"]:
            good = []
        bad = {} if field["repeated"] else ([] if kind in DESCRIPTORS else (17 if kind == "string" else "bad"))
        if field["repeated"] and kind == "KeyValuePair":
            bad = 17  # JSON maps and repeated-message arrays are both supported.
        name = root + "-" + display_path(path)
        for suffix, val, expected in [("valid", good, ()), ("wrong-type", bad, (("openrtb.profile.magnite.type_invalid", display_path(path)),))]:
            payload = copy.deepcopy(base)
            set_path(payload, path, val)
            fullpath = display_path(path)
            if mode == "native_request":
                fullpath = "imp[0].native.request." + fullpath
            if mode == "native_response":
                fullpath = "seatbid[0].bid[0].adm." + fullpath
            expected = tuple((identifier, fullpath) for identifier, _ in expected)
            add("magnite-" + name + "-" + suffix, "magnite", mode, payload, expected,
                valid=suffix == "valid", source="proto2-declaration:" + field["scope"] + "." + field["name"])


def rqcase(name, profile, parts, value, issues=(), valid=True, warnings=()):
    payload = request()
    set_path(payload, parts, value)
    add(name, profile, "request", payload, issues, valid=valid, warnings=warnings)


def bidcase(name, profile, parts, value, issues=(), valid=True, warnings=()):
    payload = response()
    set_path(payload, ["seatbid", 0, "bid", 0] + parts, value)
    add(name, profile, "response", payload, issues, valid=valid, warnings=warnings)


for field, minimum, maximum in [("zone_id", -(2**31), 2**31 - 1)]:
    for value in [minimum, maximum]:
        rqcase(f"magnite-int32-boundary-{value}", "magnite", ["imp", 0, "ext", "rp", field], value)
    for value in [minimum - 1, maximum + 1]:
        rqcase(f"magnite-int32-overflow-{value}", "magnite", ["imp", 0, "ext", "rp", field], value,
               [("openrtb.profile.magnite.type_invalid", "imp[0].ext.rp.zone_id")], False)
for value in [0, 2**32 - 1]:
    rqcase(f"magnite-uint32-boundary-{value}", "magnite", ["site", "ext", "rp", "site_id"], value)
for value in [-1, 2**32]:
    rqcase(f"magnite-uint32-overflow-{value}", "magnite", ["site", "ext", "rp", "site_id"], value,
           [("openrtb.profile.magnite.type_invalid", "site.ext.rp.site_id")], False)
rqcase("magnite-optional-null", "magnite", ["imp", 0, "ext", "rp", "zone_id"], None)
rqcase("magnite-target-json-map", "magnite", ["imp", 0, "ext", "rp", "target"], {"section": ["news", "science"]})
rqcase("magnite-target-proto-repeated", "magnite", ["imp", 0, "ext", "rp", "target"], [{"key": "section", "value": ["news"]}])
rqcase("magnite-target-invalid-map-element", "magnite", ["imp", 0, "ext", "rp", "target"], {"section": [True]},
       [("openrtb.profile.magnite.type_invalid", "imp[0].ext.rp.target.section[0]")], False)
rqcase("magnite-unknown-extension-retained", "magnite", ["imp", 0, "ext", "rp", "future_field"], {"arbitrary": True})
for name, parts, value, path in [
    ("orientation", ["imp", 0, "video", "ext", "orientation"], "landscape", "imp[0].video.ext.orientation"),
    ("banner-mime", ["imp", 0, "banner", "ext", "rp", "mime"], "image/png", "imp[0].banner.ext.rp.mime"),
    ("ad-quality", ["site", "ext", "rp", "aq", "sensitivity"], "medium", "site.ext.rp.aq.sensitivity"),
    ("geo-consent", ["device", "geo", "ext", "rp", "consent"], 2, "device.geo.ext.rp.consent"),
]:
    payload = request()
    if name == "orientation":
        payload["imp"][0] = {"id": "imp-1", "video": {"mimes": ["video/mp4"], "protocols": [2]}}
    set_path(payload, parts, value)
    add("magnite-invalid-" + name, "magnite", "request", payload, [("openrtb.profile.value_invalid", path)], valid=False)
for field, maximum in [("badvid", 20), ("baindid", 50)]:
    rqcase("magnite-" + field + "-limit", "magnite", ["ext", "rp", field], list(range(maximum)))
    rqcase("magnite-" + field + "-over-limit", "magnite", ["ext", "rp", field], list(range(maximum + 1)),
           [("openrtb.profile.magnite.array_limit", "ext.rp." + field)], False)
for value in [0, 10, 20]:
    payload = response(); set_path(payload, ["ext", "rp", "time"], value)
    add(f"magnite-time-valid-{value}", "magnite", "response", payload, valid=True)
for value in [-10, 11]:
    payload = response(); set_path(payload, ["ext", "rp", "time"], value)
    add(f"magnite-time-invalid-{value}", "magnite", "response", payload, [("openrtb.profile.value_invalid", "ext.rp.time")], valid=False)
bidcase("magnite-invalid-adtype", "magnite", ["ext", "rp", "adtype"], "display", [("openrtb.profile.value_invalid", "seatbid[0].bid[0].ext.rp.adtype")], False)
bidcase("magnite-invalid-estimated", "magnite", ["ext", "rp", "estimated"], 2, [("openrtb.profile.value_invalid", "seatbid[0].bid[0].ext.rp.estimated")], False)
bidcase("magnite-invalid-response-format", "magnite", ["ext", "rp", "response_format"], 5, [("openrtb.profile.value_invalid", "seatbid[0].bid[0].ext.rp.response_format")], False)
rqcase("magnite-deprecated-flag-warning", "magnite", ["imp", 0, "banner", "ext", "rp", "usenurl"], True,
       warnings=[("openrtb.profile.magnite.deprecated", "imp[0].banner.ext.rp.usenurl")])


# Xandr prose-derived boundaries and direction controls.
add("xandr-optional-absence", "xandr", "request", request(), valid=True)
for context in range(11):
    payload = request(); payload["imp"][0] = {"id": "imp-1", "video": {"mimes": ["video/mp4"], "protocols": [2], "ext": {"appnexus": {"context": context}}}}
    add(f"xandr-context-{context}", "xandr", "request", payload, valid=True, source="bidder-2.6-context-table")
for context in [-1, 11, "1", True]:
    payload = request(); payload["imp"][0] = {"id": "imp-1", "video": {"mimes": ["video/mp4"], "protocols": [2], "ext": {"appnexus": {"context": context}}}}
    add("xandr-context-invalid-" + str(context), "xandr", "request", payload, [("openrtb.profile.value_invalid", "imp[0].video.ext.appnexus.context")], valid=False)
rqcase("xandr-supplier-markup-field-not-misapplied", "xandr", ["ext", "appnexus", "markup_delivery"], 2)
rqcase("xandr-seller-member-type", "xandr", ["ext", "appnexus", "seller_member_id"], "123",
       [("openrtb.profile.xandr.type_invalid", "ext.appnexus.seller_member_id")], False)
rqcase("xandr-appnexus-parent-type", "xandr", ["ext", "appnexus"], [], [("openrtb.profile.xandr.type_invalid", "ext.appnexus")], False)
for flag in [0, 1, True, False]:
    rqcase("xandr-header-flag-" + str(flag), "xandr", ["ext", "appnexus", "publisher_integration", "is_header"], flag)
rqcase("xandr-header-flag-invalid", "xandr", ["ext", "appnexus", "publisher_integration", "is_header"], 2,
       [("openrtb.profile.xandr.type_invalid", "ext.appnexus.publisher_integration.is_header")], False)
for tidt in [1, 2]:
    rqcase(f"xandr-tidt-{tidt}", "xandr", ["source", "ext", "tidt"], tidt)
rqcase("xandr-tidt-invalid", "xandr", ["imp", 0, "ext", "tidt"], 3, [("openrtb.profile.value_invalid", "imp[0].ext.tidt")], False)
rqcase("xandr-badv-limit", "xandr", ["badv"], [f"advertiser-{i}.example" for i in range(64)])
rqcase("xandr-badv-over-limit", "xandr", ["badv"], [f"advertiser-{i}.example" for i in range(65)], [("openrtb.profile.xandr.array_limit", "badv")], False)
payload = response(); del payload["seatbid"][0]["seat"]
add("xandr-response-seat-required", "xandr", "response", payload, [("openrtb.profile.field_required", "seatbid[0].seat")], valid=False)
add("xandr-nonnumeric-registered-seat", "xandr", "response", response(), valid=True)
add("xandr-json-no-bid-representation", "xandr", "response", {"id": "request-1", "seatbid": [], "nbr": 0}, valid=True)
for count in [550, 551]:
    bidcase(f"xandr-custom-macro-value-{count}", "xandr", ["ext", "appnexus", "custom_macros"], [{"name": "campaign", "value": "é" * count}],
            [] if count == 550 else [("openrtb.profile.xandr.string_limit", "seatbid[0].bid[0].ext.appnexus.custom_macros[0].value")], count == 550)
bidcase("xandr-custom-macro-value-must-be-string", "xandr", ["ext", "appnexus", "custom_macros"], [{"name": "campaign", "value": 42}],
        [("openrtb.profile.xandr.type_invalid", "seatbid[0].bid[0].ext.appnexus.custom_macros[0].value")], False)
for field, macro in [("nurl", "AUCTION_PRICE"), ("burl", "AN_PAYMENT_TYPE"), ("lurl", "AUCTION_LOSS"), ("lurl", "AUCTION_MIN_TO_WIN")]:
    bidcase(f"xandr-{field}-{macro}", "xandr", [field], "https://notify.example/?value=${" + macro + "}")
for field, macro in [("nurl", "AUCTION_LOSS"), ("burl", "AUCTION_MIN_TO_WIN"), ("lurl", "AUCTION_PRICE"), ("nurl", "CUSTOM_ONLY_IN_CREATIVE")]:
    bidcase(f"xandr-{field}-unsupported-{macro}", "xandr", [field], "https://notify.example/?value=${" + macro + "}", [("openrtb.profile.xandr.notify_macro", "seatbid[0].bid[0]." + field)], False)
for count in [2000, 2001]:
    prefix = "https://notify.example/"
    bidcase(f"xandr-static-notify-length-{count}", "xandr", ["nurl"], prefix + "a" * (count - len(prefix)),
            [] if count == 2000 else [("openrtb.profile.xandr.string_limit", "seatbid[0].bid[0].nurl")], count == 2000)
bidcase("xandr-macro-expansion-length-unknown", "xandr", ["nurl"], "https://notify.example/${AUCTION_ID}" + "a" * 2000)
for count in [100, 101]:
    bidcase(f"xandr-dsa-unicode-length-{count}", "xandr", ["ext", "dsa"], {"behalf": "é" * count, "paid": "Advertiser"},
            [] if count == 100 else [("openrtb.profile.xandr.string_limit", "seatbid[0].bid[0].ext.dsa.behalf")], count == 100)
bidcase("xandr-dsa-render-invalid", "xandr", ["ext", "dsa"], {"behalf": "Advertiser", "paid": "Advertiser", "adrender": 2},
        [("openrtb.profile.value_invalid", "seatbid[0].bid[0].ext.dsa.adrender")], False)
for kind in [1, 2, 6, 8, 9]:
    payload = response(); bid = payload["seatbid"][0]["bid"][0]
    bid["ext"] = {"appnexus": {"bid_payment_type": [{"payment_type": kind, "price": 1.5}]}}
    if kind != 1:
        bid["burl"] = "https://notify.example/bill"
    add(f"xandr-payment-type-{kind}", "xandr", "response", payload, valid=True)
bidcase("xandr-payment-type-invalid", "xandr", ["ext", "appnexus", "bid_payment_type"], [{"payment_type": 3}],
        [("openrtb.profile.value_invalid", "seatbid[0].bid[0].ext.appnexus.bid_payment_type[0].payment_type")], False)
bidcase("xandr-view-payment-needs-burl", "xandr", ["ext", "appnexus", "bid_payment_type"], [{"payment_type": 8}],
        [("openrtb.profile.xandr.billing_url_required", "seatbid[0].bid[0].burl")], False)


def paircase(name, profile, req, res, issues=(), valid=True):
    add(name, profile, "pair", res, issues, valid=valid, request_payload=req)


for tracker in ["https://tracker.example/i", "HTTPS://tracker.example/i", "http://tracker.example/i", "//tracker.example/i"]:
    req = request(); req["imp"][0]["secure"] = 1
    res = response(); res["seatbid"][0]["bid"][0]["ext"] = {"rp": {"imptrackers": [tracker]}}
    valid = tracker.lower().startswith("https://")
    paircase("magnite-secure-tracker-" + tracker.split(":")[0], "magnite", req, res,
             [] if valid else [("openrtb.profile.magnite.insecure_tracker", "seatbid[0].bid[0].ext.rp.imptrackers[0]")], valid)
req = request(); res = response(); res["seatbid"][0]["bid"][0]["ext"] = {"rp": {"imptrackers": ["http://tracker.example/i"]}}
paircase("magnite-unknown-secure-state", "magnite", req, res)
for selected in [1, 2, 3]:
    req = request(); req["imp"][0]["banner"]["ext"] = {"rp": {"size_id": 1, "alt_size_ids": [2], "mime": "text/html"}}
    res = response(); res["seatbid"][0]["bid"][0]["ext"] = {"rp": {"adtype": "banner", "size_id": selected, "mime": "text/html"}}
    paircase(f"magnite-selected-size-{selected}", "magnite", req, res,
             [] if selected != 3 else [("openrtb.profile.magnite.size_not_offered", "seatbid[0].bid[0].ext.rp.size_id")], selected != 3)
req["imp"][0]["ext"] = {"rp": {"slot": "configured-slot"}}
paircase("magnite-slot-sizes-are-account-configured", "magnite", req, res)
req = request(); req["imp"][0]["banner"]["ext"] = {"rp": {"mime": "text/html"}}
res = response(); res["seatbid"][0]["bid"][0]["ext"] = {"rp": {"response_format": 1, "mime": "application/javascript"}}
paircase("magnite-mime-mismatch", "magnite", req, res, [("openrtb.profile.magnite.mime_mismatch", "seatbid[0].bid[0].ext.rp.mime")], False)
req = request(); res = response(); res["seatbid"][0]["bid"][0]["ext"] = {"rp": {"response_format": 2}}
paircase("magnite-vendor-media-mismatch", "magnite", req, res, [("openrtb.profile.magnite.media_not_offered", "seatbid[0].bid[0].ext.rp.response_format")], False)
req = request(); req["imp"][0]["banner"]["api"] = [5]
res = response(); res["seatbid"][0]["bid"][0]["ext"] = {"rp": {"adtype": "banner", "creativeapi": 7}}
paircase("magnite-api-not-offered", "magnite", req, res, [("openrtb.profile.magnite.api_not_offered", "seatbid[0].bid[0].ext.rp.creativeapi")], False)
req["imp"][0]["banner"]["api"] = [5, 7]
paircase("magnite-api-offered", "magnite", req, res)
for offered in [None, [1], [1, 8]]:
    req = request()
    if offered is not None:
        req["imp"][0]["ext"] = {"appnexus": {"allowed_payment_types": [{"payment_type": kind} for kind in offered]}}
    res = response(); bid = res["seatbid"][0]["bid"][0]
    bid["ext"] = {"appnexus": {"bid_payment_type": [{"payment_type": 8}]}}; bid["burl"] = "https://notify.example/bill"
    valid = offered is not None and 8 in offered
    paircase("xandr-payment-offer-" + str(offered), "xandr", req, res,
             [] if valid else [("openrtb.profile.xandr.payment_not_offered", "seatbid[0].bid[0].ext.appnexus.bid_payment_type[0].payment_type")], valid)
req = request(); req["imp"][0]["ext"] = {"appnexus": {"allowed_payment_types": [{"payment_type": 8}]}}; req["cur"] = ["EUR"]
res = response(); res["cur"] = "EUR"; bid = res["seatbid"][0]["bid"][0]
bid["ext"] = {"appnexus": {"bid_payment_type": [{"payment_type": 8}]}}; bid["burl"] = "https://notify.example/bill"
paircase("xandr-view-payment-non-usd", "xandr", req, res, [("openrtb.profile.xandr.payment_currency", "cur")], False)
for duration in [30, 61]:
    req = request(); req["imp"][0] = {"id": "imp-1", "video": {"mimes": ["video/mp4"], "protocols": [2], "poddur": 60, "maxseq": 1}}
    res = response(); res["seatbid"][0]["bid"][0]["dur"] = duration
    # Offers may total more than poddur/maxseq. Only selected winners are assembled.
    if duration == 30:
        res["seatbid"][0]["bid"] += [{"id": "bid-2", "impid": "imp-1", "price": 2, "dur": 30}, {"id": "bid-3", "impid": "imp-1", "price": 3, "dur": 30}]
    paircase(f"xandr-pod-candidate-duration-{duration}", "xandr", req, res,
             [] if duration == 30 else [("openrtb.profile.xandr.pod_duration", "seatbid[0].bid[0].dur")], duration == 30)
req = request(); req["imp"][0] = {"id": "imp-1", "video": {"mimes": ["video/mp4"], "protocols": [2]}}
res = response(); res["seatbid"][0]["bid"][0]["slotinpod"] = 1
paircase("xandr-guaranteed-slot-not-offered", "xandr", req, res, [("openrtb.profile.xandr.slot_not_offered", "seatbid[0].bid[0].slotinpod")], False)
req["imp"][0]["video"]["slotinpod"] = 1
paircase("xandr-guaranteed-slot-offered", "xandr", req, res)

# Type rows authored directly from Xandr's documented extension tables.
for parts, good, bad in [
    (["ext", "appnexus", "ext_inv_code"], 10039, "10039"),
    (["ext", "appnexus", "publisher_integration"], {}, []),
    (["imp", 0, "ext", "appnexus", "estimated_clear_price"], 0.57, "0.57"),
    (["imp", 0, "ext", "appnexus", "predicted_view_rate"], 0.07, True),
    (["imp", 0, "ext", "appnexus", "predicted_view_rate_over_total"], 0.06, True),
    (["imp", 0, "ext", "appnexus", "predicted_video_view_rate"], 0.07, True),
    (["imp", 0, "ext", "appnexus", "predicted_video_view_rate_over_total"], 0.06, True),
    (["imp", 0, "ext", "appnexus", "predicted_video_completion_rate"], 0.59, "0.59"),
    (["imp", 0, "ext", "appnexus", "member_ad_profile_id"], 111, "111"),
    (["imp", 0, "ext", "appnexus", "traffic_source_code"], "source-1", 1),
    (["imp", 0, "ext", "appnexus", "gpid"], "placement-1", 1),
    (["imp", 0, "ext", "tid"], "break-1", 1),
    (["device", "ext", "ifa_type"], "publisher-configured-future-type", 1),
    (["device", "geo", "ext", "appnexus", "timezone"], "America/Los_Angeles", 1),
]:
    path = display_path(parts)
    rqcase("xandr-type-" + path + "-valid", "xandr", parts, good)
    rqcase("xandr-type-" + path + "-invalid", "xandr", parts, bad, [("openrtb.profile.xandr.type_invalid", path)], False)
for parts, good, bad in [
    (["ext", "appnexus", "min_price"], 1.2, "1.2"),
    (["ext", "appnexus", "custom_notify_data"], "opaque-data", 1),
    (["ext", "appnexus", "click_url"], "https://click.example/?redirect=", 1),
]:
    path = "seatbid[0].bid[0]." + display_path(parts)
    bidcase("xandr-type-" + path + "-valid", "xandr", parts, good)
    bidcase("xandr-type-" + path + "-invalid", "xandr", parts, bad, [("openrtb.profile.xandr.type_invalid", path)], False)
bidcase("xandr-payment-large-integer-not-an-enum", "xandr", ["ext", "appnexus", "bid_payment_type"], [{"payment_type": 2**63}],
        [("openrtb.profile.value_invalid", "seatbid[0].bid[0].ext.appnexus.bid_payment_type[0].payment_type")], False)
bidcase("xandr-dsa-transparency-types", "xandr", ["ext", "dsa"], {"behalf": "Advertiser", "paid": "Advertiser", "transparency": [{"domain": "buyer.example", "params": [1, 2]}]})
bidcase("xandr-dsa-transparency-invalid-types", "xandr", ["ext", "dsa"], {"behalf": "Advertiser", "paid": "Advertiser", "transparency": [{"domain": 1, "params": [True]}]},
        [("openrtb.profile.xandr.type_invalid", "seatbid[0].bid[0].ext.dsa.transparency[0].domain"),
         ("openrtb.profile.xandr.type_invalid", "seatbid[0].bid[0].ext.dsa.transparency[0].params")], False)
res = response(); res["cur"] = "EUR"; bid = res["seatbid"][0]["bid"][0]
bid["ext"] = {"appnexus": {"bid_payment_type": [{"payment_type": 8}]}}; bid["burl"] = "https://notify.example/bill"
add("xandr-single-response-view-payment-non-usd", "xandr", "response", res,
    [("openrtb.profile.xandr.payment_currency", "cur")], valid=False)
req = request(); req["imp"][0] = {"id": "imp-1", "video": {"mimes": ["video/mp4"], "protocols": [2], "slotinpod": "unknown"}}
res = response(); res["seatbid"][0]["bid"][0]["slotinpod"] = 1
add("xandr-malformed-request-slot-defers", "xandr", "pair", res, request_payload=req)
req = request(); req["imp"][0]["ext"] = {"appnexus": {"allowed_payment_types": [{"payment_type": "unknown"}]}}
res = response(); bid = res["seatbid"][0]["bid"][0]
bid["ext"] = {"appnexus": {"bid_payment_type": [{"payment_type": 8}]}}; bid["burl"] = "https://notify.example/bill"
add("xandr-malformed-payment-offer-defers", "xandr", "pair", res, request_payload=req)

# Malformed supplied media objects leave the captured offer unknown. Optional
# absence remains distinct: selecting absent video in a valid banner offer fails.
for field, selected in [("banner", 1), ("video", 2), ("native", 3), ("audio", 4)]:
    for kind, value in [("boolean", False), ("string", "unknown"), ("array", []), ("null", None)]:
        req = request(); req["imp"][0][field] = value
        res = response(); res["seatbid"][0]["bid"][0]["ext"] = {"rp": {"response_format": selected}}
        add(f"magnite-malformed-{field}-{kind}-standalone", "magnite", "request", req, valid=False,
            source="https://github.com/InteractiveAdvertisingBureau/openrtb2.x/blob/main/2.6.md#32-object-imp",
            required=[("openrtb.type.mismatch", "imp[0]." + field)])
        add(f"magnite-malformed-{field}-{kind}-media-context-defers", "magnite", "pair", res, request_payload=req)
        if field != "banner":
            req["imp"][0]["banner"].update(api=[5], ext={"rp": {"mime": "text/html", "size_id": 1}})
            res["seatbid"][0]["bid"][0]["ext"] = {"rp": {"response_format": 1, "creativeapi": 7, "mime": "application/javascript", "size_id": 3}}
            add(f"magnite-malformed-competing-{field}-{kind}-banner-comparisons-defer", "magnite", "pair", res, request_payload=req)
req = request(); req["imp"][0]["video"] = False; req["imp"][0]["secure"] = 1
res = response(); res["seatbid"][0]["bid"][0]["ext"] = {"rp": {"response_format": 2, "imptrackers": ["http://tracker.example/i"]}}
add("magnite-malformed-media-retains-independent-secure-tracker", "magnite", "pair", res,
    [("openrtb.profile.magnite.insecure_tracker", "seatbid[0].bid[0].ext.rp.imptrackers[0]")],
    request_payload=req)
req = request(); req["imp"][0]["video"] = {"mimes": ["video/mp4"], "protocols": [2]}
res = response(); res["seatbid"][0]["bid"][0]["ext"] = {"rp": {"response_format": 2}}
paircase("magnite-explicit-video-selected-from-two-valid-media", "magnite", req, res)

assert len({case["name"] for case in CASES}) == len(CASES)
(HERE / "cases.jsonl").write_text("".join(json.dumps(case, ensure_ascii=False, separators=(",", ":")) + "\n" for case in CASES))
print(json.dumps({"cases": len(CASES), "magnite": sum(c["profile"] == "magnite" for c in CASES), "xandr": sum(c["profile"] == "xandr" for c in CASES)}, indent=2))
