"""The wiring diagram of a grid — what M1..M3 actually measure.

A version of the OS is read as a graph: strands are the units, channels are
the pathways between them, definitions are the shared substructure. Growth
is a change in this graph, so every claim about "forming new pathways" is a
diff of two of these.

The dangling census mirrors what the runtime itself reports on deadlock
(SPEC 4.6): a channel with senders but no receivers, or the reverse, is a
pathway that terminates nowhere. Here it is computed statically, so a
half-grown pathway is visible without having to wait for a deadlock.
"""

import argparse
import json
import re
import sys

ARG1 = "≔⇒↥↧⇂⇈⇟"          # these consume the next non-blank glyph
SENDS = "↥⇈"
RECVS = "↧⇂⇟"


def mask(line):
    """Blank out comments and string literals so their glyphs are not read as code."""
    out = []
    i = 0
    while i < len(line):
        c = line[i]
        if c == "※":
            break
        if c == "«":
            j = line.find("»", i)
            j = len(line) if j < 0 else j + 1
            out.append(" " * (j - i))
            i = j
            continue
        out.append(c)
        i += 1
    return "".join(out)


def scan(code):
    """(sends, receives, defs, locals, refs) for one masked chunk of code."""
    sends, recvs, defs, locs, refs = set(), set(), [], set(), []
    i = 0
    while i < len(code):
        c = code[i]
        if c == "⇉":                      # pump: source channel, then destination
            rest = [g for g in code[i + 1:] if not g.isspace()]
            if len(rest) >= 2:
                recvs.add(rest[0])
                sends.add(rest[1])
            i += 1
            consumed = 0
            while i < len(code) and consumed < 2:
                if not code[i].isspace():
                    consumed += 1
                i += 1
            continue
        if c in ARG1:
            j = i + 1
            while j < len(code) and code[j].isspace():
                j += 1
            if j < len(code):
                a = code[j]
                if c in SENDS:
                    sends.add(a)
                elif c in RECVS:
                    recvs.add(a)
                elif c == "≔":
                    defs.append(a)
                elif c == "⇒":
                    locs.add(a)
            i = j + 1
            continue
        if c.isalpha() or (ord(c) > 0x390 and c not in ARG1 + "⇉"):
            refs.append(c)
        i += 1
    return sends, recvs, defs, locs, refs


def topology(path):
    raw = open(path, encoding="utf-8").read().splitlines()
    lines = [mask(l) for l in raw]
    div = next((i for i, l in enumerate(lines) if l.strip() == "⇊"), -1)
    boot, body = (lines[:div], lines[div + 1:]) if div >= 0 else ([], lines)

    defs = []
    for l in boot:
        defs += scan(l)[2]

    strands = []
    for n, l in enumerate(body):
        if not l.strip():
            continue
        s, r, _, loc, refs = scan(l)
        strands.append({"row": (div + 1 if div >= 0 else 0) + n + 1,
                        "id": len(strands), "sends": sorted(s), "recvs": sorted(r),
                        "locals": sorted(loc), "glyphs": len(l.strip())})

    chans = {}
    for st in strands:
        for c in st["sends"]:
            chans.setdefault(c, {"from": [], "to": []})["from"].append(st["id"])
        for c in st["recvs"]:
            chans.setdefault(c, {"from": [], "to": []})["to"].append(st["id"])

    dangling = sorted(c for c, e in chans.items() if not e["from"] or not e["to"])

    fanin = {d: 0 for d in defs}
    for l in boot + body:
        for r in scan(l)[4]:
            if r in fanin:
                fanin[r] += 1

    return {"path": path, "strands": strands, "channels": chans,
            "definitions": defs, "fanin": fanin, "dangling": dangling,
            "n_strands": len(strands), "n_channels": len(chans),
            "n_definitions": len(defs),
            "n_pathways": sum(len(e["from"]) * len(e["to"]) for e in chans.values())}


def render(t):
    out = ["%s — %d strands, %d channels, %d definitions, %d pathways"
           % (t["path"], t["n_strands"], t["n_channels"], t["n_definitions"], t["n_pathways"])]
    for st in t["strands"]:
        wiring = " ".join(["↧%s" % c for c in st["recvs"]] + ["↥%s" % c for c in st["sends"]])
        out.append("  strand %d (row %d, %d glyphs) %s" % (st["id"], st["row"], st["glyphs"], wiring or "—"))
    for c, e in sorted(t["channels"].items()):
        out.append("  %s: %s → %s" % (c, e["from"] or "·", e["to"] or "·"))
    if t["fanin"]:
        out.append("  definitions: " + " ".join("%s×%d" % (d, n) for d, n in t["fanin"].items()))
    if t["dangling"]:
        out.append("  ⚠ dangling: " + " ".join(t["dangling"]))
    return "\n".join(out)


def diff(a, b):
    """What grew, what was pruned, between two versions."""
    out = []
    ds = b["n_strands"] - a["n_strands"]
    if ds:
        out.append("strands %+d (%d → %d)" % (ds, a["n_strands"], b["n_strands"]))
    new = sorted(set(b["channels"]) - set(a["channels"]))
    gone = sorted(set(a["channels"]) - set(b["channels"]))
    if new:
        out.append("channels formed: " + " ".join(new))
    if gone:
        out.append("channels pruned: " + " ".join(gone))
    nd = [d for d in b["definitions"] if d not in a["definitions"]]
    od = [d for d in a["definitions"] if d not in b["definitions"]]
    if nd:
        out.append("definitions added: " + " ".join(nd))
    if od:
        out.append("definitions removed: " + " ".join(od))
    if b["dangling"]:
        out.append("⚠ dangling after: " + " ".join(b["dangling"]))
    return "\n".join(out) or "no structural change"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("files", nargs="+")
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--diff", action="store_true", help="structural diff of two versions")
    args = ap.parse_args()
    ts = [topology(f) for f in args.files]
    if args.json:
        print(json.dumps(ts if len(ts) > 1 else ts[0], indent=2, ensure_ascii=False))
    elif args.diff:
        if len(ts) != 2:
            raise SystemExit("--diff takes exactly two files")
        print(diff(ts[0], ts[1]))
    else:
        print("\n".join(render(t) for t in ts))


if __name__ == "__main__":
    sys.exit(main())
