import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from mlang.vm import run_text  # noqa: E402


def run(src, stdin=""):
    """Run MLang source, return (exit_code, stdout, stderr)."""
    return run_text(src, stdin_text=stdin)


def out_of(src, stdin=""):
    """Run source that must succeed; return stdout."""
    code, out, err = run(src, stdin)
    assert code == 0, f"exit {code}, stderr: {err}"
    return out
