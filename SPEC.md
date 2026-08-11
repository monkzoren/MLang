# The MLang Specification

Version 0.1. This document is normative for the MLang toolchain in
`compiler/` and is written so that an independent implementation can be
built from it without language changes. The observable behavior it
describes is additionally pinned, byte for byte, by the recorded
conformance corpus in `conformance/` — any implementation must reproduce
those goldens exactly.

## 1. Model

An MLang **program** is a grid of Unicode code points. The grid contains
**strands**: independent machines that execute their own glyph sequence in
order, concurrently with all other strands. Each strand owns:

* a **data stack** of immutable values,
* a set of **strand-local bindings** (`⇒`),
* a **frame stack** (its execution state).

Strands share exactly two things, both mediated by the runtime:

* **global bindings** — single-assignment, created by `≔`;
* **channels** — named unbounded FIFO queues. `↥c` (send) never blocks;
  `↧c` (receive) blocks until a value is available.

There is no other communication. There are no references: every value is
immutable and conceptually copied. Consequently data races cannot be
expressed in the language.

### 1.1 Values

| type | literal | notes |
|---|---|---|
| int | `42`, `¯5` | arbitrary precision; negatives use `¯`, never `-` |
| float | `2.5`, `.5`, `¯1.25` | IEEE double |
| string | `«text»` | any glyphs except `»`; `⏎` denotes newline |
| list | `⟨1 «a» ⟨2⟩⟩` | immutable, heterogeneous, nestable |
| quotation | `[∂×]` | unexecuted code as a value |
| nil | `∅` | the absent value; conventional end-of-stream sentinel |

**Truthiness**: `0`, `0.0`, `«»` (empty string), `⟨⟩` (empty list) and `∅`
are false; everything else is true. Comparison operators push `1` or `0`.

**Rendering**: printing formats negatives with `¯`, strings bare, lists as
`⟨…⟩` with strings quoted `«…»`, nil as `∅`.

## 2. Source forms

Source is UTF-8 text. Tab characters are a load error anywhere ("tabs break
the grid"). Each code point is one grid cell (implementations are not
required to handle combining sequences or double-width rendering; authors
should avoid combining marks).

### 2.1 Flat form (linear — the authoring form)

Any file not starting with `⇓` is in flat form. **Each line is one
strand**, executed left to right. Blank lines are ignored. A line whose
content is exactly `⇊` is the divider: lines above it are concatenated into
the boot strand. A line starting with `⋮` continues the previous strand.
Flat form is the intended authoring form — text generators emit lines.

### 2.2 Rain form (vertical — the presentation form)

A file whose first line consists of `⇓` is in rain form. The remaining
lines form a grid. **Each column of the grid is one strand**, executed
top to bottom. Columns containing only blanks are ignored (use them as
gutters). Blank cells within a column act as separators (§3.1).

A grid row whose first non-blank glyph is `⇊` is the **divider**: rows
above it are the **boot section**, rows below it are the strands. The boot
section's columns are concatenated left-to-right into a single boot strand.

Rain and flat forms are transposes of each other and are semantically
identical; engines must accept both. Tooling (`mlang rain`, `mlang flat`)
converts losslessly (strings that span column boundaries in a rain boot
section are the one caveat; keep boot strings within one column or author
boot sections flat).

### 2.3 Comments

`※` begins a comment running to the end of the physical line (flat form)
or the end of the column (rain form). A strand that lexes to zero
instructions (e.g. a comment-only line) is not a strand and does not
receive a strand id.

## 3. Lexing

Each strand's cell sequence is lexed independently.

1. Blank cells are skipped; they terminate number runs.
2. A maximal run of digits, optionally preceded by `¯` and containing at
   most one `.`, is a number literal. A lone `¯`, a stray `.`, or two `.`
   in one run is a load error.
3. `«` begins a string, closed by the next `»` (load error at end of strand
   if unclosed). Inside, `⏎` becomes a newline; all other glyphs stand for
   themselves.
4. `[` … `]` delimit a quotation (nestable; unbalanced brackets are load
   errors). `⟨` and `⟩` compile to list-building instructions.
5. `≔ ⇒ ↥ ↧ ⇂ ⇈ ⇟` consume the next non-blank glyph as their argument;
   `⇉` consumes the next two (source channel, then destination channel).
   Argument glyphs must not be reserved (an operation, digit, or
   structural glyph) — load error otherwise.
