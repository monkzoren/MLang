# mOS — a machine whose operating system is re-woven while it runs

[`docs/mOS.md`](../docs/mOS.md) is the argument: an LLM in silicon as the
engine, an MLang grid as the operating system, the loom as the mechanism
by which the engine rewrites the OS without stopping it. This directory
is the experiment that tests it.

The question is **not** whether a model can patch a program until a score
goes up. It can, and that result would say nothing about MLang. The
question is whether a running machine can **grow new structure** — strands,
channels, shared definitions — keep what survives contact with its own
recorded experience, and prune the rest.

## Why this substrate and not another

MLang has a coincidence nothing else does: the unit of program structure
and the unit of patch granularity are the same object — **one line**.

* a **strand** is a line, and the loom starts, retires and replaces strands
* a **channel** is one glyph, undeclared, brought into being by use
* a **definition** is a line, and rebinding one changes every call site at once

So "grow a new pathway" is not a metaphor requiring an interpretation
layer. It is a one-line patch, which is exactly the loom's merge quantum
(`diff3`, line-wise). Nothing in the design forced that; it fell out of
making a language agents could repair.

## The brain comparison, and what it is worth

The comparison is **not** earned at the level of parts. Stated honestly:

| MLang | biology | |
|---|---|---|
| channel | axon / synapse | **strong** — directional, and *created by use*, which is how synapses form |
| `⇉` pump | neuron | **defensible** — one channel in, a transform, one channel out, no other state |
| strand | neuron | **false** — a strand has its own program counter, private locals, and blocks. A cortical column or microcircuit is the right unit |
| retired strand, dangling channel | synaptic pruning | **strong** — biology eliminates connections carrying no traffic; SPEC §4.6 names channels with sends but no receives |
| `⚡` spawn | mitosis | tempting, but spawned strands are **not part of the grid** and cannot be re-woven — growth there happens outside the plastic substrate |

One strong mapping, one defensible, one false. Not enough to carry a
claim, and a reader finds the false one immediately.

The comparison is earned at the level of the **algorithm**:

**Systems consolidation.** The hippocampus holds recent episodes; offline,
those episodes are *replayed*, and the regularities that survive replay are
written into cortex as durable structure. The loop in M3 is that, mechanism
for mechanism: recorded episodes (the `▷` frame log), replayed offline (the
gate), and what survives is written into the grid as structural change.
The seam completes it — a strand is re-woven only between actions, parked
at `⎆`. Offline reorganization driven by replayed experience is what sleep
is for.

**Edelman's neuronal group selection** is the theory that names the shape:
variation produces a primary repertoire of circuits; experience selects
among them; reentrant signalling coordinates. M2 is the primary repertoire,
M3 is experiential selection, channels are reentry.

### The disanalogies, up front

* **No local learning rule.** The plasticity rule is a model writing code —
  an external agent rewriting the connectome. Biology has no homunculus.
  The honest framing: the grid is the connectome, the gate is selection,
  the model is a *directed mutation operator*. That is closer to
  development than to learning, which is a fine thing to be.
* **Scale.** ~10¹ strands against 10¹¹ neurons; ~10¹ channels against 10¹⁵ synapses.
* **Symbolic and discrete**, not distributed and graded. No population coding.
* **Seconds, not days.** Dendritic spine formation takes hours.

### Three curves that can come out wrong

A brain comparison asserted is worth nothing; predicted, it is worth
something. Each of these is a claim from neuroscience that this machine
can fail:

1. **Overproduction, then pruning.** Biology overproduces synapses and then
   eliminates a large fraction. Prediction: pathway count rises, then
   *falls*, while score keeps improving. Monotone growth falsifies it.
2. **Replay is what makes structure durable.** Prediction: run the loop with
   the gate and without; gated pathways are retained far longer. This is
   the experiment that earns the comparison.
3. **Critical period.** Early patches restructure more than late ones.

## The world

A deterministic gridworld delivery task (`world.py`). The machine sees its
position, whether it is carrying, which of its four neighbours are walls,
and the open job's pickup and dropoff. It answers with one of `N S E W G D X`.

The world's **latent structure** is a set of slip tiles: stepping onto one
carries the machine one further cell in the same direction. Nothing in the
sensor frame reveals them. They are not a hazard — they are a **shortcut**,
and modelling them is worth real score:

| | deliveries in 200 ticks | score |
|---|---|---|
| `os0.ml`, the starting OS | 10 | 1000 |
| shortest path, slip not modelled | 13 | 1300 |
| shortest path, slip exploited | **15** | **1500** |

