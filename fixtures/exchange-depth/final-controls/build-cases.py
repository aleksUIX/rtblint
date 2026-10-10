"""Author final Unity and Vungle boundaries from current primary vendor tables.

This builder does not read validator code or derive its oracle from execution.
"""
import copy
import json
from pathlib import Path

out = Path(__file__).resolve().parent
cases = []
unity_source = "https://docs.unity.com/en-us/grow/programmatic/unity-exchange/bid-responses"
vungle_source = "https://support.vungle.com/hc/en-us/articles/360045953431-Vungle-Exchange-OpenRTB-2-5-Integration-Guide"


def issue(identifier, path, severity="error"):
    return {"id": identifier, "path": path, "severity": severity}


def add(name, profile, mode, payload, expected=(), forbidden=(), valid=None, request=None):
    case = {"id": name, "profile": profile, "direction": mode,
            "version": "2.5" if profile == "vungle" else "2.6-202606",
            "input": copy.deepcopy(payload), "expected": list(expected), "forbidden": list(forbidden),
            "source": vungle_source if profile == "vungle" else unity_source}
    if request is not None:
        case["request"] = copy.deepcopy(request)
    if valid is not None:
        case["valid"] = valid
    cases.append(case)


def source_for_last_case(url):
    cases[-1]["source"] = url


def unity_request():
    return {"id": "unity-final-control", "at": 1, "app": {"id": "publisher"},
            "imp": [{"id": "1", "secure": 1, "instl": 0,
                     "banner": {"w": 320, "h": 50},
                     "pmp": {"deals": [{"id": "offer-a"}, {"id": "offer-b"}]}}]}


def unity_response():
    return {"id": "unity-final-control", "cur": "USD", "seatbid": [{"seat": "dsp", "bid": [
        {"id": "creative", "impid": "1", "price": 0.75, "adm": "<div>Creative</div>",
         "cat": ["IAB9"], "adomain": ["advertiser.example"], "burl": "https://dsp.example/bill",
         "w": 320, "h": 50, "ext": {"crtype": "HTML"}, "dealid": "offer-a"}]}]}


deal_error = issue("openrtb.profile.unity.deal_not_offered", "seatbid[0].bid[0].dealid")
for chosen in ["offer-a", "offer-b", "unoffered"]:
    req = unity_request(); res = unity_response(); res["seatbid"][0]["bid"][0]["dealid"] = chosen
    add("unity-deal-selection-" + chosen, "unity", "pair", res,
        expected=[] if chosen != "unoffered" else [deal_error], valid=chosen != "unoffered", request=req)
req = unity_request(); res = unity_response(); del res["seatbid"][0]["bid"][0]["dealid"]
add("unity-no-deal-selection-remains-optional", "unity", "pair", res, forbidden=[deal_error], valid=True, request=req)
for name, pmp in [("absent", None), ("deals-absent", {}), ("pmp-string", "unknown"),
                  ("deals-null", {"deals": None}), ("deals-string", {"deals": "unknown"}),
                  ("nonobject-entry", {"deals": [False]}), ("missing-id", {"deals": [{}]}),
                  ("numeric-id", {"deals": [{"id": 7}]}), ("empty-id", {"deals": [{"id": ""}]}),
                  ("mixed-valid-malformed", {"deals": [{"id": "offer-a"}, {"id": None}]}),
                  ("duplicate-ids", {"deals": [{"id": "offer-a"}, {"id": "offer-a"}]})]:
    req = unity_request(); res = unity_response(); res["seatbid"][0]["bid"][0]["dealid"] = "unoffered"
    if pmp is None:
        del req["imp"][0]["pmp"]
    else:
        req["imp"][0]["pmp"] = pmp
    add("unity-incomplete-deal-capture-" + name, "unity", "pair", res, forbidden=[deal_error], request=req)
