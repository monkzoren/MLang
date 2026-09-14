"""M2 — the structural repertoire.

Before asking a model to grow a machine, find out what the substrate can
express. Five patches are written by hand and woven into one running grid,
in one episode, without stopping it:

    v1  rebind a definition          nothing structural; behaviour moves
    v2  add a strand                 a unit before its connections
    v3  form the pathway             the unit is spliced in as a layer
    v4  retire a strand              pruning
    v5  hoist shared substructure    one definition, two call sites

Each patch rides the same stdin stream as sensation (SPEC 4.7: a patch is a
⟡ frame), so the whole session -- the machine living, being restructured
five times, and living on -- is one deterministic byte stream that replays
offline. Run it with --record to pin it.
"""

import argparse
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import topo
import world as W

HERE = os.path.dirname(os.path.abspath(__file__))
OS0 = os.path.join(HERE, "os0.ml")
VERSIONS = [os.path.join(HERE, "versions", "v%d.ml" % i) for i in range(1, 6)]
AT = [20, 50, 80, 110, 140]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ticks", type=int, default=200)
    ap.add_argument("--record", metavar="DIR")
    args = ap.parse_args()

    print("== each version alone ==")
    chain = [OS0] + VERSIONS
    for path in chain:
        w, _, err, rc = W.episode(path, args.ticks)
        note = ""
        if rc:
            lines = [l for l in err.decode().splitlines() if l.startswith("  ⚠") or "deadlock" in l]
            note = "  ← " + "; ".join(l.strip().rstrip(" —").split(" — ")[0] for l in lines[:1])
            note += " (" + ", ".join(l.split()[2] for l in lines if l.startswith("  ⚠")) + " dangling)"
        print("  %-8s %s rc=%d%s" % (os.path.basename(path), w.summary(), rc, note))

    print("\n== structure, version to version ==")
    for a, b in zip(chain, chain[1:]):
        d = topo.diff(topo.topology(a), topo.topology(b))
        t = topo.topology(b)
        print("  %s → %-6s %s" % (os.path.basename(a).replace(".ml", ""),
                                  os.path.basename(b).replace(".ml", ""),
                                  d.replace("\n", "; ")))
        print("           %d strands, %d channels, %d definitions, %d pathways"
              % (t["n_strands"], t["n_channels"], t["n_definitions"], t["n_pathways"]))

    print("\n== all five, woven into one running grid ==")
    patches = list(zip(AT, VERSIONS))
    w, frames, err, rc = W.episode(OS0, args.ticks, patches=patches, record=args.record)
    for tick, status, report in w.versions:
        head, *rest = report.splitlines()
        print("  tick %3d  %d  %s" % (tick, status, head))
        for r in rest:
            print("           %s" % r.strip())
    print("  %s rc=%d" % (w.summary(), rc))
    if err:
        sys.stderr.write(err.decode(errors="replace"))

    if args.record:
        fp = os.path.join(args.record, "episode.frames")
        os.rename(fp, os.path.join(args.record, "m2.frames"))
        fp = os.path.join(args.record, "m2.frames")
        out, rerr, rrc = W.replay(OS0, fp)
        again = W.replay(OS0, fp)
        open(os.path.join(args.record, "m2.out"), "wb").write(out)
        ok = (out, rerr, rrc) == again
        print("\n  replay: %s (%d bytes stdout, exit %d)"
              % ("byte-identical" if ok else "DIVERGED", len(out), rrc))
        return 0 if ok else 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
