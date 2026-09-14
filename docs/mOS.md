# mOS — the grid as the operating system of a machine that learns

A thought experiment, written against what the loom (SPEC §4.7,
[`loom.md`](loom.md)) can actually do today. Everything in the first
three sections exists and is conformance-pinned. Section 4 is the part
that does not exist yet, and it is the part that matters.

## 0. The premise

Suppose an LLM is fabricated into silicon: a large, capable model at an
absurd token rate, fixed weights, no persistent memory. Put it in a
robot. The obvious architecture — the model *is* the controller, every
tick is a forward pass — has two problems that speed does not fix:

* **Nothing learned survives.** The weights are literally etched.
  Whatever the machine figured out on Tuesday is gone on Wednesday
  unless it lives somewhere outside the model.
* **A sampled distribution between a sensor and an actuator is the
  wrong thing** regardless of how fast it samples. You do not want the
  inner control loop of a thing with hands to be stochastic.

The proposal: the ASIC is the engine, an MLang grid is the operating
system. The grid drives; the model rewrites the grid, through the loom,
while it runs. The machine learns by having its OS re-woven.

## 1. Which memory this is

"No persistent memory" is two problems, and this solves one of them.

* **Episodic memory** — *the counter is 91 cm; do not stack the blue
  plates.* Facts. Code is a bad medium for facts; a key-value store is a
  good one, and always has been. A `≔` per fact would be silly.
* **Procedural memory** — *how to grip a wet glass; what to check first
  when a door will not open; what to do when the left caster stalls.*
  Skills, policies, reflexes. This is the open problem. Fine-tuning is
  slow and destructive; stuffing retrieved notes into the prompt makes
  a longer prompt, not a changed reflex.

The loom is a **procedural-memory consolidator**. That is a narrower
claim than "the grid is the robot's memory" and a stronger one, because
consolidating procedure is the part nobody has a good answer for.

## 2. Two rates, one seam

The architecture is a two-rate system, and it falls out of the loom
rather than being bolted onto it.

```
   ┌──────────────────────────────────────────────────────────┐
   │  ASIC  (slow relative to control; fast relative to a day) │
   │   pull /.loom  ·  read /.loom/faults  ·  propose a patch  │
   └───────────────┬───────────────────────────▲──────────────┘
                   │ POST /.loom               │ faults, coordinates,
                   ▼ (applied at a seam)       │ /.loom/log
   ┌──────────────────────────────────────────────────────────┐
   │  grid  (deterministic; strands = sensor / plan / motor)   │
   │   [perception]⇉αβ   [planner]⇉βγ   [↧γ …actuate…]⟳        │
   └──────────────────────────────────────────────────────────┘
```

* **The grid is what drives.** Deterministic scheduler, share-nothing
  strands, faults as values with grid coordinates, deadlocks proven and
  reported. It has its hands on the motors.
* **The model rewrites the grid.** It never sits between a sensor and an
  actuator. It reads what went wrong, proposes new code, and weaves it
  in.

What makes the loom the right hot-reload for this and not just *a*
hot-reload is the **seam**. A strand is re-woven only at the boundary
between two iterations of its outermost loop — parked at `⎆` waiting
for input, or a top-level `⟳` about to test its condition. So the
machine re-learns *between actions*, never mid-grasp. This is not a
metaphor for sleep consolidation; it is the same trick, mechanically:
do not rewrite the behavior while the behavior is executing.

Four properties the loom already has do most of the work:

**`⟲` carries state across the re-weave.** A replaced strand resumes
inside its new outermost loop with stack and locals intact, and a
`⟲` line runs once at the seam to reshape them. The machine learns a
better way to do something without forgetting where its arm currently
is. (In [`loom.md`](loom.md) §2 a counter gains a second counter seeded
from the first, mid-run.)

**A dead strand is a seam.** A subsystem glitches; the served grid does
not exit. It holds its port, answers 503 naming the dead strand, and the
next patch brings it back to life with its locals intact. A machine
whose perception stack dies *degrades* instead of bricking, and the fix
arrives as data on the same channel as everything else.

