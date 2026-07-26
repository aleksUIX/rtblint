#!/usr/bin/env python3
"""Figures for the OpenRTB machine-checkability preprint."""

import csv
import json
from datetime import datetime
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Patch

HERE = Path(__file__).parent
DATA = HERE / "data"
FIGS = HERE / "figures"
FIGS.mkdir(exist_ok=True)

# validated ordinal ramp (blue light->dark) + neutral gray for "not decidable"
C_A, C_B, C_C, C_D = "#0d366b", "#3987e5", "#b7d3f6", "#6f6d66"
SERIES_1, SERIES_2 = "#2a78d6", "#e8781a"
INK, INK2, GRID = "#0b0b0b", "#52514e", "#e6e5e0"

plt.rcParams.update(
    {
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
        "svg.fonttype": "none",
    }
)

stats = json.load(open(DATA / "final_stats.json"))

CLASS_COLORS = [C_A, C_B, C_C, C_D]
CLASS_LABELS = [
    "A · JSON Schema",
    "B · stateless lint",
    "C · runtime / cross-message",
    "D · not machine-decidable",
]


def stacked_pct(ax, rows, row_labels, ns):
    """100% stacked horizontal bars with white gaps and inline % labels."""
    for yi, (shares, n) in enumerate(zip(rows, ns)):
        left = 0.0
        for share, color in zip(shares, CLASS_COLORS):
            ax.barh(
                yi, share, left=left, height=0.52, color=color,
                edgecolor="white", linewidth=1.6,
            )
            if share >= 6:
                lum_dark = color in (C_A, C_D)
                ax.text(
                    left + share / 2, yi, f"{share:.0f}%",
                    ha="center", va="center", fontsize=9,
                    color="white" if lum_dark else INK,
                )
            left += share
    ax.set_yticks(range(len(row_labels)))
    ax.set_yticklabels(
        [f"{lab}\n(n = {n})" for lab, n in zip(row_labels, ns)], fontsize=9.5
    )
    ax.set_xlim(0, 100)
    ax.invert_yaxis()
    ax.spines[["top", "right", "left"]].set_visible(False)
    ax.xaxis.set_visible(False)
    ax.tick_params(left=False)


# ---------------------------------------------------------------- figure 1
fig, ax = plt.subplots(figsize=(7.2, 2.3))
rows = []
ns = []
for spec in ("2.6-202606", "3.0"):
    s = stats[spec]
    ns.append(s["conformance_statements"])
    rows.append([s["by_class_pct"][c] for c in "ABCD"])
stacked_pct(ax, rows, ["OpenRTB 2.6\n(202606)", "OpenRTB 3.0"], ns)
ax.legend(
    handles=[Patch(facecolor=c, label=l) for c, l in zip(CLASS_COLORS, CLASS_LABELS)],
    loc="upper center", bbox_to_anchor=(0.5, -0.06), ncol=2, frameon=False,
    fontsize=8.8, handlelength=1.2, handleheight=1.0,
)
fig.tight_layout()
fig.savefig(FIGS / "fig1-classes.png", dpi=300, bbox_inches="tight")
plt.close(fig)

# ---------------------------------------------------------------- figure 2
cat = list(csv.DictReader(open(DATA / "catalog_versions.csv")))
cat26 = [r for r in cat if r["version"].startswith("2.") and int(r["fields"]) > 0]
dates = [datetime.strptime(r["release_date"], "%Y-%m") for r in cat26]
fields = [int(r["fields"]) for r in cat26]
required = [int(r["required"]) for r in cat26]

fig, ax = plt.subplots(figsize=(7.2, 3.1))
ax.plot(dates, fields, color=SERIES_1, linewidth=2, marker="o", markersize=4.5,
        markerfacecolor="white", markeredgewidth=1.4, clip_on=False)
ax.plot(dates, required, color=SERIES_2, linewidth=2, marker="o", markersize=4.5,
        markerfacecolor="white", markeredgewidth=1.4, clip_on=False)
ax.text(dates[-1], fields[-1] + 14, f"total fields  {fields[-1]}",
        ha="right", va="bottom", color=SERIES_1, fontsize=9.5, fontweight="bold")
ax.text(dates[-1], required[-1] + 14, f"required fields  {required[-1]}",
        ha="right", va="bottom", color=SERIES_2, fontsize=9.5, fontweight="bold")
ax.text(dates[0], fields[0] + 14, str(fields[0]), ha="center", va="bottom",
        color=SERIES_1, fontsize=9)
ax.text(dates[0], required[0] + 14, str(required[0]), ha="center", va="bottom",
        color=SERIES_2, fontsize=9)
ax.set_ylim(0, 460)
ax.spines[["top", "right"]].set_visible(False)
ax.grid(axis="y", color=GRID, linewidth=0.6)
ax.set_axisbelow(True)
ax.set_ylabel("field definitions in object catalog", fontsize=9)
fig.tight_layout()
fig.savefig(FIGS / "fig2-growth.png", dpi=300, bbox_inches="tight")
plt.close(fig)

# ---------------------------------------------------------------- figure 3
s = stats["2.6-202606"]["class_by_obligation"]
fig, ax = plt.subplots(figsize=(7.2, 2.9))
rows, ns = [], []
for ob in ("obligation", "recommendation", "permission"):
    counts = [s[ob][c] for c in "ABCD"]
    n = sum(counts)
    ns.append(n)
    rows.append([100 * v / n for v in counts])
stacked_pct(
    ax, rows,
    ['"must" statements', '"should" statements', '"may" statements'], ns,
)
ax.legend(
    handles=[Patch(facecolor=c, label=l) for c, l in zip(CLASS_COLORS, CLASS_LABELS)],
    loc="upper center", bbox_to_anchor=(0.5, -0.06), ncol=2, frameon=False,
    fontsize=8.8, handlelength=1.2, handleheight=1.0,
)
fig.tight_layout()
fig.savefig(FIGS / "fig3-obligation.png", dpi=300, bbox_inches="tight")
plt.close(fig)

print("figures written")
