#!/usr/bin/env python3
"""Verify an MLang engine against the recorded conformance corpus.

Usage:
  python3 conformance/run.py python3 -m mlang          # the reference itself
  python3 conformance/run.py rust/target/release/mlang # the native engine

The engine command must support `<cmd> run <file>`. Every case is compared
byte-for-byte on stdout, stderr, and exit code.
"""

import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)
from cases import CASES, EXAMPLE_FILES, EXAMPLE_STDIN  # noqa: E402


def run_engine(cmd, path, stdin_text):
    r = subprocess.run(
        cmd + ["run", path],
        input=stdin_text, capture_output=True, text=True, cwd=ROOT, timeout=30,
    )
    return {"exit": r.returncode, "stdout": r.stdout, "stderr": r.stderr}


def main():
    cmd = sys.argv[1:]
    if not cmd:
        print(__doc__)
        return 2
    with open(os.path.join(HERE, "expected.json"), encoding="utf-8") as f:
        expected = json.load(f)

    jobs = []
    tmp = os.path.join(HERE, ".case.ml")
    for name, source, stdin_text in CASES:
        jobs.append((name, source, None, stdin_text))
    for fname in EXAMPLE_FILES:
        jobs.append((f"example:{fname}", None,
                     os.path.join(ROOT, "examples", fname),
                     EXAMPLE_STDIN.get(fname, "")))

    failed = []
    try:
        for name, source, path, stdin_text in jobs:
            if path is None:
                with open(tmp, "w", encoding="utf-8") as f:
                    f.write(source)
                path = tmp
            got = run_engine(cmd, path, stdin_text)
            want = expected[name]
            if got != want:
                failed.append(name)
                print(f"✗ {name}")
                for key in ("exit", "stdout", "stderr"):
                    if got[key] != want[key]:
                        print(f"    {key}: want {want[key]!r}")
                        print(f"    {key}:  got {got[key]!r}")
            else:
                print(f"✓ {name}")
    finally:
        if os.path.exists(tmp):
            os.remove(tmp)

    total = len(jobs)
    print(f"\n{total - len(failed)}/{total} conformance cases pass")
    if failed:
        print("failed:", ", ".join(failed))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
