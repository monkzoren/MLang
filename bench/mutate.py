"""Mechanical bug seeding — one mutation per program copy, automatically labeled.

Because every conformance case has a recorded golden (stdout, stderr, exit,
byte for byte), every mutant is born labeled: the correct output is known
exactly. The same four operator classes are applied to both arms so the
comparison is one-edit against one-edit:

    MLang                      Python
    ---------------------      ------------------------------
    swap one op glyph          swap one operator/punct token
    drop one glyph             drop one token (incl. an indent level)
    transpose adjacent glyphs  transpose adjacent tokens
    rename one channel use     rename one identifier occurrence

Mutations land in code only — string literal interiors and comments are
masked in both arms (flipping a character inside «…» or "…" measures
nothing about a language).
"""

import io
import json
import random
import tokenize

# The engine's op glyphs (from compiler/src/ops.txt) plus the std-library
# sigils — the pool a swapped glyph is drawn from.
MLANG_OPS = list(
    "∂⇅⌫⊚⥀≢+-×÷%^√⌊⌈±=≠<≤>≥∧∨¬⊻!?⟳⍣∵∀⌿⍀⍸⍙⌽⍋∈⍷#⧺@⊂⊆⊇⍕⍎⌗⍘⚡⋈⍳≣⌛⍥↯⍞⊸⌨⍇⍈⍟⌂"
    "≔⇒↥↧⇂⇈⇟⇉"
    "π τ ℯ ∞ ∣ ⊓ ⊔ ‼ ⟌ ∑ ∏ µ ⊃ ⌷ ⍫ ⊤ ⊥ ⍒ ⍚ ⇑ ⇩ ⍭ ⍖".replace(" ", "")
)
MLANG_CHANNEL_PREFIX = "↥↧⇂⇈⇟"  # one name char follows; ⇉ takes two


def mlang_code_mask(source):
    """True at positions that are code: outside «…» strings and ※ comments."""
    mask = [True] * len(source)
    in_string = False
    in_comment = False
    for i, ch in enumerate(source):
        if ch == "\n":
            in_comment = False
            mask[i] = False
            continue
        if in_string:
            if ch == "»":
                in_string = False
                mask[i] = True  # the closing delimiter is code
            else:
                mask[i] = False
            continue
        if in_comment:
            mask[i] = False
            continue
        if ch == "«":
            in_string = True
        elif ch == "※":
            in_comment = True
            mask[i] = False
            continue
        if ch.isspace():
            mask[i] = False
    return mask


def mlang_channel_positions(source, mask):
    """Positions of channel-name characters."""
    pos = []
    chars = list(source)
    for i, ch in enumerate(chars):
        if not mask[i]:
            continue
        if ch in MLANG_CHANNEL_PREFIX and i + 1 < len(chars):
            pos.append(i + 1)
        elif ch == "⇉":
            if i + 1 < len(chars):
                pos.append(i + 1)
            if i + 2 < len(chars):
                pos.append(i + 2)
    return [p for p in pos if p < len(mask) and mask[p]]


def mutate_mlang(source, rng):
    """One seeded mutation; returns (mutated, op_name) or None."""
    chars = list(source)
    mask = mlang_code_mask(source)
    code_pos = [i for i in range(len(chars)) if mask[i]]
    if not code_pos:
        return None
    ops = ["swap-op", "drop", "transpose", "chan-rename"]
    weights = [0.40, 0.25, 0.20, 0.15]
    for _ in range(30):
        op = rng.choices(ops, weights)[0]
        if op == "swap-op":
            cand = [i for i in code_pos if chars[i] in MLANG_OPS]
            if not cand:
                continue
            i = rng.choice(cand)
            new = rng.choice([g for g in MLANG_OPS if g != chars[i]])
            out = chars[:]
            out[i] = new
            return "".join(out), op
        if op == "drop":
            i = rng.choice(code_pos)
            out = chars[:i] + chars[i + 1:]
            return "".join(out), op
        if op == "transpose":
            cand = [i for i in code_pos
                    if i + 1 in code_pos and chars[i] != chars[i + 1]]
            if not cand:
                continue
            i = rng.choice(cand)
            out = chars[:]
            out[i], out[i + 1] = out[i + 1], out[i]
            return "".join(out), op
        if op == "chan-rename":
            cand = mlang_channel_positions(source, mask)
            if not cand:
                continue
            i = rng.choice(cand)
            new = rng.choice([c for c in "abcdexyzστω" if c != chars[i]])
            out = chars[:]
            out[i] = new
            return "".join(out), op
    return None


