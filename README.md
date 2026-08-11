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

MLang is its own language with its own toolchain: one native binary,
`mlang`, is the compiler, runner, and runtime. (The toolchain is
implemented in Rust — the way C's first compilers were implemented in
assembly — but MLang programs never touch Rust, and nothing else is
involved: no C compiler, no linker, no interpreter dependency.)

Build the toolchain once (from the repo root, any platform):

```sh
cargo build --release --manifest-path compiler/Cargo.toml
```

Then use the bundled launcher — `./mlang` on Linux/macOS, `.\mlang.cmd` in
PowerShell — no alias or PATH setup needed. Linux / macOS:

```sh
./mlang build examples/mandelbrot.ml -o mandelbrot   # compile → native binary
./mandelbrot                                         # standalone: no toolchain needed

./mlang run examples/editor.ml         # or compile-and-run in one step
./mlang eval '«Hello, Matrix»⍞'      # inline source
./mlang check examples/calc.ml       # compile only, report weave errors
./mlang rain examples/pipeline.ml    # render the vertical rain view
./mlang ops                          # the sigil reference
./mlang std                          # the standard library source
```

Windows (PowerShell) — same commands via `.\mlang.cmd`, and name compiled
output `.exe`:

```powershell
.\mlang.cmd build examples\mandelbrot.ml -o mandelbrot.exe
.\mandelbrot.exe
.\mlang.cmd run examples\editor.ml
```

Windows notes: WSL is *not* required — the toolchain and welded binaries
are fully native on Windows. Use Windows Terminal (not legacy conhost)
with a Unicode-capable font so the glyphs render. The `cargo test` suite,
however, assumes a Unix-like filesystem (`/tmp` paths in the recorded
file-I/O goldens; CI runs Ubuntu) — run it on Linux, macOS, or WSL.

`mlang build` compiles source to MLang bytecode and welds it into a copy
of the toolchain's own runtime image — the same shape as a Go binary,
where the runtime is linked into every executable. The result is a
self-contained native executable: it starts in milliseconds, embeds the
standard library, and can never hit a runtime-version mismatch, because
it carries the exact runtime it was built with.

The language's observable behavior is pinned by a recorded conformance
corpus — 125 cases covering every operation, concurrency, glitches, both
source forms, and all example programs, compared byte-for-byte on stdout,
stderr, and exit code (`cargo test` runs it; the goldens in
`conformance/expected.json` are the spec's ground truth, and any future
second implementation must reproduce them exactly).

## The idea

| Requirement | MLang's answer |
|---|---|
| **Token efficiency** | One glyph = one operation, drawn from the full Unicode set. FizzBuzz is 71 characters. A concatenative stack model means no variable-name ceremony, no keywords, no delimiters between operations. |
| **Parallel strands, linear writing** | One flat line = one strand = one machine; all strands run concurrently. Because LLMs emit text linearly, flat form is the authoring form; the vertical rain grid is the same program transposed (a `mlang rain` view), so the punch-tape geometry costs nothing at writing time. |
| **Linking machines without breaking code** | Strands share *nothing*. They communicate only over named channels — raw `↥α` send / `↧α` receive, or the stream combinators `⇈` pour, `⇉` pump, `⇟` drain, which carry the end-of-stream protocol for you. Adding, removing, or reordering strands never changes the meaning of another strand — the channel names are the entire interface. |
| **Memory safety** | Every value is immutable. There are no pointers, no references, no shared mutable state. Data races are impossible *by construction*, not by discipline. Indexing is bounds-checked; arithmetic on wrong types is a caught fault, never corruption. |
| **Error handling** | Any fault is a **glitch** carrying a value and exact grid coordinates. `⍥` catches glitches with the stack restored to a known depth. An uncaught glitch kills only its own strand — the rest of the grid keeps running (Erlang-style isolation) and the run exits nonzero with a precise report. If every remaining strand is blocked, the scheduler *proves* the deadlock and reports who waits on what, instead of hanging. |
| **Performance & reproducibility** | Programs compile to standalone native binaries (`mlang build`). Strands are scheduled round-robin with a fixed slice: identical input ⇒ identical run, including output interleaving. Deterministic concurrency is what makes machine-written concurrent code debuggable by a machine. |

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

