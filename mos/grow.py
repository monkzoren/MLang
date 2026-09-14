"""M5 — a model writes what the new unit computes.

M4 drew the line: selection discovers where structure goes, and cannot
invent what structure does. A spliced pump is a relay; it adds topology,
not function. So the model's job here is deliberately narrow, and it is
the half selection cannot do.

The operator does the structural work, exactly as in M4: strand 0 is
rewired to feed a fresh channel γ, and a new pump is spliced between it
and the policy.

    strand 0   ⎆ → γ                the body, sensing
    NEW        [ … ]⇉γα            the model writes this line
    strand 1   «X»⇒l [P]⇉αβ         the policy, untouched

The unit sees every sensor tick before the policy does and may rewrite it.
Its strand-locals persist across ticks, so unlike the policy it can
remember. Everything else -- the gate, the metabolic cost, the invariant
corpus -- is M4's, unchanged, so the number it produces is comparable to
M4's ceiling of 1300.
"""

import argparse
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "bench"))
import evolve
import gate as G
import heal
import topo
import world as W

HERE = os.path.dirname(os.path.abspath(__file__))
TMP = "/tmp/mos-grow-%d.ml" % os.getpid()   # unique, so arms can run side by side

OPERAND_ORDER = """\
OPERAND ORDER. Every operation takes what it needs off the stack, so ALL of
its operands must already be there. The glyph comes last, never first:

  «5» ⍎            parses  — the string is pushed, then ⍎ consumes it
  s « » ⊆          splits  — the string, THEN the separator, THEN ⊆
  L [⍎] ∵          maps    — the list, THEN the quotation, THEN ∵
  L i @            indexes — the list, THEN the index, THEN @

Writing ⊆« » or ∵[⍎] is the commonest mistake in this language and it
faults with `stack underflow` pointing at the glyph you wrote too early.

NAMES ARE ONE GLYPH, and storing and loading are not symmetric.

  ⇒q     store the top of the stack into the local q
  q      push the local q back (just the name, on its own)
  ↧q     NOT a local — this RECEIVES FROM CHANNEL q, and will block forever

⇒ ≔ ↥ ↧ ⇈ ⇟ each swallow exactly ONE glyph as their argument. So `⇒pos`
does not make a local called pos: it stores into the local `p` and then
leaves `o` and `s` behind as two name references, which fault as undefined
or, worse, silently read something else. Use single glyphs: q w x y z n m.

A pump that remembers the previous tick and passes this one through
unchanged, which is the shape you want, is exactly this:

```
«»⇒q [⇒w q⌫ w⇒q w]⇉γα
```

reading: take the tick into w; push the remembered q and drop it (this is
where you would compare it to w instead); store w into q for next time;
push w as the value to send on. Everything before the [ runs once, at the
start, which is how q gets its first value.
"""

CONTRACT = OPERAND_ORDER + """
You are writing ONE LINE of a running MLang program: a new strand that has
just been spliced into a machine while it was running, and is currently a
pass-through doing nothing.

THE MACHINE. It drives a robot in a grid world. Every tick it is handed a
sensor reading and answers with one action. Its three strands are:

  strand 0   the body     accepts a tick, sends it on channel γ, waits on β
                          for an action, and performs it
  strand 1   YOUR STRAND  a pump γ→α: it receives each sensor tick, may
                          transform it, and sends it on to the policy
  strand 2   the policy   a pump α→β: turns a sensor tick into an action

THE VALUE YOU RECEIVE AND MUST SEND ON is a string of eleven integers
separated by single spaces:

  x y carrying north east south west px py dx dy

  x, y        where the robot is (x is the column, y the row, y grows south)
  carrying    1 while it holds a package
  north east south west   1 when that neighbour is a wall
  px, py      where the open job must be picked up
  dx, dy      where it must then be dropped

The policy walks greedily toward (px,py) when not carrying and toward
(dx,dy) when carrying, grips when it stands on the pickup, drops when it
stands on the dropoff, and steps around walls. It has no memory of
previous ticks. YOU DO: strand-locals you set with ⇒ persist from one tick
to the next, because it is the same strand each time.

THE WORLD HAS STRUCTURE THE SENSORS DO NOT REPORT. The eleven fields are
everything the robot is told, and they are not everything there is. You
see the consequences of the world one tick at a time; nothing stops you
from remembering them.

SCORING. +100 per delivery, −1 per step into a wall, −5 for gripping or
dropping in the wrong place, over 200 ticks. A machine that walks shortest
paths scores 1300. The best possible is 1500. Your line is kept only if it
scores at least as well as it costs and never makes the robot walk into a
wall, fail to grip while standing on the pickup, or fail to drop while
standing on the dropoff.

YOUR ANSWER must be exactly one line of MLang, in a single fenced code
block, and it must be a pump from γ to α — that is, it must end with ⇉γα.
Reply with nothing but that fenced block. Do not use tools, do not read
files, do not explain. There is no repository to inspect and no command to
run; everything you need is in this message.
Anything before the pump on that line runs once, when the strand starts,
which is where to give a local its first value. The line doing nothing is:

```
[]⇉γα
```

A line that sends on a value of the wrong shape will make the policy
glitch, and you will be shown the report.
"""


