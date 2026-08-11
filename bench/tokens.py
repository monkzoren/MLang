"""Measure what MLang's glyph density actually costs in BPE tokens.

Needs `pip install tiktoken` (the one optional dependency in bench/, and
only for this script). Counts characters vs. tokens for the example
programs and their natural Python counterparts where one exists.
"""

import os

import tiktoken

import common

ENCODINGS = ["o200k_base", "cl100k_base"]


def rows():
    ex = os.path.join(common.ROOT, "examples")
    ports = os.path.join(common.BENCH, "python_ports")
    fizz_ml = open(os.path.join(ex, "fizzbuzz.ml"), encoding="utf-8").read()
    yield "fizzbuzz.ml", fizz_ml
    yield "fizzbuzz.py (port)", open(os.path.join(ports, "fizzbuzz.py"),
                                     encoding="utf-8").read()
    yield "pipeline.ml", open(os.path.join(ex, "pipeline.ml"),
                              encoding="utf-8").read()
    yield "pump_pipeline.py (port)", open(os.path.join(ports, "pump_pipeline.py"),
                                          encoding="utf-8").read()
    yield "mandelbrot.ml", open(os.path.join(ex, "mandelbrot.ml"),
                                encoding="utf-8").read()


def main():
    encs = [(n, tiktoken.get_encoding(n)) for n in ENCODINGS]
    head = f"{'program':26s} {'chars':>6s}" + "".join(
        f" {n:>12s}" for n, _ in encs)
    print(head)
    for name, text in rows():
        cells = f"{name:26s} {len(text):6d}"
        for _, enc in encs:
            cells += f" {len(enc.encode(text)):12d}"
        print(cells)


if __name__ == "__main__":
    main()
