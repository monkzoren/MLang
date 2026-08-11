# MLang — the Matrix Language

**Programs are grids. Columns are threads. Every operation is one glyph.**

MLang is a language designed as a **substrate for LLM agents**, not for
human authors: deterministic runs (identical input ⇒ identical output,
interleaving included), faults that arrive as values with exact grid
coordinates, deadlocks that are *proven and reported* instead of hanging,
strands that share nothing — so a patch to one machine cannot break its
neighbors — and a recorded byte-exact conformance corpus that doubles as
an evaluator. No human needs to read it. That's fine.

## This language cannot hang

![One stage of a concurrent pipeline dies: Python freezes forever; MLang prints the proven wait graph and exits](docs/deadlock-demo.svg)

**An agent iterating on concurrent code needs the failure, not a hung
process. MLang proves its deadlocks.** One stage of this three-machine
pipeline glitches mid-stream ([`examples/deadlock.ml`](examples/deadlock.ml)
— its twin, [`docs/deadlock.py`](docs/deadlock.py), is the Python you'd
naturally write with threads and queues):

```
9⍸[1+∂×]∵⇈α                ※ machine 1: pour the squares of 1..9 into α
[∂25=[«boom»↯][]?2×]⇉αβ    ※ machine 2: double each value α→β — dies at 25
[∂⍞]⇉βγ                    ※ machine 3: print each value as it arrives
```

```
$ mlang run examples/deadlock.ml
2
8
18
32
✗ glitch in strand 1 (row 2) at 2:13: boom
✗ deadlock — every remaining strand is blocked:
  strand 2 (row 3) waiting on channel β at 3:5
```

The glitch names its strand and grid coordinates; the scheduler then
proves that every remaining strand is blocked and reports who waits on
which channel, at which coordinates — and exits nonzero, instantly. The
Python twin prints a traceback from the dead worker thread (interleaved
nondeterministically with the output) and then sits frozen until you kill
it. This output is pinned by the conformance suite — the demo above cannot
silently rot.

## The numbers: machines repairing machine code

The conformance corpus doubles as a benchmark generator: every case has a
recorded golden (stdout, stderr, exit — byte for byte), so a mechanically
seeded bug is born labeled, and a fix is verified against the identical
failure because the runtime is deterministic. The [self-repair
benchmark](bench/) flips one token per program copy, shows an LLM the
broken program + expected output + the failure as the runtime announced
it, and lets it patch and re-run for up to 3 rounds — same protocol on
natural Python translations, where the model gets a stack trace instead
of a wait graph.

| Self-repair — claude-haiku-4-5-20251001, ≤3 rounds | MLang | Python |
|---|---|---|
| seeded one-edit bugs | 80 | 80 |
| **healed (byte-exact output)** | **99%** | **100%** |
| healed in one round | 91% | 100% |
| median rounds to green | 1 | 1 |

| healed, by what the bug turned into | MLang | Python |
|---|---|---|
| caught before running | 5/5 | 54/54 |
| runtime fault, precise report | 37/37 | 23/23 |
| proven deadlock | 3/3 | — |
| silent wrong output | 34/35 | 2/2 |
| hang | — | 1/1 |

A small current model repairs one-edit bugs in a language it has *never
seen in training* at essentially the same rate as Python (99% vs 100%),
from an op-reference primer plus the runtime's failure report alone —
including all three proven-deadlock mutants, healed in one round from
the wait graph. The
honest reading is that this experiment bounds the floor, not the
ceiling: the corpus programs are small, and the interesting difference
is *what kind of failure* each language hands the model (see the next
table). Scale it up or swap in any model with one flag —
`bench/README.md` has the protocol and every caveat.

A mutant counts as healed only when stdout, stderr, and exit code match
the golden byte for byte. The same mutation engine, with no LLM, measures
what a one-token bug *becomes* in each language — including the bucket
that should scare you, the silent one:

