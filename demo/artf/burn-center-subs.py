#!/usr/bin/env python3
# Burn movie-style center subtitles (white on black) onto artf-sim-five.mp4.
# Used because this ffmpeg build has no libass/subtitles filter.

from pathlib import Path
from subprocess import run
import sys

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent
VIDEO = ROOT / "artf-sim-five.mp4"
OUT = ROOT / "artf-sim-five-burn.mp4"
PNG_DIR = ROOT / "record-out" / "subs"
FONT = "/System/Library/Fonts/Supplemental/Arial.ttf"

CUES = [
    (2.0, 41.0, "0 · Publisher / CTV",
     "A mid-roll is about to fire. This is the OpenRTB the exchange is holding. ARTF lets an agent rewrite it."),
    (41.2, 80.0, "1 · SSP · looks legal, is not",
     "Wrong lifecycle, wrong ids, missing paths. One legacy write would have landed. We still drop the set."),
    (80.2, 111.0, "2 · Engineer · mutation fine, auction not",
     "ADD_METRICS value \"high\". Well-formed ARTF. After apply, OpenRTB is not."),
    (111.2, 154.0, "3 · DSP · curation lands",
     "New segments, new deal, floor $12 to $14.50, viewability 0.82. The DSP bids on the rewritten ticket."),
    (154.2, 185.0, "4 · Publisher · a deal leaves the wire",
     "SUPPRESS_DEALS pulls deal-standard. Curation is allowed to do that. The path still has to be this auction."),
    (185.2, 224.0, "5 · DSP · the bid is shaded",
     "Response-stage hop. $18.40 in, $12.88 out. Same dummy, new clearing price."),
    (224.2, 253.0, "Volume · same hop, many living rooms",
     "Synthetic mix at 200/s. 90% clean, 8% illegal, 2% breaking. Real gRPC. Not an exchange's QPS."),
    (253.2, 285.0, "Grafana · real rtblint-grpc",
     "p99 against a 150ms tmax. Drops are the dirty mix. Shed should sit at zero."),
    (285.2, 301.5, "The bid hop",
     "Envelope, mutation, applied. VASTlint was the creative hop. This is the bid hop. Independent. Not an IAB product."),
]

W, H = 1920, 1080
MAX_TEXT = 860
PAD_X, PAD_Y = 28, 18


def wrap(draw, text, font, max_w):
    words = text.split()
    lines, cur = [], ""
    for word in words:
        trial = word if not cur else f"{cur} {word}"
        if draw.textlength(trial, font=font) <= max_w:
            cur = trial
        else:
            if cur:
                lines.append(cur)
            cur = word
    if cur:
        lines.append(cur)
    return lines


def render(kicker, line, path):
    img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    font_k = ImageFont.truetype(FONT, 22)
    font_t = ImageFont.truetype(FONT, 36)
    kicker_lines = wrap(draw, kicker, font_k, MAX_TEXT)
    body_lines = wrap(draw, line, font_t, MAX_TEXT)
    gap = 10
    k_h = 26
    t_h = 48
    box_h = PAD_Y * 2 + len(kicker_lines) * k_h + gap + len(body_lines) * t_h
    widths = [draw.textlength(s, font=font_k) for s in kicker_lines] + [
        draw.textlength(s, font=font_t) for s in body_lines
    ]
    box_w = int(max(widths) + PAD_X * 2)
    x0 = (W - box_w) // 2
    y0 = (H - box_h) // 2
    draw.rectangle((x0, y0, x0 + box_w, y0 + box_h), fill=(0, 0, 0, 255))
    y = y0 + PAD_Y
    for s in kicker_lines:
        tw = draw.textlength(s, font=font_k)
        draw.text((x0 + (box_w - tw) / 2, y), s, font=font_k, fill=(255, 255, 255, 220))
        y += k_h
    y += gap
    for s in body_lines:
        tw = draw.textlength(s, font=font_t)
        draw.text((x0 + (box_w - tw) / 2, y), s, font=font_t, fill=(255, 255, 255, 255))
        y += t_h
    img.save(path)


def main():
    if not VIDEO.exists():
        sys.exit(f"missing {VIDEO}")
    PNG_DIR.mkdir(parents=True, exist_ok=True)
    pngs = []
    for i, (start, end, kicker, line) in enumerate(CUES):
        path = PNG_DIR / f"{i:02d}.png"
        render(kicker, line, path)
        pngs.append((start, end, path))

    inputs = ["-i", str(VIDEO)]
    for _, _, path in pngs:
        inputs += ["-i", str(path)]

    parts = [
        "[0:v]drawbox=x=0:y=ih-120:w=iw:h=120:color=black:t=fill[v0]"
    ]
    last = "v0"
    for i, (start, end, _) in enumerate(pngs, start=1):
        nxt = f"v{i}"
        parts.append(
            f"[{last}][{i}:v]overlay=0:0:enable='between(t,{start},{end})'[{nxt}]"
        )
        last = nxt
    fc = ";".join(parts)

    cmd = [
        "ffmpeg", "-y", *inputs,
        "-filter_complex", fc,
        "-map", f"[{last}]",
        "-c:v", "libx264", "-pix_fmt", "yuv420p", "-movflags", "+faststart",
        str(OUT),
    ]
    print(" ".join(cmd[:8]), "...")
    run(cmd, check=True)
    VIDEO.write_bytes(OUT.read_bytes())
    OUT.unlink()
    print(VIDEO)


if __name__ == "__main__":
    main()