req = unity_request(); req["imp"][0]["pmp"]["deals"] = []
add("unity-explicit-empty-deal-offer", "unity", "pair", unity_response(), expected=[deal_error], valid=False, request=req)
for name, change in [("missing-bid-impid", "missing"), ("unknown-bid-impid", "unknown"), ("duplicate-request-impid", "duplicate")]:
    req = unity_request(); res = unity_response(); bid = res["seatbid"][0]["bid"][0]; bid["dealid"] = "unoffered"
    if change == "missing":
        del bid["impid"]
    elif change == "unknown":
        bid["impid"] = "unknown"
    else:
        req["imp"].append(copy.deepcopy(req["imp"][0]))
    add("unity-deal-reference-defers-" + name, "unity", "pair", res, forbidden=[deal_error], request=req)

# Media-context requirements need well-formed containers. Response fields that
# explicitly identify a banner retain their independent dimension requirements.
dimensions = [issue("openrtb.profile.field_required", "seatbid[0].bid[0]." + field) for field in ["w", "h"]]
placement = issue("openrtb.profile.value_invalid", "imp[0].instl")
req = unity_request(); req["imp"][0]["instl"] = 1
add("unity-well-formed-banner-instl-boundary", "unity", "request", req, expected=[placement], valid=False)
req = unity_request(); res = unity_response(); bid = res["seatbid"][0]["bid"][0]; del bid["w"]; del bid["h"]
add("unity-well-formed-banner-context-requires-dimensions", "unity", "pair", res, expected=dimensions, valid=False, request=req)
for field in ["video", "audio", "native"]:
    for kind, value in [("boolean", False), ("string", "unknown"), ("array", []), ("null", None)]:
        req = unity_request(); req["imp"][0][field] = value; req["imp"][0]["instl"] = 1
        add("unity-malformed-" + field + "-" + kind + "-standalone", "unity", "request", req,
            expected=[issue("openrtb.type.mismatch", "imp[0]." + field)], forbidden=[placement], valid=False)
        add("unity-malformed-" + field + "-" + kind + "-context-dimensions-defer", "unity", "pair", res,
            forbidden=dimensions, request=req)
for kind, value in [("boolean", False), ("string", "unknown"), ("array", []), ("null", None)]:
    req = unity_request(); req["imp"][0]["banner"] = value
    req["imp"][0]["video"] = {"mimes": ["video/mp4"], "protocols": [2], "minduration": 5, "maxduration": 30, "pos": 7}
    add("unity-malformed-banner-" + kind + "-video-context-defers", "unity", "request", req,
        expected=[issue("openrtb.type.mismatch", "imp[0].banner")], forbidden=[placement], valid=False)
req = unity_request(); req["imp"][0]["video"] = False
explicit = copy.deepcopy(res); explicit["seatbid"][0]["bid"][0]["ext"]["crtype"] = "BANNER"
add("unity-explicit-response-banner-retains-dimensions", "unity", "pair", explicit, expected=dimensions, valid=False, request=req)
req = unity_request(); req["imp"][0]["instl"] = 1
req["imp"][0]["video"] = {"mimes": ["video/mp4"], "protocols": [2], "minduration": 5, "maxduration": 30, "pos": 7}
add("unity-two-well-formed-media-defer-banner-dimensions", "unity", "pair", res, forbidden=dimensions, valid=True, request=req)

# Existing attribution diagnostics need explicit source oracles too. Their
# ranges and string representations come from Unity's current attribution table.
unity_attribution = "https://docs.unity.com/en-us/grow/programmatic/unity-exchange/ios14-support"
def attributed_unity():
    res = unity_response(); bid = res["seatbid"][0]["bid"][0]
    bid["bundle"] = "123456789"
    bid["ext"].update(storeurl="https://apps.apple.com/app/id123456789", skadn={
        "version": "2.2", "network": "abcd123456.skadnetwork", "campaign": "1",
        "itunesitem": "123456789", "sourceapp": "987654321", "signature": "signature",
        "nonce": "00000000-0000-4000-8000-000000000001", "timestamp": "1700000000000"})
    return res

res = unity_response(); res["seatbid"][0]["bid"][0]["ext"]["appname"] = 7
add("unity-explicit-extension-string-type-oracle", "unity", "response", res,
    expected=[issue("openrtb.profile.unity.type_invalid", "seatbid[0].bid[0].ext.appname")], valid=False)