| One-token mutation becomes | MLang | Python |
|---|---|---|
| caught before running (load error) | 13.2% | 72.5% |
| caught at runtime, precise report | 50.4% | 19.2% |
| deadlock — proven and reported | 2.8% | 0.0% |
| **silent wrong output** | 28.6% | 6.3% |
| hang (killed at timeout) | 0.7% | 1.4% |
| no behavior change (equivalent mutant) | 4.4% | 0.6% |

1124 MLang mutants over 119 programs; 797 Python mutants over 28 ports. Same four operator classes per arm (swap / drop / transpose / rename), one edit per mutant, strings and comments masked. 7 of 11 Python hangs printed a thread traceback first — the process still never exited.

The trade is visible and it cuts both ways. Python's redundant syntax
stops 7 in 10 one-edit bugs at the parser; MLang's dense syntax lets 82%
of mutants run. Of the mutants that do run, MLang faults loudly on 65%
against Python's 71% — and fails silently on 35% against Python's 23% —
so density genuinely costs silent failures, and the table says so. What
density buys back is the *quality* of the loud failures: coordinates and
proven wait graphs instead of a traceback or a frozen process — every
blocked-channel bug above is a printed deadlock proof in MLang and a
kill-it-yourself hang in Python, and in the self-repair benchmark all of
MLang's deadlock mutants healed in one round from the wait graph alone.
Numbers, protocol, and the unflattering buckets all live in
[`bench/`](bench/README.md).

## Programs are grids

You *write* MLang flat: **one line is one strand** — an independent
machine — and all strands run concurrently. This is a complete
three-machine pipeline in 25 glyphs:

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
./mlang ui                           # the Construct — the UI library source
```

Windows (PowerShell) — same commands via `.\mlang.cmd`, and name compiled
output `.exe`:

```powershell
.\mlang.cmd build examples\mandelbrot.ml -o mandelbrot.exe
.\mandelbrot.exe
.\mlang.cmd run examples\editor.ml
```

Windows notes: WSL is *not* required — the toolchain, the welded
binaries, and the `cargo test` suite are fully native on Windows (the
recorded file-I/O goldens use relative paths, and `⌨` strips `\r\n` line
endings). Use Windows Terminal (not legacy conhost) with a
Unicode-capable font so the glyphs render.

`mlang build` compiles source to MLang bytecode and welds it into a copy
of the toolchain's own runtime image — the same shape as a Go binary,
where the runtime is linked into every executable. The result is a
self-contained native executable: it starts in milliseconds, embeds the
standard library, and can never hit a runtime-version mismatch, because
it carries the exact runtime it was built with.

The language's observable behavior is pinned by a recorded conformance
corpus — 154 cases covering every operation, concurrency, glitches, both
source forms, and all example programs, compared byte-for-byte on stdout,
stderr, and exit code (`cargo test` runs it; the goldens in
`conformance/expected.json` are the spec's ground truth, and any future
second implementation must reproduce them exactly).

## The idea

| What an agent needs | MLang's answer |
|---|---|
| **Failures it can act on** | Any fault is a **glitch** carrying a value and exact grid coordinates. `⍥` catches glitches with the stack restored to a known depth. An uncaught glitch kills only its own strand — the rest of the grid keeps running (Erlang-style isolation) and the run exits nonzero with a precise report. If every remaining strand is blocked, the scheduler *proves* the deadlock and reports who waits on what, instead of hanging. |
| **Reproducible runs** | Strands are scheduled round-robin with a fixed slice: identical input ⇒ identical run, including output interleaving. A failure reproduces exactly, so a fix is verified against the identical failure. Deterministic concurrency is what makes machine-written concurrent code debuggable by a machine. |
| **Patches that can't break neighbors** | Strands share *nothing*. They communicate only over named channels — raw `↥α` send / `↧α` receive, or the stream combinators `⇈` pour, `⇉` pump, `⇟` drain, which carry the end-of-stream protocol for you. Adding, removing, or reordering strands never changes the meaning of another strand — the channel names are the entire interface. |
| **Memory safety by construction** | Every value is immutable. There are no pointers, no references, no shared mutable state. Data races are impossible *by construction*, not by discipline. Indexing is bounds-checked; arithmetic on wrong types is a caught fault, never corruption. |
| **A built-in evaluator** | The recorded conformance corpus pins every observable behavior byte for byte — and doubles as a benchmark generator: seed a bug in any corpus program and the correct output is already known ([`bench/`](bench/README.md)). |
| **Linear writing, parallel structure** | One flat line = one strand = one machine. Because LLMs emit text linearly, flat form is the authoring form; the vertical rain grid is the same program transposed (a `mlang rain` view), so the punch-tape geometry costs nothing at writing time. Programs compile to standalone native binaries (`mlang build`). |

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
And `examples/editor.ml` is **MatrixPad** — a real, full-screen text
editor. The document fills the terminal, you type to insert, the cursor
keys move you around: `↑ ↓ ← → Home End PgUp PgDn` — or a mouse click, which places the
cursor where you point — Enter/Backspace/Delete edit, `^S` saves (asking for a name if there is none), `^O`
opens, `^Z`/`^Y` undo and redo, `^F` finds (wrapping around), and `^X`
exits — warning once if there are unsaved changes. The screen looks
like an editor because it is one:

```
 MatrixPad — neo.txt ×
