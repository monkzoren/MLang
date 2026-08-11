"""The mutation-robustness sweep: what does a one-edit bug turn into?

For every exit-0 conformance program (MLang arm) and every verified Python
port (Python arm), seed many single-token mutations and bucket what each
one becomes. The dangerous bucket is `wrong-output`: the run completed
cleanly and produced the wrong bytes — nothing told the author anything
was wrong.

    python3 bench/robustness.py [--seeds-mlang N] [--seeds-python N]

Writes bench/results/robustness.json and bench/results/robustness.md.
"""

import argparse
import collections
import json
import os
from concurrent.futures import ThreadPoolExecutor

import common
import mutate

TIMEOUT = 3.0  # pristine worst case is 0.3s (mandelbrot); 10× headroom

BUCKETS = ["before-run", "runtime-fault", "deadlock-proven",
           "wrong-output", "hang", "pass"]
BUCKET_OF = {
    "weave": "before-run", "syntax": "before-run",
    "glitch": "runtime-fault", "exception": "runtime-fault",
    "deadlock": "deadlock-proven",
    "wrong-output": "wrong-output", "hang": "hang", "pass": "pass",
}


def sweep(arm, cases, seeds, jobs):
    run = common.run_mlang if arm == "mlang" else common.run_python
    classify = common.classify_mlang if arm == "mlang" else common.classify_python
    mutants = mutate.make_mutants(arm, cases, seeds)

    def label(m):
        r = run(m["source"], m["stdin"], timeout=TIMEOUT)
        return {"id": m["id"], "case": m["case"], "op": m["op"],
                "outcome": classify(r, m["expected"]),
                "stderr_seen": bool(r["stderr"])}

    with ThreadPoolExecutor(max_workers=jobs) as pool:
        return list(pool.map(label, mutants))


def summarize(labeled):
    total = len(labeled)
    buckets = collections.Counter(BUCKET_OF[m["outcome"]] for m in labeled)
    by_op = collections.defaultdict(collections.Counter)
    for m in labeled:
        by_op[m["op"]][BUCKET_OF[m["outcome"]]] += 1
    hang_with_traceback = sum(1 for m in labeled
                              if m["outcome"] == "hang" and m["stderr_seen"])
    return {"total": total,
            "buckets": {b: buckets.get(b, 0) for b in BUCKETS},
            "hang_with_traceback": hang_with_traceback,
            "by_op": {op: dict(c) for op, c in sorted(by_op.items())}}


ROW_LABELS = [
    ("before-run", "caught before running (load error)"),
    ("runtime-fault", "caught at runtime, precise report"),
    ("deadlock-proven", "deadlock — proven and reported"),
    ("wrong-output", "**silent wrong output**"),
    ("hang", "hang (killed at timeout)"),
    ("pass", "no behavior change (equivalent mutant)"),
]


def render(results):
    ml, py = results["mlang"], results["python"]

    def pct(s, b):
        return f"{100.0 * s['buckets'][b] / s['total']:.1f}%"

    lines = [
        "| One-token mutation becomes | MLang | Python |",
        "|---|---|---|",
    ]
    for bucket, label in ROW_LABELS:
        lines.append(f"| {label} | {pct(ml, bucket)} | {pct(py, bucket)} |")
    lines.append(f"\n{ml['total']} MLang mutants over "
                 f"{results['meta']['mlang_programs']} programs; "
                 f"{py['total']} Python mutants over "
                 f"{results['meta']['python_programs']} ports. "
                 f"Same four operator classes per arm "
                 f"(swap / drop / transpose / rename), one edit per mutant, "
                 f"strings and comments masked. "
                 f"{py['hang_with_traceback']} of {py['buckets']['hang']} "
                 f"Python hangs printed a thread traceback first — "
                 f"the process still never exited.")
    return "\n".join(lines) + "\n"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seeds-mlang", type=int, default=10)
    ap.add_argument("--seeds-python", type=int, default=32)
    ap.add_argument("--jobs", type=int, default=8)
    ap.add_argument("--cases", default=None,
                    help="comma-separated case names to target; give the "
                         "MLang name and the port name (e.g. "
                         "example:oracle.ml,oracle)")
    ap.add_argument("--tag", default=None,
                    help="suffix for the results files, e.g. oracle")
    args = ap.parse_args()
    cases_filter = set(args.cases.split(",")) if args.cases else None

    common.ensure_mlang()
    results = {"meta": {"seeds_mlang": args.seeds_mlang,
                        "seeds_python": args.seeds_python,
                        "cases": args.cases,
                        "timeout_s": TIMEOUT}}

    ml_cases = common.load_corpus()
    if cases_filter:
        ml_cases = [c for c in ml_cases if c["name"] in cases_filter]
    labeled_ml = sweep("mlang", ml_cases, args.seeds_mlang, args.jobs)
    results["meta"]["mlang_programs"] = len(ml_cases)
    results["mlang"] = summarize(labeled_ml)
    results["mlang"]["mutants"] = labeled_ml

    py_cases = common.load_python_corpus()
    if cases_filter:
        py_cases = [c for c in py_cases if c["name"] in cases_filter]
    labeled_py = sweep("python", py_cases, args.seeds_python, args.jobs)
    results["meta"]["python_programs"] = len(py_cases)
    results["python"] = summarize(labeled_py)
    results["python"]["mutants"] = labeled_py

    outdir = os.path.join(common.BENCH, "results")
    os.makedirs(outdir, exist_ok=True)
    stem = "robustness" + (f"-{args.tag}" if args.tag else "")
    with open(os.path.join(outdir, f"{stem}.json"), "w") as f:
        json.dump(results, f, indent=1)
        f.write("\n")
    table = render(results)
    with open(os.path.join(outdir, f"{stem}.md"), "w") as f:
        f.write(table)
    print(table)


if __name__ == "__main__":
    main()