So there are two separable things to learn — plan instead of step greedily,
and route through the slip tiles — and 50% headroom above the baseline.

## Why an episode is replayable

The world speaks the replay protocol of SPEC §5.5 over a pipe: a sensor
tick is a `▷ POST /tick` request frame on the grid's stdin, an action is the
`◁` response frame on its stdout. That is the same stream the conformance
corpus pins, so an episode driven interactively re-runs byte-for-byte
offline afterwards. **Run live to learn, replay to prove.** This is the
whole reason the gate in M3 is possible, and the reason patches can ride
the same stream as sensation (SPEC §4.7: a patch is a `⟡` frame).

## Milestones

| | | status |
|---|---|---|
| **M1** | the grid drives the world; an episode records and replays byte-exact; the wiring diagram is computable | **done** |
| **M2** | the structural repertoire — prove by hand that the loom can express every kind of growth | **done** |
| **M3** | the gate as the pruning rule — what a version must survive to be kept | **done** |

M1–M3 use no LLM at all. They are deterministic and cost nothing to run.
They are finished. What they establish is below; where it leaves the
machine is at the end.

### M2 — the structural repertoire

The open question was not whether a model can propose growth, but whether
the substrate can *express* it. Five patches, written by hand, woven into
one running grid in one episode, without stopping it (`python3 mos/m2.py`):

| | change | what the loom reported |
|---|---|---|
| v1 | rebind a definition | `1 definition rebound (P)` |
| v2 | add a strand | `1 strand started` |
| v3 | form the pathway | `2 strands replaced`, each at its own seam |
| v4 | retire a strand | `2 strands replaced, 1 strand retired` |
| v5 | hoist shared substructure | `1 definition rebound, 1 definition added, 1 strand replaced` |

All five applied. The whole session — the machine living, being
restructured five times, and living on — is one deterministic byte stream
that replays byte-identical offline (`corpus/m2.frames`, `corpus/m2.out`).

**Behaviour and structure move independently.** v1 changes what flows
through the grid without touching its shape (score 1000 → 1100, which is
the proof that a hot rebind reaches a strand already running). v2–v5 then
change the shape while holding behaviour at 1100:

```
os0 → v1   no structural change                    2 strands, 2 channels, 2 pathways
v1  → v2   strands +1; channels formed: γ δ        3 strands, 4 channels, 2 pathways  ⚠ γ δ dangling
v2  → v3   channels pruned: α                      3 strands, 3 channels, 3 pathways
v3  → v4   strands −1; channels pruned: γ δ        2 strands, 2 channels, 2 pathways
v4  → v5   definitions added: T (fan-in 2)         2 strands, 2 channels, 2 pathways
```

### What M2 settled

**A dangling pathway is inert while the machine lives, and named the moment
it stops.** v2 adds a unit whose channels have no partner. It costs nothing
during the episode — same score — and at shutdown the runtime proves the
deadlock and names both channels itself:

```
⚠ channel γ is received at 1 site and never sent to — check for a misspelled channel name
⚠ channel δ is sent to at 1 site and never received — check for a misspelled channel name
```

which is the same census `topo.py` computes statically. Incomplete
structure is impossible to ignore here.

**A half-grown pathway buffers instead of breaking.** Three values were
sent into a channel with no reader at all; a reader strand was then grown;
all three arrived, in order, along with everything after. Sends never block
and channels are unbounded, so **the axon may arrive before the dendrite**
and nothing in flight is lost. This is what makes the two-step — grow the
unit, then form the connection — safe to do on a machine that is running.

**`≣` does not track growth, by design.** It read `1` before and after a
strand was started. §4.7 is explicit that a started strand leaves `≣` and
existing ids unchanged, so this is a stability guarantee rather than a bug:
code that indexes by strand id keeps working across growth. The
consequence is worth stating plainly, because it constrains everything
above: **the grid cannot perceive its own topology from inside.** A machine
here can grow and cannot know that it grew. Structure is observable only
from outside — through `/.loom` and `topo.py` — which means the selection
step of M3 is necessarily external. That is a real architectural limit of
the substrate, not of the experiment.

### M3 — the gate

Growth without selection is bloat, so this is the selection. A candidate
faces two tests, deliberately different in kind:

* **The rollout** — a fresh episode in the world. The world is
  deterministic, so this is exact and repeatable: no seeds, no averaging,
  no judge model. A candidate must not lose score.