The Matrix has you.
Wake up, Neo.█
Follow the white rabbit.

 ^S save  ^O open  ^Z undo  ^Y redo  ^F find  ^X exit · Ln 2 Col 14
```

Under the hood it is the same three-strand event loop — keyboard (`⌥`
raw input events), editor core, and screen (ANSI frames) joined by channels —
and the whole editor rests on the language's guarantees: the document
is an immutable list of lines, every edit a slice-and-concat, so
undo/redo is literally a list of old `⟨buffer cursor⟩` snapshots;
dispatch runs inside `⍥`, so a glitch becomes a status-bar message and
the document survives by construction. Because `⌥` decodes piped bytes
exactly like live keys, the *same recorded keystrokes always produce
the same screens* — the conformance corpus drives this editor with a
scripted session and pins every frame it draws. Weld it
(`mlang build examples/editor.ml -o matrixpad.exe`) and dropping a
.txt onto the executable opens that file (`⌂`), on any platform —
the runtime enables ANSI processing even in a legacy Windows console.

## The Construct — the UI library

> "This is the Construct. It's our loading program. We can load anything…"

MLang has a widget toolkit in the lineage of Qt/PySide, written in MLang
itself: **the Construct** (`std/ui.ml`, printed by `mlang ui`). The Qt
cast is all here, one glyph each — `Ⓛ` QLabel, `Ⓑ` QPushButton, `Ⓔ`
QLineEdit, `Ⓒ` QCheckBox, `Ⓟ` QProgressBar, `Ⓘ` QListWidget, `Ⓥ`/`Ⓗ`
box layouts, `Ⓦ` QMainWindow — plus `⌺` draw, `▶` `app.exec()`, `⏵`
the live event loop, `◼` `quit()`, and `✎` the status bar. And there
is no import statement: reference a Construct sigil and the loom
weaves the library into your program's boot strand (SPEC §6.1).
Libraries load like weapons racks in the Construct — you name them,
they appear.

Widgets are immutable values, so a PySide program's shape inverts, and
Qt's signals-and-slots become the good kind of simple: a **slot** is a
quotation carried by the widget, and the event loop runs it when the
widget's key arrives on stdin (`key`, or `key argument` for a line
edit). State lives in your strand's locals; the view quotation rebuilds
the widget tree from them every frame, so a stray slot can corrupt
nothing — the worst it can do is glitch, which `▶` catches and shows as
a `✗` status message while the app keeps running. A whole application
is a view and a handful of slots:

```
0⇒c [⟨«count: »c⍕⧺Ⓛ «+1»«+»[c1+⇒c]Ⓑ «Quit»«q»[◼]Ⓑ⟩Ⓥ«Counter»Ⓦ]▶
```

```
┌─ Counter ───┐
│ count: 0    │
│ [ +1 ](+)   │
│ [ Quit ](q) │
└─────────────┘
```

And it is genuinely interactive — keyboard and mouse. The engine op
`⌥` reads one input event: keys arrive as the glyph they are («↑»
«↵» «⌫», Ctrl-C is «␃»), a mouse press as `⟨«⌖» x y⟩`. `⏵` is `▶`
gone live: Tab and the arrow keys move focus (the focused widget wears
`⟦brackets⟧`), typing lands in the focused line edit behind a `▏`
caret, `↵` or space activates, a click lands on whatever drew the
`(key)` under the pointer, and Ctrl-C jacks out. On a real terminal
the runtime flips to raw input with mouse reporting on the alternate
screen and restores everything on exit; piped, the same bytes replay
the same session — which is exactly how the conformance corpus pins
it. Views and slots don't change at all: `examples/jack-in.ml` is the
operator console below with one glyph changed, `▶` → `⏵`.

The showcase is `examples/construct.ml`, the Nebuchadnezzar's operator
console — line edit, buttons, checkbox, progress bar, item list, status
bar, all live (`printf '+⏎n Trinity⏎j⏎r⏎q⏎' | mlang run
examples/construct.ml`, or weld it into a standalone binary like any
other program):

```
┌─ Nebuchadnezzar — operator console ──────────┐
│ Wake up, Neo…                                │
│ ──────────────────────────────────────────── │
│ Operator: Neo▁▁▁▁▁▁▁▁▁ (n)                   │
│ [ Jack in ](j)  [ ] Red pill (r)             │
│ ──────────────────────────────────────────── │
│ Signal strength                              │
│ ▓▓▓▓▓▓░░░░░░░░░░░░░░ 30%  [ + ](+)  [ − ](-) │
│ ──────────────────────────────────────────── │
│ Crew aboard                                  │
│ • Morpheus                                   │
│ • Trinity                                    │
│                                              │
│ [ Exit ](q)                                  │
└──────────────────────────────────────────────┘
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
std/ui.ml         the Construct — the UI library, also written in MLang
conformance/      cases.json + expected.json: 154 recorded goldens, the
                  language's observable ground truth (RECORD=1 to re-record)
