"""The MLang sigil registry: every operation is exactly one Unicode character.

This table is the single source of truth for the instruction set. The lexer
uses the keys; the VM binds implementations; `mlang ops` prints the docs.
"""

from collections import namedtuple

OpInfo = namedtuple("OpInfo", "name sig doc")

# Ops that consume the NEXT character in the strand as their argument.
ARG_OPS = {
    "≔": OpInfo("define", "v ≔X →", "bind v (often a quotation) to global sigil X; rebinding glitches"),
    "⇒": OpInfo("store", "v ⇒X →", "pop v into strand-local sigil X (rebindable)"),
    "↥": OpInfo("send", "v ↥X →", "send v down channel X (never blocks; unbounded FIFO)"),
    "↧": OpInfo("recv", "↧X → v", "receive from channel X; blocks until a value arrives"),
    "⇂": OpInfo("try-recv", "⇂X → v 1 | 0", "non-blocking receive: value and 1, or just 0"),
    "⇈": OpInfo("pour", "L ⇈X →", "send each item of a list/string down channel X, then the ∅ end-marker"),
    "⇟": OpInfo("drain", "⇟X → L", "collect from channel X until ∅ into a list (blocks as needed)"),
}

# Ops that consume the next TWO characters (source and destination channels).
ARG2_OPS = {
    "⇉": OpInfo("pump", "[f] ⇉XY →", "for each value from channel X run f (v → v′) and send to Y; forward ∅ and stop"),
}

# Plain single-glyph ops.
OPS = {
    # ── stack ──
    "∂": OpInfo("dup", "a → a a", "duplicate top of stack"),
    "⇅": OpInfo("swap", "a b → b a", "swap top two values"),
    "⌫": OpInfo("drop", "a →", "discard top of stack"),
    "⊚": OpInfo("over", "a b → a b a", "copy second value to top"),
    "⥀": OpInfo("rot", "a b c → b c a", "rotate third value to top"),
    "≢": OpInfo("depth", "→ n", "push current stack depth"),
    # ── arithmetic ──
    "+": OpInfo("add", "a b → a+b", "add numbers"),
    "-": OpInfo("sub", "a b → a−b", "subtract (binary only; write negatives as ¯5)"),
    "×": OpInfo("mul", "a b → a×b", "multiply numbers"),
    "÷": OpInfo("div", "a b → a÷b", "divide; int÷int stays int when exact, else float; ÷0 glitches"),
    "%": OpInfo("mod", "a b → a%b", "modulo; %0 glitches"),
    "^": OpInfo("pow", "a b → aᵇ", "exponentiation"),
    "√": OpInfo("sqrt", "a → √a", "square root (float); negative glitches"),
    "⌊": OpInfo("floor", "a → ⌊a⌋", "floor to int"),
    "⌈": OpInfo("ceil", "a → ⌈a⌉", "ceiling to int"),
    "±": OpInfo("neg", "a → −a", "negate a number"),
    # ── comparison (push 1 or 0) ──
    "=": OpInfo("eq", "a b → 1|0", "equal (any types; deep for lists)"),
    "≠": OpInfo("ne", "a b → 1|0", "not equal"),
    "<": OpInfo("lt", "a b → 1|0", "less than (numbers or strings)"),
    "≤": OpInfo("le", "a b → 1|0", "less or equal"),
    ">": OpInfo("gt", "a b → 1|0", "greater than"),
    "≥": OpInfo("ge", "a b → 1|0", "greater or equal"),
    # ── logic (truthiness: 0, 0.0, «», ⟨⟩, ∅ are false) ──
    "∧": OpInfo("and", "a b → 1|0", "logical and"),
    "∨": OpInfo("or", "a b → 1|0", "logical or"),
    "¬": OpInfo("not", "a → 1|0", "logical not"),
    "⊻": OpInfo("xor", "a b → 1|0", "logical exclusive or"),
    # ── control ──
    "!": OpInfo("apply", "[q] ! → …", "run a quotation"),
    "?": OpInfo("if", "c t e ? → …", "if c truthy take t else e; a quotation runs, a value is pushed"),
    "⟳": OpInfo("while", "[c] [b] ⟳ → …", "run [c]; while it leaves truthy, run [b] and repeat"),
    "⍣": OpInfo("repeat", "n [b] ⍣ → …", "run [b] n times"),
    # ── iteration (accept list or string; string iterates 1-char strings) ──
    "∵": OpInfo("map", "L [f] ∵ → L′", "run [f] on each item, collect results into a list"),
    "∀": OpInfo("each", "L [f] ∀ → …", "run [f] on each item for its effects"),
    "⌿": OpInfo("filter", "L [f] ⌿ → L′", "keep items for which [f] leaves truthy"),
    "⍀": OpInfo("fold", "L a [f] ⍀ → a′", "fold: for each item run [f] as a x → a′"),
    "⍸": OpInfo("range", "n ⍸ → ⟨0…n−1⟩", "push the list 0,1,…,n−1"),
    # ── sequences (strings & lists) ──
    "#": OpInfo("length", "s # → n", "length of a string or list"),
    "⧺": OpInfo("concat", "a b → ab", "concatenate two strings or two lists"),
    "@": OpInfo("index", "s i @ → v", "item at 0-based index; out of bounds glitches"),
    "⊂": OpInfo("slice", "s i j ⊂ → s′", "slice [i, j) with clamping"),
    "⊆": OpInfo("split", "s sep ⊆ → L", "split string by sep; empty sep splits into characters"),
    "⊇": OpInfo("join", "L sep ⊇ → s", "join list items (stringified) with sep"),
    "⍕": OpInfo("to-str", "v ⍕ → s", "format any value as a string"),
    "⍎": OpInfo("parse-num", "s ⍎ → n", "parse a string as int/float; glitches if malformed"),
    "⌗": OpInfo("codepoint", "c ⌗ → n", "Unicode code point of a 1-char string"),
    "⍘": OpInfo("char", "n ⍘ → c", "1-char string from a Unicode code point"),
    # ── strands & channels ──
    "⚡": OpInfo("spawn", "[q] ⚡ → id", "spawn a new strand running [q]; inherits a copy of locals"),
    "⋈": OpInfo("join", "id ⋈ →", "block until strand id has finished"),
    "⍳": OpInfo("strand-id", "→ id", "this strand's id (grid columns are 0,1,2,…; boot is ¯1)"),
    "≣": OpInfo("strand-count", "→ n", "number of main strands (the grid's width)"),
    "⌛": OpInfo("yield", "→", "end this strand's scheduler slice early"),
    # ── glitches (errors) ──
    "⍥": OpInfo("try", "[b] [h] ⍥ → …", "run [b]; on glitch restore stack, push the glitch value, run [h]"),
    "↯": OpInfo("raise", "v ↯ →", "raise v as a glitch"),
    # ── i/o ──
    "⍞": OpInfo("println", "v ⍞ →", "print v and a newline"),
    "⊸": OpInfo("print", "v ⊸ →", "print v without a newline"),
    "⌨": OpInfo("readline", "→ s|∅", "read a line from stdin (∅ at end of input)"),
    "⍟": OpInfo("debug", "→", "dump this strand's stack to stderr"),
}

# Characters with structural meaning; they can never be user sigils.
STRUCTURAL = set("«»⟨⟩[]⏎¯.※⋮⇓⇊∅ \t")
DIGITS = set("0123456789")
RESERVED = set(OPS) | set(ARG_OPS) | set(ARG2_OPS) | STRUCTURAL | DIGITS