6. Any glyph in the operation table (§5) is that operation.
7. Any other glyph is a **name reference**, resolved at execution time:
   strand-locals first, then globals. Referencing an unbound name is a
   glitch. If the bound value is a quotation it is executed; otherwise it
   is pushed.

Load errors abort the run before execution with exit code 2 and the
1-based `row:col` position in the original file.

## 4. Execution

### 4.1 Program start

1. All strands are lexed. Load errors prevent any execution.
2. If a boot section exists, the boot strand runs (as strand id `¯1`)
   until it and anything it spawned finish. A boot glitch or deadlock
   aborts the program.
3. The main strands start together, numbered `0, 1, 2, …` in grid order
   (left-to-right in rain form, top-to-bottom in flat form). `≣` is the
   count of main strands; `⍳` is the executing strand's id.

### 4.2 Scheduling

The scheduler is deterministic: strands are stepped round-robin in id
order, up to a fixed slice of 8 instructions per turn. `⌛` ends the
current slice voluntarily. Identical program + input must produce an
identical run, including output interleaving. (Implementations may use
true parallelism only where it cannot be observed.)

A strand blocks on `↧` of an empty channel or `⋈` of an unfinished strand.
Blocked instructions re-execute when the strand resumes; a blocking
operation must therefore not consume operands before it commits. If at any
point every live strand is blocked, the runtime reports the full wait graph
(strand, what it awaits, coordinates) and exits 1. Programs never hang on
an internal deadlock.

Reading stdin has the **lowest scheduling priority**: a strand executing
`⌨` (or `⌥`) waits until no other strand can make progress (each is
blocked, waiting on stdin itself, or finished) before the read happens.
An interactive pipeline therefore flushes all pending work — greetings,
prompts, responses — before the program waits on the user, and the
interleaving remains deterministic because it never depends on input
timing. (A strand waiting its turn at stdin is not deadlocked — its read
can always proceed once the grid goes quiet.)

The toolchain additionally offers an opt-in **parallel scheduler**
(`mlang run --parallel`; welded binaries honor the `MLANG_PAR=1`
environment variable): every strand runs on its own OS thread, sharing
only what the language itself shares — channels and single-assignment
globals. Per-strand execution order, FIFO channel delivery per sender,
blocking semantics, glitch isolation, deadlock detection, and exit codes
are all preserved, and output is atomic per line. What is *not* preserved
is cross-strand interleaving: the order output from different strands
mixes, the outcome of `⇂`, and id assignment among concurrently spawning
strands follow real thread timing and vary run to run. (Stdin priority
carries over naturally: a strand reading `⌨` blocks on the OS while the
other threads run on.) The deterministic scheduler remains the language
default and the conformance corpus's ground truth. Programs whose
channels each have one sender and one receiver and that print from a
single strand observe no difference — their parallel output is
byte-identical to the deterministic run.

### 4.3 Spawning

`⚡` pops a quotation and starts it as a new strand with an empty stack and
a *copy* of the spawner's locals, returning the new strand's id (ids
continue after the main strands). `⋈` blocks until the given strand id has
finished (normally or by glitch).

### 4.4 Streams

`∅` is the conventional end-of-stream marker on channels, and the stream
combinators build it in: `⇈` (pour) sends a whole list then `∅` in one
step; `⇉` (pump) receives from its source one value at a time — blocking
as needed — runs its body on each value, sends the single result to its
destination, and on receiving `∅` forwards it and stops; `⇟` (drain)
receives until `∅` and pushes the collected list. Pump bodies interleave
with other strands like any code; one pumped value is processed per
iteration. If a pump's body glitches uncaught, the pump dies with its
strand and does **not** forward `∅` — downstream stages then show up in
the deadlock report, pointing at the broken stage.

### 4.5 Glitches

A **glitch** is a fault carrying an arbitrary value — raised by the runtime
(with a message string) or by `↯` (with any value). Runtime glitch sources
include: stack underflow, type mismatches, `÷`/`%` by zero, `√` of a
negative, out-of-bounds `@`, unparseable `⍎`, unbound names, and rebinding
a global.

`[body] [handler] ⍥` runs `body`; if a glitch reaches the `⍥`, the stack
is truncated to its depth at entry, the glitch value is pushed, and
`handler` runs. Success disarms the handler. An uncaught glitch kills only
its strand; the runtime records the report (strand id, label, `row:col`,
value), other strands continue, and the process exits 1.

### 4.6 Fault reports