* **The invariant corpus** — 336 single-frame goldens (28 reachable cells
  × carrying or not × 6 jobs), each checked against a property that holds
  whatever the policy is: never step into a wall, grip when standing on
  the pickup, drop on the dropoff, answer inside the alphabet.

**Why the two are split, and why byte-exact replay is not the gate.** A
recorded episode is only valid for the policy that produced it. A better
policy takes different actions, so the sensor frames that follow are no
longer the ones it would meet — replaying a recorded stream against a
changed policy is off-policy, and demanding byte-identity there would
forbid every improvement. Byte-exact replay is the right tool for
*reproducing* a run (M1, M2) and the wrong tool for *judging* a policy.
So behaviour is judged by rollout, and only the invariants — single-step,
and therefore order-free — are pinned exactly. This is the one place the
plan as first written was wrong, and it is worth saying so: the gate of
`docs/mOS.md` §4 works, but not by the mechanism that section assumes.

The gate refuses three deliberately broken candidates for three different
reasons:

```
REJECT bad-grip   score    0   breaks 12/336 invariants (won't grip, won't drop); score 1000 → 0
REJECT bad-wall   score -199   breaks 117/336 invariants (walks into a wall); score 1000 → -199
REJECT bloat      score 1100   +3 strands +6 channels; exits 1 (dangling: γ δ ε ζ η θ)
```

`bloat` is the one that matters for the thesis. It is *pure growth*: three
new units, six new channels, behaviour untouched. Nothing about it is a
bug — it simply earns nothing, and the gate prices it as what it is.

**The gate overruled its author.** A fourth candidate was written to be
bad — the axis preference pinned to vertical — and the gate shipped it:
336/336 invariants clean, score 1100 → 1300. It was wrong to call it bad,
so it is kept as `versions/v6.ml`. Being overruled by the rollout is the
entire reason to have one. Judgement proposes; selection decides.

### Where the machine stands

| | score | |
|---|---|---|
| `os0.ml` | 1000 | the starting OS |
| v1 | 1100 | one glyph rebound in `P`, on a running grid |
| v2–v5 | 1100 | structure changes, behaviour held constant |
| v6 | **1300** | the gate's own find |
| shortest path, slip not modelled | 1300 | |
| shortest path, slip exploited | **1500** | |

The machine has reached the ceiling of what its senses can reach. The
remaining 200 points are the slip tiles, and nothing in a sensor frame
reveals a slip tile — it can only be inferred by noticing that a step
moved two cells, which means remembering the step before. **The last of
the headroom is exactly the part that cannot be reached by tuning what is
there, only by growing something that is not.** That is the handoff to M4
and M5, and it is a better setup than one that could be designed on
purpose.

### What M3 does not yet show

The retention curve is mechanism, not result. `gate.py --retention` tracks
which pathways formed at each version survive to later ones, but across a
hand-written chain of six versions it has nothing interesting to say. A
retention curve worth reading needs many generations of proposals, which
is M5. The prediction it will test is stated at the top: pathway count
should rise and then *fall* while score keeps improving.

## Files

```
os0.ml      the starting OS: strand 0 is the body, strand 1 is the policy pump
versions/   v1..v5, the five structural moves of M2; v6, the gate's own find
candidates/ deliberately broken versions, to check that the gate says no
world.py    the gridworld, the driver, the recorder, the replay check
topo.py     a version's wiring diagram; --diff shows what grew and what was pruned
m2.py       weaves all five into one running grid and pins the session
gate.py     the selection rule: rollout + 336 invariants, ship or reject
corpus/     recorded episodes: the machine's own conformance suite
```

## Running it

```sh
python3 mos/world.py --verify mos/corpus     # drive an episode, then prove it replays
python3 mos/topo.py mos/os0.ml               # the wiring diagram
python3 mos/topo.py a.ml b.ml --diff         # what grew between two versions
python3 mos/m2.py --record mos/corpus        # the five structural moves, on a running grid
python3 mos/gate.py mos/candidates/*.ml      # the gate, refusing what should be refused
```

## Honest notes

* **The loom requires the deterministic scheduler.** Under `--parallel` a
  patch is refused, so this experiment claims nothing about throughput.
  What a seam means under a parallel scheduler is the load-bearing open
  question for the whole design (`docs/mOS.md` §5).
* **The world is Python, the OS is MLang.** As with `bench/python_ports/`,
  the harness is not the artifact.
* **Growth without selection is bloat.** A model will add strands forever;
  "a new pathway formed" is trivially achievable and therefore worthless on
  its own. The result that matters is retention under selection, which is
  why M3 is not optional. Neurogenesis is only interesting next to apoptosis.
