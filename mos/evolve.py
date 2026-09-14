"""M4 — variation and selection over the structural repertoire, with no model.

M2 showed the loom can express five kinds of structural change. This makes
those five an operator set and lets selection run over it for many
generations: propose a variant, weigh it, keep or discard. No LLM, so this
is deterministic given a seed, costs nothing, and can run for hundreds of
generations across many seeds -- which is what the retention question needs
and what a model arm cannot cheaply give.

Three arms, the same proposals in the same order:

    gated     a variant is kept only if it is worth its cost, at once
    neutral   as gated, but a variant may carry unproven structural debt
    ungated   every variant is kept

The middle arm exists because of what the first two showed: strict selection
never grows at all. `grow` costs upkeep and earns nothing, so it is always
refused -- and `splice` can then never fire, because it needs an unwired unit
to exist first. The unit must arrive before its connections, and the unit
alone never pays for itself. Crossing that valley requires tolerating neutral
intermediates, which is the whole reason neutral drift matters in evolution.

The operators, from M2:

    grow      append an unwired pump on two fresh channels   (+1 strand, +2 channels)
    splice    wire an unwired pump into an existing pathway  (a layer is inserted)
    prune     remove a pump that is not the policy           (-1 strand)
    tune      perturb the policy's comparisons               (no structural change)

Fitness is the rollout score minus a *metabolic cost* per strand and per
channel. Structure is not forbidden, it is charged for. Pruning is then
something selection discovers rather than something the rule mandates --
which is how brains do it: tissue is expensive.
"""

import argparse
import os
import random
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gate as G
import topo
import world as W

HERE = os.path.dirname(os.path.abspath(__file__))
SIGILS = "γδεζηθικμνξορστυφχψω"        # channel names not already in use
COST = 8                                # score charged per strand and per channel
NEUTRAL = 32                            # debt a variant may carry while it is still unproven


class Version:
    """A version as text the operators can manipulate: boot lines and strand lines."""

    def __init__(self, text):
        lines = text.split("\n")
        d = next(i for i, l in enumerate(lines) if l.strip() == "⇊")
        self.boot = lines[:d]
        self.strands = [l for l in lines[d + 1:] if l.strip() and not l.lstrip().startswith("※")]

    def text(self):
        return "\n".join(self.boot + ["⇊"] + self.strands) + "\n"

    def write(self, path):
        open(path, "w").write(self.text())
        return path

    def copy(self):
        return Version(self.text())

    def free_sigils(self, n):
        used = set(self.text())
        out = [s for s in SIGILS if s not in used]
        return out[:n] if len(out) >= n else None

    def pumps(self):
        """Strand lines that are a bare pump — the units selection may move or drop."""
        return [i for i, l in enumerate(self.strands) if "⇉" in l and "⎆" not in l]

    def unwired(self):
        """Pumps whose channels nobody else touches: units without connections."""
        out = []
        for i in self.pumps():
            s, r = topo.scan(topo.mask(self.strands[i]))[0], topo.scan(topo.mask(self.strands[i]))[1]
            others = "".join(topo.mask(l) for j, l in enumerate(self.strands) if j != i)
            if all(c not in others for c in s | r):
                out.append(i)
        return out


def op_grow(v, rng):
    f = v.free_sigils(2)
    if not f:
        return None
    v.strands.append("[]⇉%s%s" % (f[0], f[1]))
    return "grow %s%s" % (f[0], f[1])


def op_splice(v, rng):
    """Insert an unwired pump between a sender and its receiver."""
    cand = v.unwired()
    if not cand:
        return None
    i = rng.choice(cand)
    src, dst = None, None
    for c in topo.scan(topo.mask(v.strands[i]))[1] | topo.scan(topo.mask(v.strands[i]))[0]:
        pass
    recv = sorted(topo.scan(topo.mask(v.strands[i]))[1])
    send = sorted(topo.scan(topo.mask(v.strands[i]))[0])
    if not recv or not send:
        return None
    g, d = recv[0], send[0]

    # find a live edge: some line sends on c, another receives it
    edges = []
    for a, la in enumerate(v.strands):
        if a == i:
            continue
        sa = topo.scan(topo.mask(la))[0]
        for c in sa:
            for b, lb in enumerate(v.strands):
                if b in (a, i):
                    continue
                if c in topo.scan(topo.mask(lb))[1]:
                    edges.append((a, b, c))
    if not edges:
        return None
    a, b, c = rng.choice(edges)
    v.strands[a] = v.strands[a].replace("↥" + c, "↥" + g)
    v.strands[b] = v.strands[b].replace("⇉" + c, "⇉" + d).replace("↧" + c, "↧" + d)
    return "splice %s into %s→%s" % (c, g, d)


def op_prune(v, rng):
    cand = v.pumps()
    cand = [i for i in cand if "P" not in v.strands[i]]      # never drop the policy
    if not cand:
        return None
    i = rng.choice(cand)
    line = v.strands.pop(i)
    return "prune %s" % line.strip()


import re

AXIS = ["p q>", "p q≥", "0"]
TDEF = re.compile(r"⟨((?:«[^»]*»)+)⟩≔T")