**Hot definitions must be pure.** A `≔` rebinding is hot only when its
expression evaluates from an empty stack, with no effects, within a step
budget, to exactly one value. That is an accidental safety property: the
model can hot-rebind only things it can *evaluate*. Anything with a
consequence goes through the strand path, which goes through a seam,
which is versioned and observable.

**Library sigils cannot be rebound.** `std`, `ui`, `json` and any woven
library are frozen for the life of the run. That is an immutable kernel
— the seed of a constitution the machine cannot patch out of itself,
and it already exists.

## 3. The loop as it could run today

Every step below is a route or a command that exists.

1. The grid runs. Faults accumulate at `GET /.loom/faults`, each with
   the version it came from and the line excerpted (`v3 20:40`).
2. At a quiet moment the model runs `mlang pull`, reads the faults,
   edits, and `mlang patch`es. A weave error is refused with 422; a
   change to effectful boot code is refused with 422; a line the live
   grid changed underneath it is refused with 409 and the live lines.
   Nothing changes on a refusal.
3. The patch lands at each affected strand's next seam. `GET /.loom/log`
   shows what it changed; `GET /.loom/vN` keeps every ancestor.
4. If the patch kills a strand, the grid holds its port and step 1
   already has the report.

That is a machine that can be mended forever without a restart, by any
number of agents at once. It is not yet a machine that *learns*.

## 4. The missing piece: is the patch better?

The loom proves a patch is *well-formed*: it weaves, it merges, it
cannot deadlock. It cannot prove a patch is an *improvement*. The
[self-repair benchmark](../bench/) got away with this because every
mutant had a recorded golden. A robot in a kitchen has no golden.

Without a fitness signal, a model patching its own OS a million times
does not evolve, it drifts — and it drifts fastest exactly where the
model is most confident and most wrong.

The substrate for fixing this is already here, and it is the same
substrate that pins the conformance corpus: **runs are deterministic,
and patches travel in the input stream.** The sensor stream *is* the
input. So a recorded window of real input can be replayed — as `⟡`
frames on stdin, the replay transport of SPEC §4.7 — against the live
version and against the candidate, and the two outputs diffed byte for
byte. That is a shadow evaluation of a proposed reflex against lived
experience, with no robot and no risk.

The loop that would actually learn:

1. Grid runs; faults and coordinates accumulate.
2. Model pulls, reads faults, proposes v(N+1).
3. **Replay gate.** v(N) and v(N+1) each replay a recorded window of
   real input. The candidate ships only if it strictly improves on the
   window it was written to fix and changes nothing in a pinned
   behavioral corpus — the machine's own conformance suite, grown from
   its history the way `conformance/` grew from ours.
4. **Auto-revert.** After ship, a fault-rate regression over the next
   window rolls the grid back to v(N), which `/.loom/vN` already holds.

Steps 1 and 2 and the replay transport exist. Steps 3 and 4 are the
work, and they are where this stops being a thought experiment.

## 5. Two things that bite

**`--parallel` refuses patches.** The loom requires the deterministic
scheduler. A robot wants real parallelism across its sensor pipelines.
Today you choose: a deterministic single-scheduler grid with a
throughput ceiling, or a parallel grid that cannot be re-woven. What a
seam *means* under a parallel scheduler is the single most load-bearing
open question for this design.

**Line-wise `diff3` is a weak conflict detector for self-modification.**
The merge is right for its purpose — humans, or agents, editing
different concerns. When one agent rewrites its own behavior
repeatedly, "different lines" stops implying "independent changes":
two patches can merge cleanly, be coherent line by line, and compose
into an incoherent policy. The replay gate of §4 is the honest answer;
the merge cannot be.

## 6. What this is not

It is not a claim that MLang should be a robotics language, and not a
claim that a grid is a better policy than a network. It is the
observation that the loom's constraints — patch only at a seam, hot
only if pure, kernel frozen, every version kept, every run replayable —
are exactly the constraints you would want on a machine that edits its
own operating system while it runs, and that they were arrived at for
an unrelated reason: making a language that agents could repair.
