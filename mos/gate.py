"""M3 — the gate: what a new version has to survive to be kept.

Growth without selection is bloat. A model will add strands forever, so
"a pathway formed" is worthless on its own; what counts is retention under
selection. This is the selection.

A candidate faces two tests, and they are deliberately different in kind:

*   **The rollout.** A fresh episode in the world. The world is
    deterministic, so this is exact and repeatable -- no averaging, no
    seeds, no judge. A candidate must not lose score.

*   **The invariant corpus.** Single-frame goldens: one sensor frame in,
    one action out, checked against a property that holds no matter what
    the policy is -- never step into a wall, grip when standing on the
    pickup, answer inside the alphabet. These are the structures that must
    survive learning.

The two are split because a *recorded episode* is only valid for the
policy that generated it. A better policy takes different actions, so the
sensor frames that follow would no longer be the ones it would meet:
replaying a recorded stream against a changed policy is off-policy, and
demanding byte-identity there would forbid every improvement. Byte-exact
replay is the right tool for *reproducing* a run (M1, M2) and the wrong
tool for *judging* a policy. So behaviour is judged by rollout, and only
the invariants -- which are single-step and therefore order-free -- are
pinned byte-exact.
"""

import argparse
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import topo
import world as W


def frames_for(cells, jobs):
    """The invariant corpus: every reachable cell, carrying or not, per job."""
    out = []
    for j, (pick, drop) in enumerate(jobs):
        for (x, y) in cells:
            for carrying in (0, 1):
                w = W.World()
                w.x, w.y, w.carrying, w.job = x, y, carrying, j
                out.append((w.sense(), (x, y), carrying, pick, drop))
    return out


def reachable():
    from collections import deque
    seen = {W.START}
    q = deque([W.START])
    while q:
        x, y = q.popleft()
        for dx, dy in W.DELTA.values():
            n = (x + dx, y + dy)
            if not W.wall(*n) and n not in seen:
                seen.add(n)
                q.append(n)
    return sorted(seen)


def ask(program, sensors):
    """Put every frame to the grid in one run and collect its answers."""
    stream = bytearray()
    for s in sensors:
        b = s.encode()
        stream += b"\xe2\x96\xb7 POST /tick %d\n%s\n" % (len(b), b)
    p = subprocess.run([W.MLANG, "run", program], input=bytes(stream), capture_output=True)
    out, acts = p.stdout, []
    i = 0
    while i < len(out):
        j = out.index(b"\n", i)
        parts = out[i:j].decode().split()
        if parts and parts[0] == "◁":
            n = int(parts[4])
            acts.append(out[j + 1:j + 1 + n].decode().strip())
            i = j + 1 + n + 1
        else:
            i = j + 1
    return acts


ALPHABET = set("NESWGDX")


def invariants(program, corpus):
    """Violations of the properties that hold whatever the policy is."""
    acts = ask(program, [c[0] for c in corpus])
    bad = []
    for (sense, pos, carrying, pick, drop), a in zip(corpus, acts):
        f = [int(v) for v in sense.split()]
        if a not in ALPHABET:
            bad.append(("alphabet", pos, carrying, a))
            continue
        if a in W.DELTA and f[3 + "NESW".index(a)]:
            bad.append(("walks into a wall", pos, carrying, a))
        if pos == pick and not carrying and a != "G":
            bad.append(("stands on the pickup and does not grip", pos, carrying, a))
        if pos == drop and carrying and a != "D":
            bad.append(("stands on the dropoff and does not drop", pos, carrying, a))
    return bad, len(acts)