# ---------------------------------------------------------------- Python arm

PY_OP_POOL = ["+", "-", "*", "/", "//", "%", "**", "==", "!=", "<", "<=",
              ">", ">=", "=", ",", ":", "(", ")", "[", "]"]
PY_KEYWORD_POOL = ["and", "or", "not", "if", "else", "while", "for", "in",
                   "break", "continue", "return", "try", "except", "raise"]


def py_tokens(source):
    """Concrete tokens with source spans, on their physical positions."""
    toks = []
    lines = source.splitlines(keepends=True)
    try:
        for t in tokenize.generate_tokens(io.StringIO(source).readline):
            if t.type in (tokenize.ENCODING, tokenize.ENDMARKER,
                          tokenize.NEWLINE, tokenize.NL, tokenize.COMMENT,
                          tokenize.STRING, tokenize.INDENT, tokenize.DEDENT):
                continue
            if t.start[0] != t.end[0]:
                continue
            toks.append(t)
    except tokenize.TokenizeError:
        pass
    return toks, lines


def py_splice(lines, row, col_a, col_b, replacement):
    """Replace [col_a, col_b) on 1-based physical line `row`."""
    line = lines[row - 1]
    lines = lines[:]
    lines[row - 1] = line[:col_a] + replacement + line[col_b:]
    return "".join(lines)


def mutate_python(source, rng):
    toks, lines = py_tokens(source)
    if not toks:
        return None
    ops = ["swap-op", "drop", "transpose", "ident-rename"]
    weights = [0.40, 0.25, 0.20, 0.15]
    for _ in range(30):
        op = rng.choices(ops, weights)[0]
        if op == "swap-op":
            cand = [t for t in toks
                    if t.string in PY_OP_POOL or t.string in PY_KEYWORD_POOL]
            if not cand:
                continue
            t = rng.choice(cand)
            pool = PY_OP_POOL if t.string in PY_OP_POOL else PY_KEYWORD_POOL
            new = rng.choice([o for o in pool if o != t.string])
            return py_splice(lines, t.start[0], t.start[1], t.end[1], new), op
        if op == "drop":
            # Any token can vanish — including one level of indentation,
            # Python's structural whitespace (the analog of dropping a
            # structural glyph like ] or ⇊ in MLang).
            indented = [i + 1 for i, l in enumerate(lines)
                        if l.startswith("    ") and l.strip()]
            if indented and rng.random() < 0.25:
                row = rng.choice(indented)
                return py_splice(lines, row, 0, 4, ""), "drop"
            t = rng.choice(toks)
            return py_splice(lines, t.start[0], t.start[1], t.end[1], ""), op
        if op == "transpose":
            cand = [(a, b) for a, b in zip(toks, toks[1:])
                    if a.end == b.start and a.string != b.string]
            if not cand:
                continue
            a, b = rng.choice(cand)
            merged = b.string + a.string
            return py_splice(lines, a.start[0], a.start[1], b.end[1], merged), op
        if op == "ident-rename":
            cand = [t for t in toks if t.type == tokenize.NAME
                    and t.string not in PY_KEYWORD_POOL
                    and t.string not in ("def", "import", "from", "as",
                                         "class", "None", "True", "False")]
            if not cand:
                continue
            t = rng.choice(cand)
            return py_splice(lines, t.start[0], t.start[1], t.end[1], "qz"), op
    return None


def make_mutants(arm, cases, seeds):
    """One seeded mutation per case × seed, deduplicated per case.

    Pure generation — no execution. Every mutant carries the golden of the
    program it was cut from, so it is born labeled.
    """
    mutate = mutate_mlang if arm == "mlang" else mutate_python
    mutants = []
    for case in cases:
        seen = {case["source"]}
        for k in range(seeds):
            rng = random.Random(f"{arm}:{case['name']}:{k}")
            m = mutate(case["source"], rng)
            if m is None or m[0] in seen:
                continue
            mutated, op = m
            seen.add(mutated)
            mutants.append({
                "id": f"{arm}:{case['name']}:{k}",
                "case": case["name"],
                "op": op,
                "source": mutated,
                "pristine": case["source"],
                "stdin": case["stdin"],
                "expected": case["expected"],
            })
    return mutants


if __name__ == "__main__":
    import common
    cases = common.load_corpus(max_source_chars=400)
    for mut in make_mutants("mlang", cases[:6], 3):
        r = common.run_mlang(mut["source"], mut["stdin"])
        print(mut["id"], mut["op"], common.classify_mlang(r, mut["expected"]),
              "|", mut["source"].replace("\n", "⏎")[:60])
