#!/usr/bin/env python3
"""Figures for the OpenRTB in-the-wild conformance preprint."""

from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Patch

FIGS = Path(__file__).parent / "figures"
FIGS.mkdir(exist_ok=True)

# validated two-series palette (all checks pass on the light surface)
BLUE, ORANGE = "#2a78d6", "#e8781a"
# ordinal ramp for the enforceability classes, with direct labels
C_A, C_B, C_MUTED = "#0d366b", "#3987e5", "#6f6d66"
INK, INK2, GRID = "#0b0b0b", "#52514e", "#e6e5e0"

plt.rcParams.update({
    "font.family": "Helvetica",
    "font.size": 9.5,
    "text.color": INK,
    "axes.edgecolor": INK2,
    "axes.labelcolor": INK2,
    "xtick.color": INK2,
    "ytick.color": INK2,
    "axes.linewidth": 0.6,
    "figure.facecolor": "white",
    "axes.facecolor": "white",
})


def save(fig, name):
    fig.savefig(FIGS / name, dpi=300, bbox_inches="tight")
    plt.close(fig)


# ---------------------------------------------------------------- figure 1
# The attribution split, with site-clustered confidence intervals.
fig, ax = plt.subplots(figsize=(7.2, 3.0))
groups = ["Bid requests\n(built client-side)", "Bid responses\n(built by SSP servers)"]
a_vals, a_lo, a_hi = [69.4, 21.4], [63.9, 16.2], [75.0, 27.3]
b_vals, b_lo, b_hi = [55.4, 14.9], [49.6, 12.3], [61.4, 17.8]
x = range(len(groups))
w = 0.34
for i, (vals, lo, hi, color, label) in enumerate([
    (a_vals, a_lo, a_hi, BLUE, "Sample A: random (Tranco)"),
    (b_vals, b_lo, b_hi, ORANGE, "Sample B: purposive publishers"),
]):
    pos = [xi + (i - 0.5) * w for xi in x]
    err = [[v - l for v, l in zip(vals, lo)], [h - v for v, h in zip(vals, hi)]]
    ax.bar(pos, vals, width=w, color=color, edgecolor="white", linewidth=1.5, label=label, zorder=3)
    ax.errorbar(pos, vals, yerr=err, fmt="none", ecolor=INK, elinewidth=1.3,
                capsize=4, capthick=1.3, zorder=4)
    for p, v, h in zip(pos, vals, hi):
        ax.text(p, h + 2.4, f"{v:.1f}%", ha="center", va="bottom", fontsize=9.5,
                fontweight="bold", color=color)
ax.set_xticks(list(x))
ax.set_xticklabels(groups, fontsize=9.5)
ax.set_ylim(0, 85)
ax.set_ylabel("payloads failing validation", fontsize=9)
ax.spines[["top", "right"]].set_visible(False)
ax.grid(axis="y", color=GRID, linewidth=0.6, zorder=0)
ax.set_axisbelow(True)
ax.legend(loc="upper right", frameon=False, fontsize=9)
save(fig, "fig1-attribution.png")

# ---------------------------------------------------------------- figure 2
# Where client-side defects originate.
fig, ax = plt.subplots(figsize=(7.2, 1.5))
# Random sample (A); Sample B differs and both are reported in the text.
segs = [
    ("SSP adapter code", 37.1, C_A, "white"),
    ("ecosystem-wide convention", 60.9, C_B, INK),
    ("publisher config", 2.0, C_MUTED, "white"),
]
left = 0.0
for label, share, color, textcolor in segs:
    ax.barh(0, share, left=left, height=0.5, color=color, edgecolor="white", linewidth=1.8)
    if share > 5:
        ax.text(left + share / 2, 0, f"{label}\n{share:.1f}%", ha="center", va="center",
                fontsize=9, color=textcolor)
    left += share
ax.annotate(f"publisher config  {segs[2][1]:.1f}%", xy=(99.0, 0.28), xytext=(88, 0.75),
            fontsize=8.8, color=INK2, ha="center",
            arrowprops=dict(arrowstyle="-", color=INK2, linewidth=0.7))
ax.set_xlim(0, 100)
ax.set_ylim(-0.45, 1.0)
ax.axis("off")
save(fig, "fig2-origin.png")

# ---------------------------------------------------------------- figure 3
# H2: the share of the spec each class covers vs the share of findings it carries.
fig, ax = plt.subplots(figsize=(7.2, 2.9))
labels = ["Class A\nplain JSON Schema", "Class B\nlint rules"]
spec_share = [14.5, 39.0]
found_b = [74.3, 25.7]
found_a = [64.6, 35.4]
x = range(len(labels))
w = 0.26
series = [
    (spec_share, C_MUTED, "share of the specification"),
    (found_b, ORANGE, "share of findings, Sample B"),
    (found_a, BLUE, "share of findings, Sample A"),
]
for i, (vals, color, label) in enumerate(series):
    pos = [xi + (i - 1) * w for xi in x]
    ax.bar(pos, vals, width=w, color=color, edgecolor="white", linewidth=1.4, label=label, zorder=3)
    for p, v in zip(pos, vals):
        ax.text(p, v + 1.6, f"{v:.1f}", ha="center", va="bottom", fontsize=8.6, color=color)
ax.set_xticks(list(x))
ax.set_xticklabels(labels, fontsize=9.5)
ax.set_ylim(0, 88)
ax.set_ylabel("percent", fontsize=9)
ax.spines[["top", "right"]].set_visible(False)
ax.grid(axis="y", color=GRID, linewidth=0.6, zorder=0)
ax.set_axisbelow(True)
ax.legend(loc="upper center", bbox_to_anchor=(0.5, -0.16), ncol=3, frameon=False, fontsize=8.8)
save(fig, "fig3-hypothesis.png")

print("figures written")
