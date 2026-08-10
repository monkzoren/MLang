# MLang — the Matrix Language

**Programs are grids. Columns are threads. Every operation is one glyph.**

MLang is a programming language designed for LLMs, not humans. One Unicode
character is one operation, so semantic density per character is pushed to
the physical limit. No human needs to read it. That's fine.

You *write* MLang flat: **one line is one strand** — an independent machine
— and all strands run concurrently. This is a complete three-machine
pipeline in 25 glyphs:

```
9⍸[1+∂×]∵⇈α
[2×]⇉αβ
⇟β[⍞]∀
```

Strand 0 pours the squares of 1–9 into channel `α`; strand 1 pumps each
value from `α`, doubles it, and sends it on to `β`; strand 2 drains `β` and
prints. The three machines are synchronized only by their channels; the
end-of-stream protocol (`∅`) is built into the `⇈ ⇉ ⇟` combinators.
Adding machines means adding lines — the program scales like feeding more
tape readers, and no strand's meaning ever depends on its neighbors.

The same program can be *viewed* as digital rain (`mlang rain` renders it;
the engine runs both forms identically — they are transposes):

```
⇓
9  [  ⇟
⍸  2  β
[  ×  [
1  ]  ⍞
+  ⇉  ]
∂  α  ∀
×  β
]
∵
⇈
α
```

Code falling in columns, like the green cascade of the Matrix — or a rack
of punched-tape readers, one tape per machine.

## Try it

**Native engine** (Rust — the primary implementation):

```sh
cd rust && cargo build --release && cd ..
rust/target/release/mlang run examples/pipeline.ml   # run a program
rust/target/release/mlang eval '«Hello, Matrix»⍞'    # run inline source
rust/target/release/mlang rain examples/pipeline.ml  # render the rain view
rust/target/release/mlang ops                        # the sigil reference
```

**Reference implementation** (pure Python, no dependencies — the executable
spec): the same commands via `python3 -m mlang …`, or `pip install .` for an
`mlang` entry point.

Both engines are verified byte-for-byte against a shared conformance corpus
(104 cases: stdout, stderr, and exit codes, including glitch coordinates and
scheduler interleaving):

```sh
python3 conformance/run.py rust/target/release/mlang
python3 conformance/run.py python3 -m mlang
```

On a 10⁶-iteration loop the native engine runs ~15× faster than the
reference (0.45s vs 6.9s on the machine this was developed on).

## The idea

| Requirement | MLang's answer |
|---|---|
| **Token efficiency** | One glyph = one operation, drawn from the full Unicode set. FizzBuzz is 71 characters. A concatenative stack model means no variable-name ceremony, no keywords, no delimiters between operations. |
| **Parallel strands, linear writing** | One flat line = one strand = one machine; all strands run concurrently. Because LLMs emit text linearly, flat form is the authoring form; the vertical rain grid is the same program transposed (a `mlang rain` view), so the punch-tape geometry costs nothing at writing time. |
| **Linking machines without breaking code** | Strands share *nothing*. They communicate only over named channels — raw `↥α` send / `↧α` receive, or the stream combinators `⇈` pour, `⇉` pump, `⇟` drain, which carry the end-of-stream protocol for you. Adding, removing, or reordering strands never changes the meaning of another strand — the channel names are the entire interface. |
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
100⍸⇈α                             ※ pour ⟨0..99⟩ into channel α, then ∅
[∂×]⇉αβ                            ※ pump: square each value from α into β
⇟β 0[+]⍀⍞                          ※ drain β to a list, fold with + : 328350
```

Numbers write negatives as `¯5` (¯ binds to the literal; `-` is always
binary subtraction). Strings are `«…»` with `⏎` for newline. Lists are
`⟨1 2 3⟩`. `∅` is nil — conventionally the end-of-stream sentinel on
channels.

In **flat form** each line is a strand — write this. In **rain form**
(first line `⇓`) each *column* is a strand — view this. `mlang rain` and
`mlang flat` transpose between them; both run identically. A `⇊` divider
marks a **boot section** that runs to completion before the strands start
— the place for shared `≔` definitions.

Scaling is literal geometry: four workers and a reducer are five columns.
Each worker reads its own strand id `⍳`, picks its chunk, and sends a
partial sum down `σ`; the reducer adds them (`examples/parallel-sum.ml`,
prints 500500). At runtime, `⚡` spawns new strands to scale beyond the
grid's static width.

The showpiece is `examples/mandelbrot.ml`: the full Mandelbrot set,
rendered as ASCII shading by four worker strands that shard the rows by
strand id while a reducer reassembles them in order — escape-time
iteration, palette lookup, and row assembly in 5 strands and two boot
definitions. `examples/calc.ml` is a fault-tolerant concurrent RPN
calculator that evaluates every input line on a freshly spawned strand:
a bad line reports `✗ …` and dies alone; the calculator keeps answering.

## Repository

```
rust/             the native engine (Rust) — the primary implementation
  src/values.rs   immutable values (bignum ints, floats, strings, lists, quotations)
  src/lex.rs      glyph stream → instructions
  src/forms.rs    rain/flat grid parsing and rendering
  src/vm.rs       strands, channels, glitches, deterministic scheduler
mlang/            the reference implementation (pure Python, stdlib only) —
                  the executable specification the native engine is held to
conformance/      shared corpus: record.py snapshots the reference,
                  run.py verifies any engine byte-for-byte (104 cases)
examples/         runnable programs (hello, fizzbuzz, pipeline, spawn, …)
tests/            85 tests: python3 -m unittest discover -s tests
SPEC.md           the full language specification
```

## Honest notes

* **Glyphs vs. tokens.** Today's BPE tokenizers may spend more than one
  token on a rare glyph. MLang optimizes *characters and context density* —
  the number of atoms a model must emit correctly — and one-glyph-one-op
  makes a dedicated tokenizer trivial (one token per sigil). Density also
  buys correctness: there is no syntax to misindent and no identifier to
  misspell twice.
* **Determinism over raw parallelism.** Both engines interleave strands
  deterministically (round-robin, fixed slice) rather than using OS
  threads. The language model (immutable values, channel-only
  communication) is exactly the one that admits true parallel execution
  without data races; the scheduler is an implementation choice, and
  reproducibility is worth more to a machine author than nondeterministic
  wall-clock wins. A parallel scheduler can be added without changing a
  single program's meaning where interleaving is unobservable.

## Name

**M**atrix **Lang**uage. The rain falls in columns. The columns compute.