for value in [1, 100, 0, 101]:
    res = attributed_unity(); res["seatbid"][0]["bid"][0]["ext"]["skadn"]["campaign"] = str(value)
    good = value in [1, 100]
    add("unity-campaign-source-boundary-" + str(value), "unity", "response", res,
        expected=[] if good else [issue("openrtb.profile.unity.skadn_campaign", "seatbid[0].bid[0].ext.skadn.campaign")], valid=good)
    source_for_last_case(unity_attribution)
for value in ["0000", "123", "12a4"]:
    res = attributed_unity(); skadn = res["seatbid"][0]["bid"][0]["ext"]["skadn"]
    skadn["version"] = "4.0"; skadn["sourceidentifier"] = value; del skadn["campaign"]
    add("unity-sourceidentifier-source-boundary-" + value, "unity", "response", res,
        expected=[] if value == "0000" else [issue("openrtb.profile.unity.skadn_sourceidentifier", "seatbid[0].bid[0].ext.skadn.sourceidentifier")], valid=value == "0000")
    source_for_last_case(unity_attribution)
res = attributed_unity(); res["seatbid"][0]["bid"][0]["ext"]["skadn"]["timestamp"] = "later"
add("unity-timestamp-source-integer-string-oracle", "unity", "response", res,
    expected=[issue("openrtb.profile.unity.skadn_timestamp", "seatbid[0].bid[0].ext.skadn.timestamp")], valid=False)
source_for_last_case(unity_attribution)


def vungle_request():
    return {"id": "vungle-final-control", "at": 1, "tmax": 200, "cur": ["USD"],
            "source": {}, "app": {"id": "app", "cat": ["IAB9"], "publisher": {"id": "publisher"}},
            "device": {"ua": "Android SDK", "ip": "192.0.2.1", "w": 1080, "h": 1920, "connectiontype": 2},
            "imp": [{"id": "1", "displaymanager": "VungleDroid", "displaymanagerver": "7.4.1",
                     "tagid": "placement", "bidfloor": 0, "bidfloorcur": "USD", "secure": 1,
                     "clickbrowser": 0, "ext": {"pcta": 0},
                     "banner": {"id": "1", "w": 320, "h": 50, "format": [{"w": 320, "h": 50}], "mimes": ["text/html"]}}]}


def chain_request():
    req = vungle_request()
    req["source"]["ext"] = {"schain": {"complete": 1, "ver": "1.0", "nodes": [
        {"asi": "vungle.com", "sid": "seller", "rid": "upstream", "name": "Seller", "hp": 1}]}}
    return req


add("vungle-optional-regs-and-schain-absent", "vungle", "request", vungle_request(), valid=True)
add("vungle-full-schain-control", "vungle", "request", chain_request(), valid=True)
for name, chain in [("empty-object", {}), ("empty-nodes", {"nodes": []}), ("optional-node-members", {"nodes": [{}]})]:
    req = vungle_request(); req["source"]["ext"] = {"schain": chain}
    add("vungle-optional-schain-" + name, "vungle", "request", req, valid=True)
for field, invalid in [("schain", "unknown"), ("schain.ver", 1), ("schain.nodes", {}), ("schain.nodes", [False])]:
    req = chain_request(); obj = req["source"]["ext"]; parts = field.split(".")
    for part in parts[:-1]:
        obj = obj[part]
    obj[parts[-1]] = invalid
    add("vungle-invalid-" + field.replace(".", "-") + ("-entry" if isinstance(invalid, list) else ""), "vungle", "request", req,
        expected=[issue("openrtb.profile.vungle.type_invalid", "source.ext." + field)], valid=False)
for field in ["asi", "sid", "rid", "name"]:
    for kind, invalid in [("integer", 7), ("null", None)]:
        req = chain_request(); req["source"]["ext"]["schain"]["nodes"][0][field] = invalid
        add("vungle-invalid-schain-node-" + field + "-" + kind, "vungle", "request", req,
            expected=[issue("openrtb.profile.vungle.type_invalid", "source.ext.schain.nodes[0]." + field)], valid=False)
