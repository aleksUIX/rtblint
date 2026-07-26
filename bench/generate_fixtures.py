#!/usr/bin/env python3
"""Deterministic OpenRTB fixture generator for the RTBlint benchmark.

Writes 50 fixtures into bench/fixtures/, ranging from a few hundred bytes to
several megabytes, plus a manifest.json describing each one (payload kind,
expected validity, size). Output is fully deterministic: the same script
revision always produces byte-identical fixtures.

Usage:
    python3 bench/generate_fixtures.py [--out DIR]
"""

import argparse
import json
import random
from pathlib import Path

VAST_SNIPPET = (
    "<VAST version='4.2'><Ad id='cr-{cid}'><Wrapper><AdSystem>bench-dsp</AdSystem>"
    "<VASTAdTagURI><![CDATA[https://dsp.example.com/vast/cr-{cid}]]></VASTAdTagURI>"
    "<Impression><![CDATA[https://dsp.example.com/imp/{cid}]]></Impression>"
    "</Wrapper></Ad></VAST>"
)

WORDS = (
    "alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima "
    "mike november oscar papa quebec romeo sierra tango uniform victor whiskey "
    "xray yankee zulu amber birch cedar dune ember flint grove harbor inlet"
).split()


def rng_for(name: str) -> random.Random:
    return random.Random(f"rtblint-bench::{name}")


def words(rng: random.Random, n: int) -> str:
    return " ".join(rng.choice(WORDS) for _ in range(n))


# ── payload building blocks ──────────────────────────────────────────────


def video(rng: random.Random, full: bool = False) -> dict:
    out = {
        "mimes": ["video/mp4"],
        "protocols": [3, 7, 8],
        "w": 1920,
        "h": 1080,
        "linearity": 1,
        "plcmt": 1,
        "minduration": 5,
        "maxduration": 60,
    }
    if full:
        out.update(
            {
                "pos": 7,
                "podid": f"pod-{rng.randint(1, 9)}",
                "podseq": 0,
                "maxseq": 4,
                "poddur": 120,
                "slotinpod": 1,
                "startdelay": 0,
                "playbackmethod": [1],
                "playbackend": 1,
                "delivery": [2],
                "api": [7],
            }
        )
    return out


def banner(rng: random.Random, formats: int = 2) -> dict:
    sizes = [(320, 50), (300, 250), (728, 90), (160, 600), (970, 250), (300, 600)]
    return {
        "format": [{"w": w, "h": h} for w, h in sizes[:formats]],
        "w": sizes[0][0],
        "h": sizes[0][1],
        "pos": 1,
        "btype": [4],
        "battr": [1, 3],
        "api": [3, 5],
    }


def audio(rng: random.Random) -> dict:
    return {
        "mimes": ["audio/mp4", "audio/mpeg"],
        "minduration": 10,
        "maxduration": 30,
        "protocols": [9, 10],
        "feed": 3,
        "nvol": 2,
    }


def native(rng: random.Random) -> dict:
    request = {
        "ver": "1.2",
        "assets": [
            {"id": 1, "required": 1, "title": {"len": 90}},
            {"id": 2, "required": 1, "img": {"type": 3, "w": 1200, "h": 627}},
            {"id": 3, "data": {"type": 2, "len": 140}},
        ],
    }
    return {"request": json.dumps(request), "ver": "1.2", "api": [3]}


def metric(rng: random.Random, i: int) -> dict:
    kinds = ["viewability", "ctr", "completion_rate", "session_depth"]
    return {
        "type": kinds[i % len(kinds)],
        "value": round(rng.random(), 4),
        "vendor": "bench-vendor.example",
    }


def deal(rng: random.Random, i: int) -> dict:
    return {
        "id": f"deal-{i:06d}",
        "bidfloor": round(1.0 + rng.random() * 20, 2),
        "bidfloorcur": "USD",
        "at": 1,
        "wadomain": [f"brand-{i % 37}.example"],
    }


def imp(rng: random.Random, i: int, media: str = "video", full: bool = False,
        metrics: int = 0, deals: int = 0) -> dict:
    out = {
        "id": str(i + 1),
        "secure": 1,
        "bidfloor": round(0.5 + rng.random() * 10, 2),
        "bidfloorcur": "USD",
    }
    if media == "video":
        out["video"] = video(rng, full=full)
    elif media == "banner":
        out["banner"] = banner(rng)
    elif media == "audio":
        out["audio"] = audio(rng)
    elif media == "native":
        out["native"] = native(rng)
    elif media == "multi":
        out["banner"] = banner(rng)
        out["video"] = video(rng, full=full)
        out["native"] = native(rng)
    if metrics:
        out["metric"] = [metric(rng, m) for m in range(metrics)]
    if deals:
        out["pmp"] = {"private_auction": 0, "deals": [deal(rng, d) for d in range(deals)]}
    return out