bench/            the self-repair benchmark — the conformance corpus doubles
                  as a labeled bug generator (see bench/README.md)
docs/             the deadlock demo: the animated SVG + the Python twin
examples/         runnable programs (mandelbrot, calc, editor, deadlock, …)
SPEC.md           the full language specification
```

## Honest notes

* **Glyphs vs. tokens.** MLang optimizes *characters* — the number of
  atoms a model must emit correctly — not today's BPE token counts, and
  on those it currently loses: rare Unicode glyphs cost 2–3 BPE tokens
  each. Measured with `bench/tokens.py`:

  | program | chars | o200k tokens | cl100k tokens |
  |---|---|---|---|
  | `fizzbuzz.ml` | 115 | 78 | 79 |
  | `fizzbuzz.py` (natural port) | 183 | 63 | 63 |
  | `pipeline.ml` | 369 | 140 | 143 |
  | `pump_pipeline.py` (natural port) | 567 | 145 | 145 |

  A dedicated tokenizer would make one-glyph-one-op literally one token
  per operation (the mapping is trivial by design), but no deployed model
  has one. The density argument that *does* hold today is correctness
  density: there is no syntax to misindent and no identifier to misspell
  twice — see the mutation-robustness table above for what that buys,
  and what it costs.
* **Determinism over raw parallelism.** The runtime interleaves strands
  deterministically (round-robin, fixed slice) rather than using OS
  threads. The language model (immutable values, channel-only
  communication) is exactly the one that admits true parallel execution
  without data races; the scheduler is an implementation choice, and
  reproducibility is worth more to a machine author than nondeterministic
  wall-clock wins. A parallel scheduler can be added without changing a
  single program's meaning where interleaving is unobservable.
* **The benchmark's limits.** The self-repair corpus programs are small,
  and the Python control arm is 28 hand-verified ports, not all 94
  programs. Models have seen enormous amounts of Python and essentially
  zero MLang — the MLang arm leans entirely on an op-reference primer in
  the prompt. `bench/README.md` spells out the protocol and every caveat.

## Name

**M**atrix **Lang**uage. The rain falls in columns. The columns compute.