def in_context(w):
    """The invariants, measured on the trajectory the machine actually walked.

    The single-frame corpus above is only valid while the machine is
    memoryless: `ask` feeds every frame to one process in sequence, so a
    stateful unit builds its memory from 336 teleporting positions and its
    answers there say nothing about its behaviour in the world. It is also
    blind to a unit that legitimately rewrites the policy's target — at the
    true pickup that reads as "did not grip", though the machine grips a
    moment later having gone where it meant to.

    So once the machine can remember, the honest check is what the world
    recorded: steps into walls, and grips or drops in the wrong place. The
    coverage is narrower — only states the trajectory visits — and it is
    sound, which the corpus no longer is.
    """
    out = []
    if w.bumps:
        out.append(("walks into a wall", w.bumps))
    if w.bad:
        out.append(("grips or drops in the wrong place", w.bad))
    return out


def rollout(program, ticks=200):
    w, _, err, rc = W.episode(program, ticks)
    return w.score(), w, rc, err


def verdict(live, candidate, corpus, ticks=200):
    """Ship or reject, with the reason. This is the whole pruning rule."""
    ls, _, _, _ = rollout(live, ticks)
    cs, cw, crc, cerr = rollout(candidate, ticks)
    bad, n = invariants(candidate, corpus)
    lt, ct = topo.topology(live), topo.topology(candidate)

    reasons = []
    if bad:
        kinds = {}
        for k, *_ in bad:
            kinds[k] = kinds.get(k, 0) + 1
        reasons.append("breaks %d/%d invariants (%s)"
                       % (len(bad), n, ", ".join("%s ×%d" % kv for kv in sorted(kinds.items()))))
    if cs < ls:
        reasons.append("score %d → %d" % (ls, cs))
    if crc != 0:
        reasons.append("exits %d (%s)"
                       % (crc, "dangling: " + " ".join(ct["dangling"]) if ct["dangling"] else "fault"))
    if ct["dangling"] and crc == 0:
        reasons.append("leaves %s dangling" % " ".join(ct["dangling"]))

    grew = (ct["n_strands"] - lt["n_strands"], ct["n_channels"] - lt["n_channels"],
            ct["n_definitions"] - lt["n_definitions"])
    return {"ship": not reasons, "reasons": reasons, "live_score": ls, "score": cs,
            "violations": len(bad), "checked": n, "grew": grew,
            "topo": ct, "live_topo": lt}


def retention(versions):
    """Of the pathways formed at each version, how many are still there later."""
    ts = [topo.topology(v) for v in versions]
    rows = []
    for i in range(1, len(ts)):
        new = sorted(set(ts[i]["channels"]) - set(ts[i - 1]["channels"]))
        if not new:
            continue
        life = []
        for c in new:
            k = i
            while k < len(ts) and c in ts[k]["channels"]:
                k += 1
            life.append((c, k - i))
        rows.append((os.path.basename(versions[i]), life))
    return rows


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("candidates", nargs="*")
    ap.add_argument("--live", default=os.path.join(os.path.dirname(os.path.abspath(__file__)), "os0.ml"))
    ap.add_argument("--ticks", type=int, default=200)
    ap.add_argument("--retention", nargs="+", metavar="VERSION")
    args = ap.parse_args()

    corpus = frames_for(reachable(), W.JOBS)
    print("invariant corpus: %d frames (%d cells × 2 × %d jobs)\n"
          % (len(corpus), len(reachable()), len(W.JOBS)))

    for c in args.candidates:
        v = verdict(args.live, c, corpus, args.ticks)
        mark = "ship  " if v["ship"] else "REJECT"
        ds, dc, dd = v["grew"]
        growth = " ".join(x for x in ["%+ds" % ds if ds else "", "%+dch" % dc if dc else "",
                                      "%+ddef" % dd if dd else ""] if x) or "no growth"
        print("  %s %-10s score %4d  %-22s %s"
              % (mark, os.path.basename(c), v["score"], growth,
                 "; ".join(v["reasons"]) or "invariants %d/%d clean" % (v["checked"], v["checked"])))

    if args.retention:
        print("\nretention of formed pathways:")
        for name, life in retention(args.retention):
            print("  %-8s %s" % (name, ", ".join("%s survives %d version(s)" % l for l in life)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
