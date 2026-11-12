#!/usr/bin/env python3
"""The VM speed suite: run every program in bench/perf/, check its output
against the recorded golden, and report the best wall time of N runs.

    python3 bench/perf.py                    # the built toolchain
    python3 bench/perf.py --mlang ./old      # any other build, to compare
    python3 bench/perf.py --check            # CI: outputs only, one run each
    python3 bench/perf.py --record           # re-record bench/perf/expected.json

A wrong answer fails the run before any time is printed — a faster VM
that computes something else is not faster.
"""
import argparse, json, pathlib, subprocess, sys, time

ROOT = pathlib.Path(__file__).resolve().parent.parent
PERF = ROOT / "bench" / "perf"
GOLDEN = PERF / "expected.json"


def run(mlang, prog):
    t = time.perf_counter()
    p = subprocess.run([mlang, "run", str(prog)], capture_output=True, timeout=600)
    return time.perf_counter() - t, p.returncode, p.stdout.decode("utf-8"), p.stderr.decode("utf-8")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--mlang", default=str(ROOT / "compiler" / "target" / "release" / "mlang"))
    ap.add_argument("--runs", type=int, default=5)
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--record", action="store_true")
    a = ap.parse_args()
    progs = sorted(PERF.glob("*.ml"))
    if a.record:
        GOLDEN.write_text(json.dumps({p.name: run(a.mlang, p)[2] for p in progs},
                                     ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
        print(f"recorded {len(progs)} goldens")
        return 0
    golden = json.loads(GOLDEN.read_text(encoding="utf-8"))
    total = 0.0
    for p in progs:
        best = None
        for _ in range(1 if a.check else a.runs):
            secs, code, out, err = run(a.mlang, p)
            if code != 0 or out != golden.get(p.name):
                print(f"✗ {p.name}: exit {code}, output differs from the golden\n{out}{err}", file=sys.stderr)
                return 1
            best = secs if best is None else min(best, secs)
        total += best
        print(f"{p.name:<14} {best * 1000:9.1f} ms")
    print(f"{'total':<14} {total * 1000:9.1f} ms")
    return 0


if __name__ == "__main__":
    sys.exit(main())