Every fault report is written for a machine reader that cannot count
columns. The first line of each report keeps the fixed shape shown above
(`✗ glitch …`, `✗ deadlock …`, `✗ weave error …`). It is followed by a
**source excerpt**: the offending physical line, windowed to at most 61
glyphs around the fault (`…` marks a trimmed side), prefixed `  row│ `,
with a caret line beneath marking the exact glyph and repeating the
coordinates. A glitch report then adds the **call chain** — one `  in X, called at
row:col` line per active named definition, innermost first, so a fault
inside a definition or a library word names its caller — followed by the
strand's **stack as the fault left it** (`  stack: …`, deepest first,
capped to the topmost eight values and to a readable width each,
`(empty)` when bare). Deadlock reports add an excerpt for every waiting
strand and then a **channel census**: any channel with send sites but no
receive sites (or the reverse) is named as a `⚠` line, since a channel
that can never complete a handoff is almost always a misspelled or
renamed name. Weave errors carry the same excerpt treatment, and a
`]` without an opener reports the strand's bracket tally.

Positions in woven library code report with the library's own file name
and coordinates (`std.ml 26:7`, `ui.ml 3:12`) and excerpt the library's
source, never a coincidental program line. `mlang build` welds the
program's source lines into the binary, so a standalone executable's
reports carry the same excerpts. All of this output is part of the
language's deterministic, conformance-pinned behavior.

## 5. Operations

Stack effects are written `inputs → outputs`, top of stack rightmost.
`X` marks operations that consume the following glyph as an argument.

### Stack
| glyph | effect | |
|---|---|---|
| `∂` | `a → a a` | dup |
| `⇅` | `a b → b a` | swap |
| `⌫` | `a →` | drop |
| `⊚` | `a b → a b a` | over |
| `⥀` | `a b c → b c a` | rot |
| `≢` | `→ n` | depth |

### Arithmetic
| glyph | effect | |
|---|---|---|
| `+` `-` `×` | `a b → r` | add, subtract, multiply |
| `÷` | `a b → r` | division; int÷int stays int when exact, else float; ÷0 glitches |
| `%` | `a b → r` | modulo; %0 glitches |
| `^` | `a b → aᵇ` | power |
| `√` | `a → r` | square root (float); negative glitches |
| `⌊` `⌈` | `a → n` | floor, ceiling (to int) |
| `±` | `a → −a` | negate |

### Comparison & logic (push `1`/`0`)
`=` `≠` (any values, deep on lists) · `<` `≤` `>` `≥` (two numbers or two
strings; otherwise glitch) · `∧` `∨` `¬` `⊻` (truthiness).

### Control
| glyph | effect | |
|---|---|---|
| `!` | `[q] → …` | apply a quotation |
| `?` | `c t e → …` | if `c` truthy take `t` else `e`; quotations run, other values are pushed |
| `⟳` | `[c] [b] → …` | while: run `[c]`; while it leaves truthy, run `[b]` |
| `⍣` | `n [b] → …` | run `[b]` n times |

### Iteration (over a list, or a string as 1-char strings)
| glyph | effect | |
|---|---|---|
| `∵` | `L [f] → L′` | map |
| `∀` | `L [f] → …` | each (for effects) |
| `⌿` | `L [f] → L′` | filter |
| `⍀` | `L a [f] → a′` | fold; `[f]` sees `acc item → acc′` |
| `⍸` | `n → ⟨0…n−1⟩` | range |

### Sequences (strings & lists)
| glyph | effect | |
|---|---|---|
| `#` | `s → n` | length |
| `⧺` | `a b → ab` | concat (two strings or two lists) |
| `@` | `s i → v` | 0-based index; out of bounds glitches |
| `⊂` | `s i j → s′` | slice `[i, j)`, clamping |
| `⊆` | `s sep → L` | split; empty sep splits into characters |
| `⊇` | `L sep → s` | join (items stringified) |
| `⍕` | `v → s` | to string |
| `⍎` | `s → n` | parse number; glitches if malformed |
| `⌗` | `c → n` | code point of 1-char string |
| `⍘` | `n → c` | 1-char string from code point |

### Bindings
| glyph | effect | |
|---|---|---|
| `≔X` | `v →` | bind `v` to global `X`; rebinding glitches |
| `⇒X` | `v →` | store into strand-local `X`; rebindable |
| `X` | `→ …` | reference: run if quotation, else push |