def op_tune(v, rng):
    """Perturb the policy without touching the wiring: the axis comparison, or
    the order of the action alphabet T -- whose first four entries are the
    escape preference, so one definition tunes every strand that reads it."""
    boot = "\n".join(v.boot)
    choices = []
    for a in AXIS:
        if a in boot:
            choices += [("axis", a, b) for b in AXIS if b != a]
    m = TDEF.search(boot)
    if m:
        items = re.findall(r"«[^»]*»", m.group(1))
        if len(items) >= 4:
            for i in range(4):
                for j in range(i + 1, 4):
                    choices.append(("order", (m.group(0), items), (i, j)))
    if not choices:
        return None
    c = rng.choice(choices)
    if c[0] == "axis":
        boot = boot.replace(c[1], c[2], 1)
        note = "tune axis %s→%s" % (c[1], c[2])
    else:
        whole, items = c[1]
        i, j = c[2]
        new = list(items)
        new[i], new[j] = new[j], new[i]
        boot = boot.replace(whole, "⟨" + "".join(new) + "⟩≔T", 1)
        note = "tune order %s↔%s" % (items[i], items[j])
    v.boot = boot.split("\n")
    return note


OPS = [(op_grow, 3), (op_splice, 3), (op_prune, 2), (op_tune, 4)]


def fitness(path, corpus, ticks):
    """Score, minus what the structure costs to carry. Faults are fatal."""
    score, w, rc, err = G.rollout(path, ticks)
    t = topo.topology(path)
    e = err.decode(errors="replace")
    fatal = ("glitch" in e) or ("load error" in e) or rc == 2 or w.ticks < ticks
    bad, n = ([], 0) if fatal else G.invariants(path, corpus)
    upkeep = COST * (t["n_strands"] + t["n_channels"])
    return {"fatal": fatal, "violations": len(bad), "score": score,
            "fitness": (score - upkeep) if not (fatal or bad) else None,
            "upkeep": upkeep, "topo": t, "dangling": t["dangling"]}


def run(arm, seed, generations, corpus, ticks, start):
    rng = random.Random(seed)
    cur = Version(open(start).read())
    tmp = os.path.join("/tmp", "mos-evolve-%s-%d.ml" % (arm, seed))
    base = fitness(cur.write(tmp), corpus, ticks)
    born, ever = set(base["topo"]["channels"]), set(base["topo"]["channels"])
    hist = [{"gen": 0, "op": "start", "kept": True, **{k: base[k] for k in ("score", "fitness")},
             "strands": base["topo"]["n_strands"], "channels": base["topo"]["n_channels"],
             "dangling": len(base["dangling"])}]
    best = base

    for g in range(1, generations + 1):
        cand = cur.copy()
        ops = [o for o, wgt in OPS for _ in range(wgt)]
        note = None
        for _ in range(6):                       # a few tries to find an applicable operator
            note = rng.choice(ops)(cand, rng)
            if note:
                break
        if not note:
            continue
        f = fitness(cand.write(tmp + ".c"), corpus, ticks)

        sound = (not f["fatal"]) and f["violations"] == 0 and f["fitness"] is not None
        if arm == "ungated":
            keep = not f["fatal"]                 # only a broken program is refused
        elif arm == "gated":
            keep = sound and f["fitness"] >= best["fitness"]
        else:                                     # gated, with a neutral band
            # A variant that loses no score and breaks no invariant is allowed
            # through even though its structure costs, so long as the debt stays
            # inside NEUTRAL. Without this a unit can never arrive before its
            # connections do, and no pathway can ever form.
            keep = sound and f["fitness"] >= best["fitness"] - NEUTRAL

        if keep:
            cur = cand
            ever |= set(f["topo"]["channels"])
            if f["fitness"] is not None and (best["fitness"] is None or f["fitness"] >= best["fitness"]):
                best = f
        hist.append({"gen": g, "op": note, "kept": keep, "score": f["score"],
                     "fitness": f["fitness"], "strands": f["topo"]["n_strands"],
                     "channels": f["topo"]["n_channels"], "dangling": len(f["dangling"])})
    formed = ever - born                              # pathways that did not exist at the start
    survived = formed & set(topo.topology(cur.write(tmp))["channels"])
    return hist, cur, (len(formed), len(survived))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--generations", type=int, default=60)
    ap.add_argument("--seeds", type=int, default=5)
    ap.add_argument("--ticks", type=int, default=200)
    ap.add_argument("--start", default=os.path.join(HERE, "versions", "v6.ml"))
    args = ap.parse_args()

    corpus = G.frames_for(G.reachable(), W.JOBS)
    print("start: %s   cost: %d per strand and per channel   %d generations × %d seeds\n"
          % (os.path.basename(args.start), COST, args.generations, args.seeds))

    for arm in ("gated", "neutral", "ungated"):
        rows = []
        for s in range(args.seeds):
            hist, final, (formed, survived) = run(arm, s, args.generations, corpus, args.ticks, args.start)
            kept = [h for h in hist if h["kept"]]
            last = kept[-1]
            peak = max(h["strands"] + h["channels"] for h in kept)
            rows.append((s, last["score"], last["fitness"], last["strands"],
                         last["channels"], last["dangling"], peak, formed, survived))
        print("  %s" % arm)
        print("    seed  score  fitness  strands  chans  dangling  peak(s+c)  formed  surv")
        for r in rows:
            print("    %4d  %5s  %7s  %7d  %5d  %8d  %9d  %6d  %4d"
                  % (r[0], r[1], r[2] if r[2] is not None else "—", r[3], r[4], r[5], r[6], r[7], r[8]))
        avg = lambda i: sum(r[i] for r in rows) / len(rows)
        tot_f, tot_s = sum(r[7] for r in rows), sum(r[8] for r in rows)
        print("    mean  %5.0f  %7.0f  %7.1f  %5.1f  %8.1f  %9.1f  %6.1f  %4.1f"
              % (avg(1), avg(2), avg(3), avg(4), avg(5), avg(6), avg(7), avg(8)))
        print("    retention of formed pathways: %d/%d (%.0f%%)\n"
              % (tot_s, tot_f, 100.0 * tot_s / tot_f if tot_f else 0.0))
    return 0


if __name__ == "__main__":
    sys.exit(main())
