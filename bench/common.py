"""Shared plumbing for the MLang bench harness.

Corpus loading, sandboxed program execution with a wall-clock timeout, and
outcome classification for both arms (MLang and Python). No third-party
dependencies — Python 3.10+ stdlib only.
"""

import json
import os
import subprocess
import sys
import tempfile

BENCH = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(BENCH)
MLANG_BIN = os.path.join(ROOT, "compiler", "target", "release", "mlang")
TIMEOUT = 5.0  # seconds; every conformance program finishes in milliseconds

# Outcome buckets. The first three are "loud" failures — the machine author
# is handed a precise, actionable report. `wrong-output` is the silent
# bucket: the run completed cleanly and produced the wrong bytes. `hang` is
# a busy loop that never terminates (MLang proves *deadlocks*; it cannot
# prove away infinite computation).
LOUD = {"weave", "glitch", "deadlock", "syntax", "exception"}


def ensure_mlang():
    if not os.path.exists(MLANG_BIN):
        subprocess.run(
            ["cargo", "build", "--release",
             "--manifest-path", os.path.join(ROOT, "compiler", "Cargo.toml")],
            check=True)
    return MLANG_BIN


def load_corpus(max_source_chars=None, include_examples=True):
    """Exit-0 conformance cases: [{name, source, stdin, expected}].

    Only clean-run cases are mutated — a benchmark mutant needs an
    unambiguous 'expected output' to heal toward.
    """
    cases = json.load(open(os.path.join(ROOT, "conformance", "cases.json")))
    expected = json.load(open(os.path.join(ROOT, "conformance", "expected.json")))
    corpus = []
    for c in cases["cases"]:
        e = expected[c["name"]]
        if e["exit"] != 0:
            continue
        corpus.append({"name": c["name"], "source": c["source"],
                       "stdin": c["stdin"], "expected": e})
    if include_examples:
        for x in cases["examples"]:
            name = "example:" + x["name"]
            e = expected[name]
            if e["exit"] != 0:
                continue
            source = open(os.path.join(ROOT, x["file"]), encoding="utf-8").read()
            corpus.append({"name": name, "source": source,
                           "stdin": x["stdin"], "expected": e})
    if max_source_chars is not None:
        corpus = [c for c in corpus if len(c["source"]) <= max_source_chars]
    return corpus


def load_python_corpus():
    """The verified Python ports: [{name, source, stdin, expected}]."""
    ports = os.path.join(BENCH, "python_ports")
    manifest = json.load(open(os.path.join(ports, "manifest.json")))
    expected = json.load(open(os.path.join(ports, "expected.json")))
    corpus = []
    for name, meta in sorted(manifest.items()):
        source = open(os.path.join(ports, meta["file"]), encoding="utf-8").read()
        corpus.append({"name": name, "source": source,
                       "stdin": meta["stdin"], "expected": expected[name]})
    return corpus


def run_program(argv, stdin_text, timeout=TIMEOUT, cwd=None):
    """Run a program, return {exit, stdout, stderr, hang}."""
    try:
        p = subprocess.run(argv, input=stdin_text.encode(),
                           capture_output=True, timeout=timeout, cwd=cwd)
        return {"exit": p.returncode,
                "stdout": p.stdout.decode("utf-8", "replace"),
                "stderr": p.stderr.decode("utf-8", "replace"),
                "hang": False}
    except subprocess.TimeoutExpired as t:
        out = (t.stdout or b"").decode("utf-8", "replace")
        err = (t.stderr or b"").decode("utf-8", "replace")
        return {"exit": None, "stdout": out, "stderr": err, "hang": True}


def run_mlang(source, stdin_text, timeout=TIMEOUT):
    """Run MLang source in a scratch cwd (some cases write temp files)."""
    with tempfile.TemporaryDirectory(prefix="mlang-bench-") as d:
        src = os.path.join(d, "prog.ml")
        with open(src, "w", encoding="utf-8") as f:
            f.write(source)
        return run_program([ensure_mlang(), "run", src], stdin_text,
                           timeout=timeout, cwd=d)


def run_python(source, stdin_text, timeout=TIMEOUT):
    with tempfile.TemporaryDirectory(prefix="pybench-") as d:
        src = os.path.join(d, "prog.py")
        with open(src, "w", encoding="utf-8") as f:
            f.write(source)
        return run_program([sys.executable, src], stdin_text,
                           timeout=timeout, cwd=d)


def matches(result, expected):
    return (not result["hang"] and result["exit"] == expected["exit"]
            and result["stdout"] == expected["stdout"]
            and result["stderr"] == expected["stderr"])


def classify_mlang(result, expected):
    """Bucket a mutant run. Priority: how the failure announces itself."""
    if result["hang"]:
        return "hang"
    if matches(result, expected):
        return "pass"
    if result["exit"] == 2 and "✗ weave error" in result["stderr"]:
        return "weave"
    if "✗ glitch" in result["stderr"]:
        return "glitch"
    if "✗ deadlock" in result["stderr"]:
        return "deadlock"
    return "wrong-output"


def classify_python(result, expected):
    if result["hang"]:
        return "hang"
    if matches(result, expected):
        return "pass"
    err = result["stderr"]
    if result["exit"] != 0:
        if "SyntaxError" in err or "IndentationError" in err or "TabError" in err:
            return "syntax"
        return "exception"
    return "wrong-output"


def failure_report(result, expected=None):
    """The failure exactly as the program announced it, for the repair prompt.

    When the golden is given, a first-divergence hint is appended for runs
    whose stdout is wrong — the same hint in both arms, since it is
    computed by the harness, not the language.
    """
    if result["hang"]:
        return (f"The program did not terminate (killed after {TIMEOUT:.0f}s).\n"
                f"stdout so far:\n{result['stdout'] or '(none)'}")
    parts = [f"exit code: {result['exit']}"]
    parts.append("stdout:\n" + (result["stdout"] if result["stdout"] else "(empty)"))
    if result["stderr"]:
        parts.append("stderr:\n" + result["stderr"])
    if expected is not None and result["stdout"] != expected["stdout"]:
        got = result["stdout"].split("\n")
        want = expected["stdout"].split("\n")
        for i in range(max(len(got), len(want))):
            g = got[i] if i < len(got) else None
            w = want[i] if i < len(want) else None
            if g != w:
                parts.append(
                    f"first stdout divergence at line {i + 1}:\n"
                    f"  expected: {w if w is not None else '(no such line)'}\n"
                    f"  actual:   {g if g is not None else '(no such line)'}")
                break
    return "\n".join(parts)