### Strands & channels
| glyph | effect | |
|---|---|---|
| `↥X` | `v →` | send `v` on channel `X` (never blocks) |
| `↧X` | `→ v` | receive from `X` (blocks) |
| `⇂X` | `→ v 1 \| 0` | try-receive |
| `⇈X` | `L →` | pour: send each item of a list/string on `X`, then `∅` |
| `⇉XY` | `[f] →` | pump: for each value from `X` run `f` (`v → v′`), send to `Y`; on `∅`, forward it and stop |
| `⇟X` | `→ L` | drain: collect from `X` until `∅` into a list (blocks) |
| `⚡` | `[q] → id` | spawn |
| `⋈` | `id →` | join |
| `⍳` | `→ id` | this strand's id (boot: `¯1`) |
| `≣` | `→ n` | number of main strands |
| `⌛` | `→` | yield the scheduler slice |

### Glitches
| glyph | effect | |
|---|---|---|
| `⍥` | `[b] [h] → …` | try/catch (§4.5) |
| `↯` | `v →` | raise `v` |

### I/O
| glyph | effect | |
|---|---|---|
| `⍞` | `v →` | print with newline |
| `⊸` | `v →` | print without newline |
| `⌨` | `→ s \| ∅` | read a line of stdin without its terminator (LF or CRLF — Windows line endings never reach the program); `∅` at EOF; runs only once every other strand is quiet (§4.2), with pending `⊸` output flushed first, so prompts appear |
| `⌥` | `→ e` | read one input event (§5.1): a key, or a mouse press `⟨«⌖» x y⟩`; `∅` at end of input; same lowest scheduling priority as `⌨` |
| `⍇` | `path → s` | read a whole file as a string; failure glitches `⍇ cannot read «path»` |
| `⍈` | `s path →` | write string `s` to a file; failure glitches `⍈ cannot write «path»` |
| `⍟` | `→` | dump this strand's stack to stderr |
| `⌂` | `→ L` | the program's command-line arguments, a list of strings |
| `⍜` | `→ ⟨rows cols⟩` | the terminal size; `⟨24 80⟩` when there is no terminal |

Command-line arguments and the file system are part of a run's *input*:
determinism means identical program, stdin, arguments, and file contents
produce an identical run. `⌂` sees the arguments after the source file
(`mlang run prog.ml a b`) or, in a welded binary, everything after the
executable name — which is how a file dropped onto a built editor arrives
as its argument. File-operation glitch messages name only the path, never
an operating-system error string — they are part of the language's
deterministic, conformance-pinned output.

### 5.1 Input events

`⌥` parses the standard input byte stream into one event per call:

* A printable key arrives as a one-character string («a», «é», …).
* Enter is `«↵»`, tab `«⇥»`, backspace `«⌫»` (BS or DEL), delete
  `«⌦»`; the arrow keys are `«↑» «↓» «←» «→»`. (Enter is deliberately
  not `⏎` — inside string literals that glyph denotes a newline, so an
  event named `«⏎»` could never be written or compared.)
* Enter is `«↵»`, and Home/End/PgUp/PgDn/Insert arrive as
  `«⇱» «⇲» «⇞» «⇟» «⎀»`.
* Any other control character arrives as a caret-notation chord:
  Ctrl-C is `«^C»`, Ctrl-S `«^S»`.
* The bytes `⎋[` open a CSI sequence. An SGR mouse press becomes
  `⟨«⌖» column row⟩` (1-based). Release, wheel, and motion reports, and
  unrecognized sequences, are consumed silently — `⌥` keeps reading
  until it has a deliverable event. An escape byte not followed by `[`
  is delivered as `«⎋»`, and the byte after it is kept for the next
  event.
* End of input is `∅`, including inside an unfinished sequence.

The mapping is a pure function of the byte stream, so a recorded pipe
replays exactly what a live terminal produced. When a program that
executes `⌥` runs with stdin and stdout on a real terminal, the runtime
— not the program — switches the terminal to raw input with SGR mouse
reporting on the alternate screen for the duration of the run, and
restores it afterwards. None of that scaffolding appears in the
program's own output, which stays byte-identical to a piped run.

## 6. The standard library

The standard library is written in MLang (`std/std.ml`, printed by
`mlang std`) and is woven into every program: its definitions execute at
the start of the boot strand, before the program's own boot section. Every
entry is an ordinary `≔` global, so std sigils behave exactly like user
definitions — including that rebinding one glitches with «already
defined». Names resolve late, at call time.

