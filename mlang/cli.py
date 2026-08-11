"""The mlang command line: run, render, and inspect MLang programs."""

import argparse
import sys

from .errors import LoadError
from .forms import parse_source, to_flat, to_rain
from .sigils import ARG2_OPS, ARG_OPS, OPS
from .vm import VM


def _read(path):
    if path == "-":
        return sys.stdin.read()
    with open(path, encoding="utf-8") as f:
        return f.read()


def _run_source(text):
    vm = VM()
    try:
        prog = parse_source(text)
    except LoadError as e:
        loc = f" at {e.pos[0]}:{e.pos[1]}" if e.pos else ""
        sys.stderr.write(f"✗ weave error{loc}: {e.msg}\n")
        return 2
    try:
        return vm.run_program(prog)
    except LoadError as e:
        loc = f" at {e.pos[0]}:{e.pos[1]}" if e.pos else ""
        sys.stderr.write(f"✗ weave error{loc}: {e.msg}\n")
        return 2


def _ops_table():
    lines = ["MLang sigils — every operation is one character.", ""]
    lines.append("sigil  name          effect              description")
    lines.append("─" * 78)
    for table in (OPS, ARG_OPS, ARG2_OPS):
        suffix = {id(OPS): "", id(ARG_OPS): "X", id(ARG2_OPS): "XY"}[id(table)]
        for ch, info in table.items():
            shown = ch + suffix
            lines.append(f"{shown:<6} {info.name:<13} {info.sig:<19} {info.doc}")
    lines += [
        "",
        "literals:  123  ¯5  2.5  «string»  (⏎ = newline)  ⟨1 2 3⟩  [quotation]  ∅",
        "layout:    ⇓ first line = rain form (columns are strands, read downward)",
        "           ⇊ divider = boot section above, strands below",
        "           ⋮ line prefix = continue previous strand (flat form)",
        "           ※ comment to end of line (flat) / end of column (rain)",
    ]
    return "\n".join(lines)


def main(argv=None):
    ap = argparse.ArgumentParser(
        prog="mlang",
        description="MLang — the Matrix language. Strands of glyphs, "
        "falling in columns, running in parallel.",
    )
    sub = ap.add_subparsers(dest="cmd", required=True)
    p_run = sub.add_parser("run", help="run a program (.ml, rain or flat form)")
    p_run.add_argument("file", help="source file, or - for stdin")
    p_eval = sub.add_parser("eval", help="run flat-form source given as an argument")
    p_eval.add_argument("code")
    p_rain = sub.add_parser("rain", help="render flat source as the vertical rain grid")
    p_rain.add_argument("file")
    p_flat = sub.add_parser("flat", help="render rain source as flat lines")
    p_flat.add_argument("file")
    sub.add_parser("ops", help="print the sigil reference table")
    sub.add_parser("std", help="print the standard library source (self-documenting)")

    ns = ap.parse_args(argv)
    try:
        if ns.cmd == "run":
            return _run_source(_read(ns.file))
        if ns.cmd == "eval":
            return _run_source(ns.code)
        if ns.cmd == "rain":
            sys.stdout.write(to_rain(_read(ns.file)))
            return 0
        if ns.cmd == "flat":
            sys.stdout.write(to_flat(_read(ns.file)))
            return 0
        if ns.cmd == "ops":
            print(_ops_table())
            return 0
        if ns.cmd == "std":
            import os

            path = os.path.join(os.path.dirname(__file__), "std.ml")
            with open(path, encoding="utf-8") as f:
                sys.stdout.write(f.read())
            return 0
    except LoadError as e:
        loc = f" at {e.pos[0]}:{e.pos[1]}" if e.pos else ""
        sys.stderr.write(f"✗ weave error{loc}: {e.msg}\n")
        return 2
    except OSError as e:
        sys.stderr.write(f"✗ {e}\n")
        return 2


if __name__ == "__main__":
    sys.exit(main())