def device(rng: random.Random, kind: str = "ctv") -> dict:
    if kind == "ctv":
        return {
            "ua": "Mozilla/5.0 (SMART-TV; Linux; Tizen 7.0)",
            "ip": "203.0.113.42",
            "devicetype": 3,
            "make": "Samsung",
            "model": "QN90C",
            "os": "Tizen",
            "osv": "7.0",
            "ifa": "38c1a2f0-52b6-4c9e-9a41-d0f83a6c1e11",
            "lmt": 0,
            "connectiontype": 2,
            "geo": {"country": "USA", "region": "CA", "type": 2},
        }
    return {
        "ua": "Mozilla/5.0 (Linux; Android 14; Pixel 8)",
        "ip": "198.51.100.7",
        "devicetype": 4,
        "make": "Google",
        "model": "Pixel 8",
        "os": "Android",
        "osv": "14",
        "language": "en",
        "js": 1,
        "connectiontype": 6,
        "ifa": "5c9b2a44-9d1e-4f6a-8a3d-72cb0e9a1f02",
        "sua": {
            "browsers": [{"brand": "Chromium", "version": ["124"]}],
            "platform": {"brand": "Android", "version": ["14"]},
            "mobile": 1,
            "source": 2,
        },
    }


def app(rng: random.Random, with_content: bool = True) -> dict:
    out = {
        "id": "app-552",
        "name": "StreamBox",
        "bundle": "com.streambox.tv",
        "storeurl": "https://apps.example.com/streambox",
        "publisher": {"id": "pub-4491", "name": "StreamBox Media"},
    }
    if with_content:
        out["content"] = {
            "id": "ep-2201",
            "title": "Chef's Table",
            "series": "Kitchen Stories",
            "season": "Season 4",
            "episode": 12,
            "genre": "documentary",
            "livestream": 0,
            "len": 2700,
            "language": "en",
            "network": {"id": "net-7", "name": "StreamBox Originals"},
            "channel": {"id": "ch-12", "name": "Food"},
        }
    return out


def site(rng: random.Random) -> dict:
    return {
        "id": "site-9",
        "domain": "news.example",
        "page": "https://news.example/story/12345",
        "cat": ["IAB12"],
        "cattax": 1,
        "publisher": {"id": "pub-88", "name": "Daily News Group"},
    }


def schain() -> dict:
    return {
        "schain": {
            "complete": 1,
            "ver": "1.0",
            "nodes": [{"asi": "exchange.example.com", "sid": "pub-4491", "hp": 1}],
        }
    }


def eid(rng: random.Random, i: int, uids: int = 2) -> dict:
    return {
        "source": f"idprovider-{i % 97}.example",
        "mm": 1,
        "uids": [
            {"id": f"uid-{i:06d}-{u}", "atype": 1 + (u % 3)} for u in range(uids)
        ],
    }


def data_block(rng: random.Random, i: int, segments: int) -> dict:
    return {
        "id": f"data-{i:04d}",
        "name": f"provider-{i % 23}.example",
        "segment": [
            {"id": f"seg-{i:04d}-{s:05d}", "name": words(rng, 2), "value": str(rng.randint(0, 99))}
            for s in range(segments)
        ],
    }


def request(rng: random.Random, imps: list, context: str = "app", **extra) -> dict:
    out = {
        "id": f"req-{rng.getrandbits(48):012x}",
        "at": 1,
        "tmax": 300,
        "cur": ["USD"],
        "source": schain(),
        "regs": {"coppa": 0, "gdpr": 0},
        "imp": imps,
    }
    if context == "app":
        out["app"] = app(rng)
    elif context == "site":
        out["site"] = site(rng)
    elif context == "dooh":
        out["dooh"] = {
            "id": "screen-88",
            "name": "Airport Arrivals Billboard",
            "venuetype": "transit.airports",
            "venuetypetax": 1,
            "publisher": {"id": "pub-oh-4", "name": "CityScreens"},
        }
    out["device"] = device(rng, "ctv" if context in ("app", "dooh") else "mobile")
    out.update(extra)
    return out