def splice(body_line):
    """Put the model's line into the machine, structurally as M4's splice does."""
    src = open(os.path.join(HERE, "versions", "v6.ml")).read()
    v = evolve.Version(src)
    v.strands = [s.replace("r3@↥α", "r3@↥γ").replace("∅↥α", "∅↥γ") if "⎆" in s else s
                 for s in v.strands]
    at = next(i for i, s in enumerate(v.strands) if "⎆" in s)
    v.strands.insert(at + 1, body_line)
    return v


def evaluate(body_line, corpus, ticks):
    v = splice(body_line)
    path = v.write(TMP)
    f = evolve.fitness(path, corpus, ticks)
    _, w, rc, err = G.rollout(path, ticks)
    return f, w, err.decode(errors="replace"), v


def feedback(f, w, err):
    if f["fatal"]:
        report = "\n".join(err.strip().splitlines()[:12])
        return "The machine broke. The runtime said:\n\n" + report
    out = ["It ran. %s" % w.summary()]
    if w.score() == 1300:
        out.append("That is exactly the score of the line that does nothing, []⇉γα, "
                   "so whatever you computed did not change where the robot went.")
    if f["violations"]:
        out.append("It broke %d of the invariants (walking into walls, or failing "
                   "to grip or drop where it should)." % f["violations"])
    if f["dangling"]:
        out.append("Channels left dangling: %s" % " ".join(f["dangling"]))
    if f["fitness"] is not None:
        out.append("Fitness %d = score %d minus %d upkeep for its structure."
                   % (f["fitness"], f["score"], f["upkeep"]))
    return "\n".join(out)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--provider", default="claude-cli")
    ap.add_argument("--model", default="claude-haiku-4-5-20251001")
    ap.add_argument("--rounds", type=int, default=4)
    ap.add_argument("--ticks", type=int, default=200)
    args = ap.parse_args()

    corpus = G.frames_for(G.reachable(), W.JOBS)
    complete = heal.completer(args.provider, args.model)
    primer = heal.mlang_primer()

    base, _, _, _ = evaluate("[]⇉γα", corpus, args.ticks)
    print("the machine with an inert unit spliced in: score %d, fitness %d"
          % (base["score"], base["fitness"]))
    print("M4's ceiling, with no unit at all: score 1300, fitness 1268\n")

    history, best = [], None
    for r in range(1, args.rounds + 1):
        p = [primer, "\n", CONTRACT]
        if history:
            p.append("\nWhat you have tried so far:\n")
            for i, (line, note) in enumerate(history, 1):
                p.append("\nAttempt %d:\n```\n%s\n```\n%s\n" % (i, line, note))
            p.append("\nWrite a better line.")
        # A reply that is not a pump at all is harness noise, not an attempt:
        # `claude -p` is an agentic CLI and sometimes answers with tool-call
        # text. Re-ask rather than spend a round on it.
        line = ""
        for _ in range(3):
            line = heal.extract_program(complete("".join(p))).strip()
            line = line.splitlines()[-1].strip() if line else ""
            if "⇉γα" in line:
                break
            print("         (discarded a reply that was not a γ→α pump)")
        print("round %d: %s" % (r, line[:110] + ("…" if len(line) > 110 else "")))
        if "⇉γα" not in line:
            history.append((line, "That was not a pump from γ to α."))
            continue

        f, w, err, v = evaluate(line, corpus, args.ticks)
        note = feedback(f, w, err)
        print("         " + note.replace("\n", "\n         "))
        history.append((line, note))

        sound = not f["fatal"] and f["violations"] == 0 and f["fitness"] is not None
        if sound and (best is None or f["fitness"] > best[0]["fitness"]):
            best = (f, line, v)

    print()
    if best is None:
        print("nothing shippable in %d rounds. M4's ceiling stands at 1300." % args.rounds)
        return 0
    f, line, v = best
    verdict = "BEATS" if f["score"] > 1300 else ("matches" if f["score"] == 1300 else "below")
    print("best: score %d (%s M4's 1300), fitness %d" % (f["score"], verdict, f["fitness"]))
    print("      %s" % line)
    if f["score"] > 1300:
        out = os.path.join(HERE, "versions", "v7.ml")
        v.write(out)
        print("      written to %s" % out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
