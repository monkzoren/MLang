#!/usr/bin/env python3
"""Record expected conformance outputs from the reference implementation.

Usage: python3 conformance/record.py
Writes conformance/expected.json. Review the diff before committing —
the recorded outputs ARE the language's observable specification.
"""

import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)
from cases import CASES, EXAMPLE_FILES, EXAMPLE_STDIN  # noqa: E402


def run_ref(path, stdin_text):
    r = subprocess.run(
        [sys.executable, "-m", "mlang", "run", path],
        input=stdin_text, capture_output=True, text=True, cwd=ROOT, timeout=30,
    )
    return {"exit": r.returncode, "stdout": r.stdout, "stderr": r.stderr}


def main():
    expected = {}
    tmp = os.path.join(HERE, ".case.ml")
    try:
        for name, source, stdin_text in CASES:
            with open(tmp, "w", encoding="utf-8") as f:
                f.write(source)
            expected[name] = run_ref(tmp, stdin_text)
    finally:
        if os.path.exists(tmp):
            os.remove(tmp)
    for fname in EXAMPLE_FILES:
        path = os.path.join(ROOT, "examples", fname)
        expected[f"example:{fname}"] = run_ref(path, EXAMPLE_STDIN.get(fname, ""))
    out = os.path.join(HERE, "expected.json")
    with open(out, "w", encoding="utf-8") as f:
        json.dump(expected, f, ensure_ascii=False, indent=1, sort_keys=True)
        f.write("\n")
    print(f"recorded {len(expected)} cases → {out}")


if __name__ == "__main__":
    main()
