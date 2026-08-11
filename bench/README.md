# bench — the MLang self-repair benchmark

The conformance corpus doubles as a benchmark generator. Every corpus
program has a recorded golden — stdout, stderr, and exit code, byte for
byte — so a mechanically seeded bug is *born labeled*: the exact correct
output is known in advance, and a repair is verified against the identical
failure, because the runtime is deterministic. No flaky reruns, no judge
model, no rubric — a mutant is healed when the bytes match.

Two benchmarks share the mutation engine:

* **`heal.py`** — the self-repair loop. An LLM sees the broken program,
  the byte-exact expected output, and the failure exactly as the runtime
  announced it (a weave error with coordinates, a glitch with coordinates,
  a proven deadlock wait-graph — or, in the Python control arm, a
  traceback / wrong bytes / a hang note). It replies with a corrected
  program; the harness re-runs it; up to N rounds.
* **`robustness.py`** — no LLM. Seed hundreds of one-edit mutations and
  bucket what each becomes: caught before running, caught loudly at
  runtime, proven deadlock, **silent wrong output** (the dangerous
  bucket), or hang. Run on both arms with the same operator classes.

## The two arms

The MLang arm mutates the exit-0 conformance programs directly. The
Python control arm mutates `python_ports/` — hand-written natural Python
translations of 28 of those programs (threads + queues for the channel
cases), each with its own recorded golden (`record_ports.py` re-records).
Both arms get the same four one-edit operator classes, with string
literals and comments masked so every mutation lands in code:

| MLang | Python |
|---|---|
| swap one op glyph | swap one operator/keyword token |
| drop one glyph | drop one token (incl. an indent level) |
| transpose adjacent glyphs | transpose adjacent tokens |
| rename one channel use | rename one identifier occurrence |

## The application-scale arm: the Oracle

The corpus programs are small, so the benchmark also targets one
deliberately complicated application: `examples/oracle.ml` (**the
Oracle** — a 7-strand concurrent MapReduce analytics engine with a
work-stealing mapper pool, an actor-style state owner, per-command
crash-isolated parsers, and a two-phase channel protocol) against
`python_ports/oracle.py`, the same architecture in natural Python
threads + queues. Both are driven by the same recorded query session;
the MLang side is conformance-pinned. This is the arm where the
languages' failure modes actually diverge: in the Python twin a
one-token bug regularly hangs the process (often *after* printing a
thread traceback), while MLang converts that entire class of bug into
proven deadlock reports with a wait graph — see the app-scale tables in
the top-level README.

## Running it

```sh
# the robustness table (no LLM, ~15 s)
python3 bench/robustness.py

# the self-repair benchmark
python3 bench/heal.py --arm both --provider claude-cli \
    --model claude-haiku-4-5-20251001 --mutants 80 --rounds 3

# the application-scale arms (the Oracle only)
python3 bench/robustness.py --cases example:oracle.ml,oracle \
    --tag oracle --seeds-mlang 250 --seeds-python 250
python3 bench/heal.py --arm both --cases example:oracle.ml,oracle \
    --tag oracle --mutants 40 --seeds 60 --rounds 3

# render the tables from whatever results exist
python3 bench/report.py
```

Providers for `heal.py`: `claude-cli` (headless `claude -p`, uses your
Claude Code login), `anthropic` (`ANTHROPIC_API_KEY`), `openai`
(`OPENAI_API_KEY`), or `cmd:<shell-command>` (prompt on stdin, completion
on stdout — plug in anything).

Protocol note: the failure report shown to the model is whatever the
language's runtime printed (MLang reports carry source excerpts with
carets, the stack at the fault, and proven wait graphs — SPEC §4.6;
Python gets its tracebacks), plus one harness-computed hint appended
identically in both arms: the first line where stdout diverged from the
golden. Improving MLang's reports moved the Oracle arm from 65% to 82% healed
under the identical protocol — the benchmark is the language's feedback
loop, not just its scoreboard.

**Measure the noise floor before believing a delta.** The model is
sampled, not deterministic: two runs of the *identical* configuration
(same mutants, same toolchain, same prompt) disagree on ~6 of 40
individual mutants and land ~2 apart in aggregate, i.e. **≈5 points at
n=40**. A single-run difference smaller than about 3 mutants means
nothing. Two ways to get signal anyway: raise n, or predict *which*
mutants a change should fix and check those specifically — the second
round of diagnostic work here produced no aggregate movement while
reproducibly fixing all four failures it was designed for. Result files
carry every transcript, so per-mutant comparison across runs is a
`json.load` away. Results are committed under `results/`;
`report.md` there is the rendered summary. `tokens.py` (needs
`pip install tiktoken`) measures the char-vs-BPE-token table quoted in
the top-level README's honest notes.

## What the numbers do and don't say

* The corpus programs are small (a line to a screenful). This measures
  repair of localized bugs in working programs — the agent inner loop —
  not greenfield program synthesis.
* Models have read a lot of Python and almost no MLang; the MLang arm
  leans on the op-reference primer in the prompt. That asymmetry is part
  of what's being measured — a language that models can repair *without
  training data* is exactly the claim under test.
* A mutant is healed only if output matches byte for byte. A semantically
  reasonable fix with different formatting counts as a failure, in both
  arms.
* The Python `hang` bucket usually prints a thread traceback before
  freezing; the process still never exits. MLang cannot hang on a blocked
  channel — the scheduler proves the deadlock and reports the wait graph
  — but a seeded busy-loop still hangs both languages, and is counted
  against both.
* The application arm is one application in one architecture
  (channel-heavy MapReduce + actor). Programs are presented to the model
  exactly as committed — the MLang side includes its comment header, the
  Python side is bare idiomatic code. A hang hands the model whatever
  partial output appeared before the timeout; that asymmetry (wait graph
  vs. frozen partial traceback) is precisely the thing under test.
