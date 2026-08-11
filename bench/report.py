"""Render bench/results/*.json into the benchmark tables (markdown).

    python3 bench/report.py [--model-slug claude-haiku-4-5-20251001]

Writes bench/results/report.md and prints it.
"""

import argparse
import glob
import json
import os
import statistics

import common
from robustness import BUCKET_OF, render as render_robustness

CLASS_LABELS = [
    ("before-run", "caught before running"),
    ("runtime-fault", "runtime fault, precise report"),
    ("deadlock-proven", "proven deadlock"),
    ("wrong-output", "silent wrong output"),
    ("hang", "hang"),
]


def load_pair(slug):
    out = {}
    for arm in ("mlang", "python"):
        path = os.path.join(common.BENCH, "results", f"heal-{arm}-{slug}.json")
        if os.path.exists(path):
            out[arm] = json.load(open(path))
    return out


def heal_stats(data):
    recs = data["records"]
    healed = [r for r in recs if r["healed"]]
    rounds = [r["rounds"] for r in healed]
    return {
        "n": len(recs),
        "healed": len(healed),
        "pct": 100.0 * len(healed) / len(recs) if recs else 0.0,
        "r1": sum(1 for r in healed if r["rounds"] == 1),
        "r1pct": 100.0 * sum(1 for r in healed if r["rounds"] == 1) / len(recs)
                 if recs else 0.0,
        "median": statistics.median(rounds) if rounds else None,
    }


def by_class(data):
    out = {}
    for key, _ in CLASS_LABELS:
        sub = [r for r in data["records"] if BUCKET_OF[r["first_outcome"]] == key]
        if sub:
            n_healed = sum(1 for r in sub if r["healed"])
            out[key] = (n_healed, len(sub))
    return out


def render_heal(pair):
    ml, py = pair.get("mlang"), pair.get("python")
    meta = (ml or py)["meta"]
    model = meta["model"]
    rounds_max = meta["rounds_max"]

    s_ml = heal_stats(ml) if ml else None
    s_py = heal_stats(py) if py else None

    def row(label, fmt, bold=False):
        cells = []
        for s in (s_ml, s_py):
            cell = fmt(s) if s else "—"
            cells.append(f"**{cell}**" if bold else cell)
        return f"| {label} | {cells[0]} | {cells[1]} |"

    lines = [
        f"| Self-repair — {model}, ≤{rounds_max} rounds | MLang | Python |",
        "|---|---|---|",
        row("seeded one-edit bugs", lambda s: str(s["n"])),
        row("**healed (byte-exact output)**",
            lambda s: "{:.0f}%".format(s["pct"]), bold=True),
        row("healed in one round", lambda s: "{:.0f}%".format(s["r1pct"])),
        row("median rounds to green", lambda s: str(s["median"])),
    ]

    cls_ml = by_class(ml) if ml else {}
    cls_py = by_class(py) if py else {}
    lines += ["", "| healed, by what the bug turned into | MLang | Python |",
              "|---|---|---|"]
    for key, label in CLASS_LABELS:
        if key not in cls_ml and key not in cls_py:
            continue
        def cell(cls):
            if key not in cls:
                return "—"
            h, n = cls[key]
            return f"{h}/{n}"
        lines.append(f"| {label} | {cell(cls_ml)} | {cell(cls_py)} |")
    return "\n".join(lines) + "\n"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--model-slug", default=None)
    args = ap.parse_args()

    results_dir = os.path.join(common.BENCH, "results")
    slugs = set()
    if args.model_slug:
        slugs = {args.model_slug}
    else:
        for p in glob.glob(os.path.join(results_dir, "heal-*-*.json")):
            base = os.path.basename(p)[len("heal-"):-len(".json")]
            arm, slug = base.split("-", 1)
            slugs.add(slug)

    parts = []
    for slug in sorted(slugs):
        pair = load_pair(slug)
        if pair:
            parts.append(render_heal(pair))

    rob_path = os.path.join(results_dir, "robustness.json")
    if os.path.exists(rob_path):
        parts.append(render_robustness(json.load(open(rob_path))))

    report = "\n".join(parts)
    with open(os.path.join(results_dir, "report.md"), "w") as f:
        f.write(report)
    print(report)


if __name__ == "__main__":
    main()
