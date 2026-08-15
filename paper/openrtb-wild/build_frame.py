#!/usr/bin/env python3
"""Build a reproducible sampling frame from the Tranco top-1M ranking.

Two samples, two purposes:

  Sample A (prevalence): random draw, fixed seed, from Tranco top RANK_CAP
  after mechanical exclusions. Supports unbiased statements about how common
  client-side OpenRTB is among highly ranked sites.

  Sample B (depth): the purposive publisher list in sites.txt. Not
  representative by construction; used only for per-endpoint conformance
  rates, where many payloads per endpoint are required.

Exclusions are mechanical and documented so the frame reproduces exactly:
infrastructure, CDN, cloud, and ad-tech service domains cannot serve a
publisher ad auction, and non-content TLDs are dropped.
"""

import csv
import json
import random
from pathlib import Path

TRANCO = Path(
    "/private/tmp/claude-501/-Users-aleks-Documents-workspace-vast-master/"
    "90c2a9e0-6764-48db-b365-ad11fd45bc0f/scratchpad/top-1m.csv"
)
OUT = Path(__file__).parent
RANK_CAP = 50_000
SAMPLE_N = 1200
SEED = 20260725

# Mechanical exclusions: substrings that mark infrastructure, cloud, CDN,
# ad-tech, or API endpoints rather than ad-supported publisher content.
EXCLUDE_SUBSTRINGS = [
    "gtld-servers", "root-servers", "nstld", "akamai", "akadns", "edgekey", "edgesuite",
    "cloudflare", "cloudfront", "fastly", "amazonaws", "azure", "windows.net",
    "googleapis", "gstatic", "googleusercontent", "googlevideo", "ggpht", "googlesyndication",
    "doubleclick", "google-analytics", "googletagmanager", "googletagservices",
    "facebook.net", "fbcdn", "cdninstagram", "licdn", "twimg", "ytimg", "tiktokcdn",
    "cdn77", "cdn.", "jsdelivr", "unpkg", "bootstrapcdn", "cloudinary", "imgix",
    "adnxs", "rubiconproject", "pubmatic", "openx", "criteo", "casalemedia", "adsrvr",
    "taboola", "outbrain", "scorecardresearch", "quantserve", "moatads", "adsafeprotected",
    "sharethrough", "smartadserver", "media.net", "indexww", "33across", "sovrn",
    "onetrust", "cookielaw", "usercentrics", "sourcepoint", "quantcast",
    "sentry.io", "segment.com", "newrelic", "datadoghq", "mixpanel", "hotjar",
    "office365", "office.com", "microsoftonline", "sharepoint", "live.com",
    "apple-dns", "icloud", "mzstatic", "digicert", "letsencrypt", "verisign",
    "whatsapp", "wa.me", "t.me", "bit.ly", "goo.gl",
    "ntp.org", "pool.ntp", "in-addr", "ip6.arpa", "localhost",
]

# Non-content or unreachable-by-design TLDs.
EXCLUDE_TLDS = {"arpa", "int", "mil", "local", "internal", "test", "invalid", "onion"}


def excluded(domain: str) -> str | None:
    """Returns the exclusion reason, or None if the domain is kept."""
    d = domain.lower()
    tld = d.rsplit(".", 1)[-1]
    if tld in EXCLUDE_TLDS:
        return f"tld:{tld}"
    for s in EXCLUDE_SUBSTRINGS:
        if s in d:
            return f"substring:{s}"
    if d.count(".") == 0:
        return "no-tld"
    return None


def main():
    ranked = []
    with open(TRANCO) as f:
        for rank, domain in csv.reader(f):
            r = int(rank)
            if r > RANK_CAP:
                break
            ranked.append((r, domain))

    kept, dropped = [], {}
    for r, d in ranked:
        reason = excluded(d)
        if reason:
            dropped[reason] = dropped.get(reason, 0) + 1
        else:
            kept.append((r, d))

    rng = random.Random(SEED)
    sample = sorted(rng.sample(kept, SAMPLE_N))

    (OUT / "sites-tranco.txt").write_text(
        "# Sample A (prevalence): random draw from Tranco top "
        f"{RANK_CAP:,} after mechanical exclusions.\n"
        f"# seed={SEED} n={SAMPLE_N} frame_size={len(kept):,}\n"
        f"# Reproduce: python3 build_frame.py\n"
        + "\n".join(d for _, d in sample)
        + "\n"
    )

    frame = {
        "source": "Tranco top-1M",
        "rank_cap": RANK_CAP,
        "candidates_in_cap": len(ranked),
        "excluded_total": len(ranked) - len(kept),
        "excluded_by_reason": dict(sorted(dropped.items(), key=lambda kv: -kv[1])),
        "frame_size": len(kept),
        "sample_n": SAMPLE_N,
        "seed": SEED,
        "sample_rank_min": sample[0][0],
        "sample_rank_max": sample[-1][0],
        "sample": [{"rank": r, "domain": d} for r, d in sample],
    }
    json.dump(frame, open(OUT / "frame_tranco.json", "w"), indent=1)

    print(f"Tranco top {RANK_CAP:,}: {len(ranked):,} domains")
    print(f"excluded: {len(ranked)-len(kept):,}")
    for reason, n in sorted(dropped.items(), key=lambda kv: -kv[1])[:8]:
        print(f"    {n:6d}  {reason}")
    print(f"frame: {len(kept):,} domains")
    print(f"sample A: n={SAMPLE_N}, seed={SEED}, ranks {sample[0][0]}-{sample[-1][0]}")
    print("written: sites-tranco.txt, frame_tranco.json")


if __name__ == "__main__":
    main()