A **standard library** — written in MLang itself (`mlang std` prints it) —
is woven into every program before its boot section: constants (`π τ ℯ ∞`),
numerics (`∣` abs, `⊓` min, `⊔` max, `‼` factorial, `⟌` gcd), aggregates
(`∑` sum, `∏` product, `µ` mean), list tools (`⊃` head, `⌷` last, `⍫`
tail, `⊤`/`⊥` take/drop, `⍒` sort-desc, `⍚` zip), and text tools (`⇑`/`⇩`
case, `⍭` words, `⍖` lines), built on five engine primitives: `⍙` type-of,
`⌽` reverse, `⍋` sort, `∈` contains, `⍷` find. `examples/std-tour.ml`
walks through all of it:

```
⟨31 4 15 9 2 6⟩⍋⍞         ※ ⟨2 4 6 9 15 31⟩
«the matrix has you»⍭⌽« »⊇⍞  ※ you has matrix the
462 1071⟌⍞                ※ 21
⟨«red» «blue»⟩⟨«pill» «shift»⟩⍚⍞  ※ ⟨⟨«red» «pill»⟩ ⟨«blue» «shift»⟩⟩
```

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

The showpiece is `examples/mandelbrot.ml`: an interactive Mandelbrot
explorer. Four worker strands shard the 24 rows by strand id and render
each frame in parallel; a navigator strand owns the viewport, streams
render jobs to the workers over channels, reassembles their rows in
order, and reads commands from stdin — `a d w s` pan, `z x` zoom (the
escape-time depth rises as you dive), `r` reset, `q` quit — a full
interactive event loop in 5 strands and two boot definitions. `examples/calc.ml` is a fault-tolerant concurrent RPN
calculator that evaluates every input line on a freshly spawned strand:
a bad line reports `✗ …` and dies alone; the calculator keeps answering.
And `examples/editor.ml` is **MatrixPad** — a real application: a
Notepad-style text editor in the `ed`/`edlin` lineage, wired like a real
editor's event loop. Keyboard, editor core, and screen are three strands
joined by channels; the document is an immutable list of lines, so every
edit (append, insert, replace, delete) is slice-and-concat and a stray
command can never corrupt it; `w file` and `o file` save and open real
files with the `⍇`/`⍈` primitives; and dispatch runs inside `⍥`, so a bad
command or an unreadable file answers `? …` while the editor and the
document survive untouched. Compile it to a standalone binary and edit
away:

```
$ mlang build examples/editor.ml -o matrixpad && ./matrixpad
MatrixPad — a:append i N:insert d N:delete r N txt:replace f txt:find …
a
The Matrix has you.
Follow the white rabbit.
.
i 2
Wake up, Neo.
.
p
1 │ The Matrix has you.
2 │ Wake up, Neo.
3 │ Follow the white rabbit.
w neo.txt
wrote neo.txt
q
```

## Repository

```
compiler/         the MLang toolchain (one binary: compiler + runner + runtime)
  src/lex.rs      glyph stream → instructions
  src/forms.rs    rain/flat grid parsing and rendering
  src/vm.rs       compile, strands, channels, glitches, deterministic scheduler
  src/payload.rs  bytecode serialization + native binary welding (mlang build)
  tests/          cargo test: unit, payload round-trip, standalone-binary
                  execution, and the full conformance corpus
std/std.ml        the standard library — written in MLang
conformance/      cases.json + expected.json: 122 recorded goldens, the
                  language's observable ground truth (RECORD=1 to re-record)
examples/         runnable programs (mandelbrot, calc, editor, pipeline, …)
SPEC.md           the full language specification
```

## Honest notes

* **Glyphs vs. tokens.** Today's BPE tokenizers may spend more than one
  token on a rare glyph. MLang optimizes *characters and context density* —
  the number of atoms a model must emit correctly — and one-glyph-one-op
  makes a dedicated tokenizer trivial (one token per sigil). Density also
  buys correctness: there is no syntax to misindent and no identifier to
  misspell twice.
* **Determinism over raw parallelism.** The runtime interleaves strands
  deterministically (round-robin, fixed slice) rather than using OS
  threads. The language model (immutable values, channel-only
  communication) is exactly the one that admits true parallel execution
  without data races; the scheduler is an implementation choice, and
  reproducibility is worth more to a machine author than nondeterministic
  wall-clock wins. A parallel scheduler can be added without changing a
  single program's meaning where interleaving is unobservable.

## Name

**M**atrix **Lang**uage. The rain falls in columns. The columns compute.