| sigil | | sigil | |
|---|---|---|---|
| `π` `τ` `ℯ` | circle constants, Euler's number | `∞` | positive infinity |
| `n∣` | absolute value | `a b⟌` | greatest common divisor |
| `a b⊓` / `a b⊔` | minimum / maximum | `n‼` | factorial |
| `L∑` / `L∏` | sum / product | `L µ` | mean (`⟨⟩µ` glitches) |
| `s⊃` / `s⌷` | head / last | `s⍫` | tail |
| `s n⊤` / `s n⊥` | take / drop first n | `s⍒` | sort descending |
| `A B⍚` | zip to pair list | | |
| `s⇑` / `s⇩` | upper/lowercase (ASCII) | `s⍭` | words (split, drop empties) |
| `s⍖` | split into lines | | |

Library internals use fullwidth letters (`ａ ｂ ｘ`) as strand-locals;
programs should treat those as reserved.

The engine provides five primitives the library builds on (part of the
operation set, §5): `⍙` type-of, `⌽` reverse, `⍋` sort, `∈` contains,
`⍷` find. Transcendental functions (log, exp, trig) are deliberately
absent for now: MLang guarantees bit-identical runs across engines, and
platform `libm` implementations are not correctly-rounded — they enter
the library only alongside a correctly-rounded implementation.

### 6.1 Bundled libraries

Beyond std, the toolchain bundles further libraries written in MLang.
There is no import statement: a bundled library is woven into the boot
strand — after the standard library, before the program's own boot
section — **exactly when the program references at least one sigil the
library defines without defining that sigil itself** (with `≔` or `⇒`,
anywhere in the program, including inside quotations). Only name
references count; using a library sigil as a channel or binding argument
(`↥Ⓛ`, `⇒Ⓛ`) does not trigger the weave. The decision is made at
compile time, so a welded binary carries exactly the libraries its
program uses.

Weaving a library reserves all of its sigils, exactly as std's are
reserved: a program that pulls in a library and also tries to `≔` one of
that library's sigils glitches with «already defined». A program that
defines a library sigil itself and never references the library's others
is left alone — its own definition wins and nothing is woven.

One library is currently bundled: **the Construct** (`std/ui.ml`,
printed by `mlang ui`) — a widget toolkit in the lineage of Qt/PySide.
Public sigils: `Ⓛ` label, `Ⓑ` button, `Ⓔ` line edit, `Ⓒ` checkbox,
`Ⓟ` progress bar, `Ⓘ` item list, `Ⓢ` separator, `Ⓥ`/`Ⓗ` vertical and
horizontal layouts, `Ⓦ` window, `⌺` draw, `▶` event loop, `◼` quit,
`✎` status bar. Widgets are immutable tagged lists; interactive widgets
carry a **slot** (a quotation) that `▶` runs in the application's own
strand when the widget's key arrives as the first word of a stdin line
(line edits receive the rest of the line on the stack). A glitch in a
slot is caught by the loop and shown as a `✗` status message. The
Construct's internals use circled-lowercase sigils (`ⓐ ⓑ …`) and
additional fullwidth-letter strand-locals; programs must treat both as
reserved, and must not nest `▶` or `⏵` inside a slot. Its observable
behavior is pinned by the conformance corpus like everything else.

The Construct has two event loops, one widget model. `▶` (scripted)
reads stdin lines as above. `⏵` (live) reads `⌥` events instead:
`⇥`/`↓`/`→` and `↑`/`←` move focus through the keymap in layout order
— the focused widget draws `⟦ ⟧` instead of `[ ]`, a focused line edit
shows a `▏` caret — `«↵»` or space activates, printable keys type into
the focused line edit (its slot runs after every keystroke with the new
text), `«⌫»` deletes, any other key activates the widget carrying that
mnemonic, and a mouse press `⟨«⌖» x y⟩` lands on the widget that drew
the `(key)` affordance under the pointer. `«^C»`, `«⎋»`, end of input,
or `◼` in a slot ends the loop. Each `⏵` frame is preceded by the
cursor-home and clear-screen escape sequences. Views and slots are
identical under both loops: a scripted application becomes a live one
by changing that one glyph.

## 7. Exit status

`0` — all strands completed. `1` — at least one uncaught glitch, or
deadlock. `2` — load (weave) error; nothing executed.

## 8. Design lineage

Concatenative core from Forth and Joy (quotations as the sole control-flow
mechanism); glyph vocabulary in the spirit of APL; concurrency from CSP and
Erlang (channels, share-nothing isolation, let-it-crash strands); the grid
from punched tape, transposed to fall like rain.