for field in ["complete", "hp"]:
    for value in [0, 1, 2, -1, "1", True, None]:
        req = chain_request(); obj = req["source"]["ext"]["schain"]
        path = "source.ext.schain." + field
        if field == "hp":
            obj = obj["nodes"][0]; path = "source.ext.schain.nodes[0].hp"
        obj[field] = value
        good = type(value) is int and value in [0, 1]
        add("vungle-schain-" + field + "-" + json.dumps(value), "vungle", "request", req,
            expected=[] if good else [issue("openrtb.profile.value_invalid", path)], valid=good)
for name, regs, expected, valid in [
    ("missing-ext", {}, [issue("openrtb.profile.field_required", "regs.ext")], False),
    ("empty-ext", {"ext": {}}, [], True),
    ("gdpr-zero", {"ext": {"gdpr": 0}}, [], True),
    ("gdpr-one", {"ext": {"gdpr": 1}}, [], True),
    ("null-ext", {"ext": None}, [issue("openrtb.profile.field_required", "regs.ext"), issue("openrtb.profile.vungle.type_invalid", "regs.ext")], False),
    ("scalar-ext", {"ext": "unknown"}, [issue("openrtb.profile.field_required", "regs.ext"), issue("openrtb.profile.vungle.type_invalid", "regs.ext")], False),
]:
    req = vungle_request(); req["regs"] = regs
    add("vungle-conditional-regs-" + name, "vungle", "request", req, expected=expected, valid=valid)
req = chain_request(); req["source"]["ext"]["schain"]["future"] = {"anything": [True, None]}
req["source"]["ext"]["schain"]["nodes"][0]["future"] = [False]
add("vungle-schain-unknown-members-remain-open", "vungle", "request", req, valid=True)

# These existing advisory and Native-version diagnostics were previously absent
# from the public reference corpus. Presence and severity remain source-derived.
vungle_native_source = "https://support.vungle.com/hc/en-us/articles/8582189840923-Vungle-Exchange-OpenRTB-2-5-Native-Ad-Integration"
res = {"id": "vungle-final-control", "cur": "EUR", "seatbid": [{"bid": [
    {"id": "creative", "impid": "1", "price": 1, "adm": "<div>Creative</div>", "adomain": ["advertiser.example"]}]}]}
add("vungle-non-usd-currency-source-advisory", "vungle", "response", res,
    expected=[issue("openrtb.profile.vungle.currency_ignored", "cur", "warning")], valid=True)
native = {"ver": "1.2", "assets": [{"id": 1, "img": {"url": "https://cdn.example/main.jpg", "w": 1200, "h": 627}}],
          "link": {"url": "https://advertiser.example"}, "jstracker": "<script>tracker()</script>"}
res = copy.deepcopy(res); res["cur"] = "USD"; res["seatbid"][0]["bid"][0]["adm"] = json.dumps({"native": native})
add("vungle-native-javascript-source-advisory", "vungle", "response", res,
    expected=[issue("openrtb.profile.vungle.native_unsupported", "seatbid[0].bid[0].adm.native.jstracker", "warning")], valid=True)
source_for_last_case(vungle_native_source)
for version in ["1.1", "1.2"]:
    req = vungle_request(); req["imp"][0].pop("banner")
    req["imp"][0]["native"] = {"ver": version, "request": json.dumps({"ver": "1.2", "assets": [{"id": 1, "required": 1, "img": {"type": 3, "w": 1200, "h": 627}}]})}
    add("vungle-native-request-version-source-" + version, "vungle", "request", req,
        expected=[] if version == "1.2" else [issue("openrtb.profile.vungle.native_version", "imp[0].native.ver")], valid=version == "1.2")
    source_for_last_case(vungle_source)

assert len({case["id"] for case in cases}) == len(cases)
(out / "cases.json").write_text(json.dumps({"approach": "Independently authored current primary vendor table and prose boundaries. Invalid or ambiguous pair captures defer contextual comparison. Optional child presence is not invented.", "cases": cases}, indent=2) + "\n")
print(len(cases), "final Unity and Vungle controls")