def bid(rng: random.Random, i: int, adm_bytes: int = 0) -> dict:
    out = {
        "id": f"bid-{i:06d}",
        "impid": str(i + 1),
        "price": round(0.5 + rng.random() * 30, 4),
        "adid": f"cr-{i:06d}",
        "nurl": "https://dsp.example.com/win?p=${AUCTION_PRICE}",
        "adomain": [f"brand-{i % 53}.example"],
        "crid": f"cr-{i:06d}",
        "cid": f"camp-{i % 11}",
        "cat": ["IAB8"],
        "cattax": 1,
        "w": 1920,
        "h": 1080,
        "dur": 30,
        "mtype": 2,
        "apis": [7],
        "protocol": 8,
    }
    if adm_bytes:
        base = VAST_SNIPPET.format(cid=f"{i:06d}")
        if adm_bytes > len(base):
            # Pad inside an XML comment so the markup stays plausible.
            pad = words(rng, max(1, (adm_bytes - len(base)) // 6))[: adm_bytes - len(base) - 9]
            base = base.replace("</Wrapper>", f"<!-- {pad} --></Wrapper>")
        out["adm"] = base
    return out


def response(rng: random.Random, seats: int, bids_per_seat: int, adm_bytes: int = 0) -> dict:
    return {
        "id": f"req-{rng.getrandbits(48):012x}",
        "bidid": f"resp-{rng.getrandbits(32):08x}",
        "cur": "USD",
        "seatbid": [
            {
                "seat": f"dsp-{s:03d}",
                "group": 0,
                "bid": [
                    bid(rng, s * bids_per_seat + b, adm_bytes)
                    for b in range(bids_per_seat)
                ],
            }
            for s in range(seats)
        ],
    }


def ext_blob(rng: random.Random, keys: int, depth: int) -> dict:
    def level(d: int, width: int) -> dict:
        if d == 0:
            return {f"x{k:04d}": rng.randint(0, 9999) for k in range(width)}
        return {f"xn{k:03d}": level(d - 1, width) for k in range(3)}

    per_leaf = max(1, keys // (3 ** depth))
    return level(depth, per_leaf)


# ── fixture definitions ──────────────────────────────────────────────────


def build_fixtures() -> list:
    fixtures = []

    def add(name, kind, expected_valid, payload):
        fixtures.append(
            {"name": name, "kind": kind, "expected_valid": expected_valid, "payload": payload}
        )

    # tiny (6)
    r = rng_for("tiny-video")
    add("tiny-video-request", "request",
        True, {"id": "req-1", "imp": [{"id": "1", "video": {"mimes": ["video/mp4"]}}]})
    add("tiny-banner-request", "request",
        True, {"id": "req-2", "imp": [{"id": "1", "banner": {"w": 300, "h": 250}}]})
    add("tiny-audio-request", "request",
        True, {"id": "req-3", "imp": [{"id": "1", "audio": {"mimes": ["audio/mp4"]}}]})
    r = rng_for("tiny-native")
    add("tiny-native-request", "request",
        True, {"id": "req-4", "imp": [{"id": "1", "native": native(r)}]})
    add("tiny-nobid-response", "response", True, {"id": "req-5", "nbr": 8})
    r = rng_for("tiny-dooh")
    add("tiny-dooh-request", "request",
        True, request(r, [imp(r, 0, "banner")], context="dooh"))

    # small, roughly 1-6 KB (8)
    r = rng_for("small-ctv")
    add("small-ctv-pod-request", "request",
        True, request(r, [imp(r, 0, "video", full=True, deals=2)], context="app"))
    r = rng_for("small-mobile")
    add("small-mobile-banner-request", "request",
        True, request(r, [imp(r, 0, "banner")], context="site",
                      bcat=["IAB25", "IAB26"], cattax=1, badv=["competitor.example"]))
    r = rng_for("small-webvideo")
    add("small-web-video-request", "request",
        True, request(r, [imp(r, 0, "video")], context="site"))
    r = rng_for("small-audio")
    add("small-audio-podcast-request", "request",
        True, request(r, [imp(r, 0, "audio")], context="app"))
    r = rng_for("small-multi")
    add("small-multiformat-request", "request",
        True, request(r, [imp(r, 0, "multi")], context="app"))
    r = rng_for("small-resp1")
    add("small-video-win-response", "response", True, response(r, 1, 1, adm_bytes=400))
    r = rng_for("small-resp2")
    add("small-multiseat-response", "response", True, response(r, 3, 2, adm_bytes=300))
    r = rng_for("small-invalid")
    bad = request(r, [imp(r, 0, "video")], context="site")
    bad["imp"][0]["video"]["placement"] = 1
    bad["imp"][0]["video"]["plcmt"] = 9
    bad["regs"]["ext"] = {"gdpr": 1}
    bad["zz_unknown"] = True
    add("small-invalid-mixed-request", "request", False, bad)

    # medium, roughly 10-60 KB (12)
    r = rng_for("med-imps10")
    add("medium-imps-10-video-request", "request",
        True, request(r, [imp(r, i, "video", full=True) for i in range(10)]))
    r = rng_for("med-imps25")
    add("medium-imps-25-banner-request", "request",
        True, request(r, [imp(r, i, "banner") for i in range(25)]))
    r = rng_for("med-eids")
    add("medium-eids-50-request", "request",
        True, request(r, [imp(r, 0, "video")],
                      user={"id": "u-1", "eids": [eid(r, i) for i in range(50)]}))
    r = rng_for("med-segments")
    add("medium-segments-200-request", "request",
        True, request(r, [imp(r, 0, "video")],
                      user={"id": "u-2", "data": [data_block(r, i, 20) for i in range(10)]}))
    r = rng_for("med-content")
    med_content = request(r, [imp(r, 0, "video")])
    med_content["app"]["content"]["data"] = [data_block(r, i, 15) for i in range(8)]
    med_content["app"]["content"]["keywords"] = ",".join(words(r, 60).split())
    add("medium-content-data-request", "request", True, med_content)
    r = rng_for("med-deals")
    add("medium-deals-100-request", "request",
        True, request(r, [imp(r, 0, "video", deals=100)]))
    r = rng_for("med-badv")
    add("medium-badv-1000-request", "request",
        True, request(r, [imp(r, 0, "banner")],
                      badv=[f"blocked-{i:05d}.example" for i in range(1000)]))
    r = rng_for("med-multiimps")
    add("medium-multiformat-imps-10-request", "request",
        True, request(r, [imp(r, i, "multi") for i in range(10)]))
    r = rng_for("med-resp50")
    add("medium-bids-50-response", "response", True, response(r, 5, 10, adm_bytes=250))
    r = rng_for("med-resp100")
    add("large-bids-100-adm-response", "response", True, response(r, 10, 10, adm_bytes=800))
    r = rng_for("med-invalid")
    bad = request(r, [imp(r, i, "video") for i in range(10)])
    for i, one_imp in enumerate(bad["imp"]):
        for u in range(10):
            one_imp[f"zz_unknown_{i}_{u}"] = u
    add("medium-invalid-unknown-100-request", "request", False, bad)
    r = rng_for("med-ext")
    add("medium-ext-deep-request", "request",
        True, request(r, [imp(r, 0, "video")], ext=ext_blob(r, 1500, 3)))

    # large, roughly 100-500 KB (12)
    r = rng_for("lg-imps100")
    add("large-imps-100-video-request", "request",
        True, request(r, [imp(r, i, "video", full=True) for i in range(100)]))
    r = rng_for("lg-imps250")
    add("large-imps-250-mixed-request", "request",
        True, request(r, [imp(r, i, ["video", "banner", "audio", "native"][i % 4])
                          for i in range(250)]))
    r = rng_for("lg-eids500")
    add("large-eids-500-request", "request",
        True, request(r, [imp(r, 0, "video")],
                      user={"id": "u-3", "eids": [eid(r, i) for i in range(500)]}))
    r = rng_for("lg-segments")
    add("large-segments-5000-request", "request",
        True, request(r, [imp(r, 0, "video")],
                      user={"id": "u-4", "data": [data_block(r, i, 100) for i in range(50)]}))
    r = rng_for("lg-deals")
    add("large-deals-1000-request", "request",
        True, request(r, [imp(r, 0, "video", deals=1000)]))
    r = rng_for("lg-badv")
    add("large-badv-10000-request", "request",
        True, request(r, [imp(r, 0, "banner")],
                      badv=[f"blocked-{i:06d}.example" for i in range(10000)]))
    r = rng_for("lg-full")
    add("large-imps-100-full-request", "request",
        True, request(r, [imp(r, i, "multi", full=True, metrics=5, deals=5)
                          for i in range(100)]))
    r = rng_for("lg-resp500")
    add("large-bids-500-adm-response", "response", True, response(r, 10, 50, adm_bytes=500))
    r = rng_for("lg-seats")
    add("large-seats-50x20-response", "response", True, response(r, 50, 20, adm_bytes=200))
    r = rng_for("lg-metrics")
    add("large-metrics-request", "request",
        True, request(r, [imp(r, i, "video", metrics=50) for i in range(20)]))
    r = rng_for("lg-locales")
    add("large-arrays-request", "request",
        True, request(r, [imp(r, 0, "video")],
                      wlang=["en", "de", "fr", "es", "pt", "it", "nl", "pl", "sv", "da"] * 30,
                      bcat=[f"IAB{1 + (i % 26)}" for i in range(2000)], cattax=1,
                      badv=[f"blocked-{i:06d}.example" for i in range(4000)]))
    r = rng_for("lg-invalid")
    bad = request(r, [imp(r, i, "video") for i in range(100)])
    for one_imp in bad["imp"]:
        one_imp["video"]["plcmt"] = 99
        one_imp["video"]["api"] = [99]
        one_imp["video"]["placement"] = 1
        one_imp["zz_unknown"] = 1
    add("large-invalid-enums-request", "request", False, bad)

    # xlarge, roughly 1-6 MB (12)
    r = rng_for("xl-imps1000")
    add("xlarge-imps-1000-video-request", "request",
        True, request(r, [imp(r, i, "video", full=True) for i in range(1000)]))
    r = rng_for("xl-imps2500")
    add("xlarge-imps-2500-mixed-request", "request",
        True, request(r, [imp(r, i, ["video", "banner", "audio", "native"][i % 4])
                          for i in range(2500)]))
    r = rng_for("xl-eids5000")
    add("xlarge-eids-5000-request", "request",
        True, request(r, [imp(r, 0, "video")],
                      user={"id": "u-5", "eids": [eid(r, i) for i in range(5000)]}))
    r = rng_for("xl-segments")
    add("xlarge-segments-50000-request", "request",
        True, request(r, [imp(r, 0, "video")],
                      user={"id": "u-6", "data": [data_block(r, i, 500) for i in range(100)]}))
    r = rng_for("xl-resp2000")
    add("xlarge-bids-2000-adm-response", "response", True, response(r, 20, 100, adm_bytes=900))
    r = rng_for("xl-adm4mb")
    add("xlarge-adm-4mb-response", "response", True, response(r, 1, 1, adm_bytes=4_000_000))
    r = rng_for("xl-deals")
    add("xlarge-deals-10000-request", "request",
        True, request(r, [imp(r, 0, "video", deals=10000)]))
    r = rng_for("xl-badv")
    add("xlarge-badv-100000-request", "request",
        True, request(r, [imp(r, 0, "banner")],
                      badv=[f"blocked-{i:06d}.example" for i in range(100000)]))
    r = rng_for("xl-full")
    add("xlarge-imps-500-full-request", "request",
        True, request(r, [imp(r, i, "multi", full=True, metrics=10, deals=10)
                          for i in range(500)]))
    r = rng_for("xl-ext")
    add("xlarge-ext-blob-request", "request",
        True, request(r, [imp(r, 0, "video")], ext=ext_blob(r, 40000, 6)))
    r = rng_for("xl-invalid")
    bad = request(r, [imp(r, i, "video") for i in range(5000)])
    for one_imp in bad["imp"]:
        one_imp["video"]["plcmt"] = 99
        one_imp["zz_unknown_a"] = 1
        one_imp["zz_unknown_b"] = 2
    add("xlarge-invalid-many-issues-request", "request", False, bad)
    r = rng_for("xl-mixedresp")
    add("xlarge-seats-100x50-response", "response", True, response(r, 100, 50, adm_bytes=300))

    return fixtures


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", default=str(Path(__file__).parent / "fixtures"))
    args = parser.parse_args()

    out_dir = Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)

    fixtures = build_fixtures()
    assert len(fixtures) == 50, f"expected 50 fixtures, defined {len(fixtures)}"

    manifest = []
    for fixture in fixtures:
        raw = json.dumps(fixture["payload"], separators=(",", ":"))
        file_name = f"{fixture['name']}.json"
        (out_dir / file_name).write_text(raw)
        manifest.append(
            {
                "name": fixture["name"],
                "file": file_name,
                "kind": fixture["kind"],
                "expected_valid": fixture["expected_valid"],
                "bytes": len(raw.encode()),
            }
        )

    manifest.sort(key=lambda entry: entry["bytes"])
    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")

    total = sum(entry["bytes"] for entry in manifest)
    print(f"wrote {len(manifest)} fixtures to {out_dir} "
          f"({manifest[0]['bytes']} B smallest, {manifest[-1]['bytes']:,} B largest, "
          f"{total / 1024 / 1024:.1f} MB total)")


if __name__ == "__main__":
    main()
