# MLang — the Matrix Language

**Programs are grids. Columns are threads. Every operation is one glyph.**

MLang is a programming language designed for LLMs, not humans. Code rains
down the screen in vertical strands — like the green cascade of the Matrix,
or the columns of a punched tape — and every strand executes concurrently.
One Unicode character is one operation, so semantic density per character
is pushed to the physical limit. No human needs to read it. That's fine.

```
⇓
9  [  [
⍸  ↧  ↧
[  α  β
1  ∂  ∂
+  ∅  ∅
∂  ≠  ≠
×  ]  ]
↥  [  [
α  2  ⍞
]  ×  ]
∀  ↥  ⟳
∅  β  ⌫
↥  ]
α  ⟳
   ⌫
   ∅
   ↥
   β
```

That is a complete three-machine pipeline: the left strand emits the squares
of 1–9 into channel `α`, the middle strand doubles everything from `α` into
`β`, the right strand prints everything from `β`. All three run at once,
synchronized only by their channels. Adding machines means adding columns —
the program scales horizontally, exactly like feeding more tape readers.

## Try it

No dependencies. Python ≥ 3.9.

```sh
python3 -m mlang run examples/pipeline.ml     # run a program
python3 -m mlang eval '«Hello, Matrix»⍞'      # run inline source
python3 -m mlang rain examples/pipeline.ml    # render the vertical rain view
python3 -m mlang ops                          # the full sigil reference
```

Or install the `mlang` command: `pip install .`

## The idea

| Requirement | MLang's answer |
|---|---|
| **Token efficiency** | One glyph = one operation, drawn from the full Unicode set. FizzBuzz is 71 characters. A concatenative stack model means no variable-name ceremony, no keywords, no delimiters between operations. |
| **Vertical, parallel execution** | Source is a 2D grid. Each column is a **strand** — an independent machine executing top to bottom. All strands run concurrently. A flat (transposed) form exists because LLMs emit text linearly; the two forms are the same program and convert losslessly. |
| **Linking machines without breaking code** | Strands share *nothing*. They communicate only over named channels (`↥α` send, `↧α` receive). Adding, removing, or reordering strands never changes the meaning of another strand — the channel names are the entire interface. |
| **Memory safety** | Every value is immutable. There are no pointers, no references, no shared mutable state. Data races are impossible *by construction*, not by discipline. Indexing is bounds-checked; arithmetic on wrong types is a caught fault, never corruption. |
| **Error handling** | Any fault is a **glitch** carrying a value and exact grid coordinates. `⍥` catches glitches with the stack restored to a known depth. An uncaught glitch kills only its own strand — the rest of the grid keeps running (Erlang-style isolation) and the run exits nonzero with a precise report. If every remaining strand is blocked, the scheduler *proves* the deadlock and reports who waits on what, instead of hanging. |
| **Performance & reproducibility** | Strands are scheduled round-robin with a fixed slice: identical input ⇒ identical run, including output interleaving. Deterministic concurrency is what makes machine-written concurrent code debuggable by a machine. This repo is the reference interpreter; the spec is written so a native compiler needs no language changes. |

## Thirty seconds of MLang

MLang is concatenative (Forth/Joy lineage) with APL-style glyphs. Values go
on a stack; every glyph transforms the stack. `[...]` quotes code as a
value, which is how control flow works.

```
3 4+⍞                              ※ 7 — postfix: push 3, push 4, add, print
0 1[∂1000<][∂⍞⇅⊚+]⟳⌫⌫              ※ Fibonacci below 1000
10⍸[1+∂×]∵⍞                        ※ squares of 1..10 → ⟨1 4 9 … 100⟩
101⍸ 0[+]⍀⍞                        ※ 5050 — fold ⟨0..100⟩ with +
[∂×]≔² 12²⍞                        ※ define ² as square, call it: 144
[1 0÷][«caught: »⇅⍕⧺⍞]⍥            ※ caught: ÷ by zero
[42↥r]⚡⋈↧r⍞                        ※ spawn a strand, join it, read its answer
```

Numbers write negatives as `¯5` (¯ binds to the literal; `-` is always
binary subtraction). Strings are `«…»` with `⏎` for newline. Lists are
`⟨1 2 3⟩`. `∅` is nil — conventionally the end-of-stream sentinel on
channels.

In **flat form** each line is a strand. In **rain form** (first line `⇓`)
each *column* is a strand — the canonical, vertical presentation. `mlang
rain` and `mlang flat` transpose between them; both run identically. A `⇊`
divider marks a **boot section** that runs to completion before the rain
starts — the place for shared `≔` definitions.

Scaling is literal geometry: four workers and a reducer are five columns.
Each worker reads its own strand id `⍳`, picks its chunk, and sends a
partial sum down `σ`; the reducer adds them (`examples/parallel-sum.ml`,
prints 500500). At runtime, `⚡` spawns new strands to scale beyond the
grid's static width.

## Repository

```
mlang/            the reference implementation (pure Python, stdlib only)
  sigils.py       the instruction set — single source of truth
  lex.py          glyph stream → instructions
  forms.py        rain/flat grid parsing and rendering
  vm.py           strands, channels, glitches, deterministic scheduler
  cli.py          mlang run | eval | rain | flat | ops
examples/         runnable programs (hello, fizzbuzz, pipeline, spawn, …)
tests/            79 tests: python3 -m unittest discover -s tests
SPEC.md           the full language specification
```

## Honest notes

* **Glyphs vs. tokens.** Today's BPE tokenizers may spend more than one
  token on a rare glyph. MLang optimizes *characters and context density* —
  the number of atoms a model must emit correctly — and one-glyph-one-op
  makes a dedicated tokenizer trivial (one token per sigil). Density also
  buys correctness: there is no syntax to misindent and no identifier to
  misspell twice.
* **Determinism over raw parallelism.** The reference interpreter
  interleaves strands deterministically rather than using OS threads.
  The language model (immutable values, channel-only communication) is
  exactly the one that compiles to true parallel execution without data
  races; the scheduler is an implementation choice, and reproducibility is
  worth more to a machine author than wall-clock speedups in a reference
  implementation.

## Name

**M**atrix **Lang**uage. The rain falls in columns. The columns compute.
