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
| `⌨` | `→ s \| ∅` | read a line of stdin; `∅` at EOF |
| `⍟` | `→` | dump this strand's stack to stderr |
| `⌂` | `→ L` | the program's command-line arguments, a list of strings |
| `⍇` | `path → s` | read the file at `path` into a string; unreadable path glitches |
| `⍈` | `s path →` | write string `s` to the file at `path`; failure glitches |

Command-line arguments and the file system are part of a run's *input*:
determinism means identical program, stdin, arguments, and file contents
produce an identical run. `⌂` sees the arguments after the source file
(`mlang run prog.ml a b`) or, in a welded binary, everything after the
executable name — which is how a file dropped onto a built editor arrives
as its argument. `⍇`/`⍈` glitch with a stable message that names only the
path, never an OS error string, so failures are reproducible byte for
byte.

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

## 7. Exit status

`0` — all strands completed. `1` — at least one uncaught glitch, or
deadlock. `2` — load (weave) error; nothing executed.

## 8. Design lineage

Concatenative core from Forth and Joy (quotations as the sole control-flow
mechanism); glyph vocabulary in the spirit of APL; concurrency from CSP and
Erlang (channels, share-nothing isolation, let-it-crash strands); the grid
from punched tape, transposed to fall like rain.
