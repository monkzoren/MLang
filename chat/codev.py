"""A co-development storm: many agents rewriting one running grid at once.

This is the loom's headline claim under load. Every writer pulls the live
program from /.loom, adds its own rule, and posts the whole file back stamped
with the version it started from. Two writers editing the same region get a
409 and the loser pulls and reapplies — nobody blocks, nobody locks, and the
grid never stops answering.

Readers chat throughout, against a program that is being rewritten under them
thousands of times. Their replies are checked, so a grid that started
answering nonsense would be caught rather than merely surviving.

    python3 chat/codev.py --writers 8 --readers 4 --patches 2000

Writes the program the grid became to --save, so it can be served afterwards.
"""

import argparse
import json
import os
import queue
import random
import statistics
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.request

for _k in ("HTTP_PROXY", "http_proxy", "HTTPS_PROXY", "https_proxy"):
    os.environ.pop(_k, None)

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
MLANG = os.environ.get("MLANG_BIN", os.path.join(ROOT, "compiler", "target", "release", "mlang"))
MARKER = "⟩≔R"

stop = threading.Event()
lock = threading.Lock()
stats = {"accepted": 0, "conflict": 0, "refused": 0, "error": 0,
         "reads": 0, "read_bad": 0, "weave_ms": [], "read_ms": []}


def http(base, path, body=None, timeout=120):
    req = urllib.request.Request(base + path, data=body,
                                 method="POST" if body is not None else "GET")
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return r.status, r.read().decode("utf-8", "replace")
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode("utf-8", "replace")


def writer(n, base, target, spread=False):
    """Pull, add one rule, weave it back. On a conflict, pull and reapply.

    With `spread`, each agent keeps its own block of the table and inserts
    after its own last rule rather than at the shared end. The loom merges
    line by line, so agents editing different lines never collide — the
    question this answers is whether the contention is the loom's or the
    program's.
    """
    mine = 0
    anchor = None
    while not stop.is_set():
        with lock:
            if stats["accepted"] >= target:
                return
        pattern = "w%02dr%05d" % (n, mine)
        tries = 0
        while not stop.is_set():
            tries += 1
            st, src = http(base, "/.loom")
            if st != 200 or MARKER not in src:
                with lock:
                    stats["error"] += 1
                return
            line = " ⟨«%s» «agent %d rule %d»⟩\n" % (pattern, n, mine)
            if spread and anchor and anchor in src:
                patched = src.replace(anchor, anchor + line.rstrip("\n") + "\n", 1)
            else:
                patched = src.replace(MARKER, line + MARKER, 1)
            t0 = time.time()
            st, rep = http(base, "/.loom", patched.encode())
            ms = (time.time() - t0) * 1000
            with lock:
                if st == 200:
                    stats["accepted"] += 1
                    stats["weave_ms"].append(ms)
                    anchor = line.rstrip("\n") + "\n"
                    break
                elif st == 409:
                    stats["conflict"] += 1
                elif st == 422:
                    stats["refused"] += 1
                    return
                else:
                    stats["error"] += 1
                    return
        mine += 1


def reader(base, known):
    """Chat while the program is being rewritten underneath."""
    while not stop.is_set():
        t0 = time.time()
        st, body = http(base, "/say", b"hello")
        ms = (time.time() - t0) * 1000
        with lock:
            stats["reads"] += 1
            stats["read_ms"].append(ms)
            # The seed rule must survive every patch: a grid that lost its
            # own definitions would still answer, just wrongly.
            if st != 200 or "I am a grid" not in body:
                stats["read_bad"] += 1
        time.sleep(0.002)


def rss_mb(pid):
    try:
        return int(open("/proc/%d/status" % pid).read().split("VmRSS:")[1].split()[0]) // 1024
    except Exception:
        return -1


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--writers", type=int, default=8)
    ap.add_argument("--readers", type=int, default=4)
    ap.add_argument("--patches", type=int, default=2000)
    ap.add_argument("--port", type=int, default=8081)
    ap.add_argument("--parallel", action="store_true", help="run the grid on threads")
    ap.add_argument("--spread", action="store_true",
                    help="each agent edits its own lines instead of a shared end")
    ap.add_argument("--mem-mb", type=int, default=9000, help="stop if the grid grows past this")
    ap.add_argument("--seconds", type=int, default=3600)
    ap.add_argument("--save", default=os.path.join(HERE, "grown.ml"))
    args = ap.parse_args()

    work = os.path.join("/tmp", "codev-%d" % args.port)
    os.makedirs(work, exist_ok=True)
    live = os.path.join(work, "chat.ml")
    with open(os.path.join(HERE, "chat.ml")) as f:
        src0 = f.read()
    with open(live, "w") as f:
        f.write(src0)

    cmd = [MLANG, "serve"] + (["--parallel"] if args.parallel else [])
    cmd += [live, str(args.port), "tok", live]
    grid = subprocess.Popen(cmd, stderr=subprocess.DEVNULL)
    base = "http://127.0.0.1:%d" % args.port
    for _ in range(200):
        try:
            if http(base, "/")[0]:
                break
        except Exception:
            time.sleep(0.05)
    else:
        raise SystemExit("the grid never came up")

    print("grid on %s  %s  %d writers  %d readers  target %d patches"
          % (base, "--parallel" if args.parallel else "deterministic",
             args.writers, args.readers, args.patches), flush=True)

    t0 = time.time()
    threads = [threading.Thread(target=writer, args=(i, base, args.patches, args.spread), daemon=True)
               for i in range(args.writers)]
    threads += [threading.Thread(target=reader, args=(base, None), daemon=True)
                for _ in range(args.readers)]
    for t in threads:
        t.start()

    print("%9s %9s %9s %8s %9s %9s" % ("elapsed", "accepted", "conflicts", "RSS MB", "weave ms", "reply ms"),
          flush=True)
    last = 0
    try:
        while any(t.is_alive() for t in threads):
            time.sleep(5)
            with lock:
                a, c = stats["accepted"], stats["conflict"]
                w = statistics.median(stats["weave_ms"][-500:]) if stats["weave_ms"] else 0
                r = statistics.median(stats["read_ms"][-500:]) if stats["read_ms"] else 0
            m = rss_mb(grid.pid)
            el = time.time() - t0
            if a != last:
                print("%8.0fs %9d %9d %8d %9.1f %9.1f" % (el, a, c, m, w, r), flush=True)
                last = a
            if a >= args.patches or m > args.mem_mb or el > args.seconds:
                stop.set()
                break
    except KeyboardInterrupt:
        stop.set()

    stop.set()
    for t in threads:
        t.join(timeout=10)
    el = time.time() - t0

    st, final = http(base, "/.loom")
    st2, log = http(base, "/.loom/log")
    rules = final.count("⟩\n") if st == 200 else 0
    with open(args.save, "w") as f:
        f.write("\n".join(l for l in final.splitlines() if not l.startswith("※ loom v")) + "\n")
    mem = rss_mb(grid.pid)
    grid.terminate()

    pct = lambda v, p: statistics.quantiles(v, n=100)[p - 1] if len(v) > 2 else 0
    print("\n── the grid after %d agents rewrote it %d times ──" % (args.writers, stats["accepted"]))
    print("  elapsed            %.0fs (%.0f patches/s)" % (el, stats["accepted"] / max(el, 1e-9)))
    print("  accepted           %d" % stats["accepted"])
    print("  conflicts (409)    %d  (%.1f%% of attempts, all retried)"
          % (stats["conflict"], 100 * stats["conflict"] / max(stats["accepted"] + stats["conflict"], 1)))
    print("  refused / errors   %d / %d" % (stats["refused"], stats["error"]))
    print("  weave ms           p50 %.1f  p99 %.1f" % (pct(stats["weave_ms"], 50), pct(stats["weave_ms"], 99)))
    print("  replies served     %d, %d wrong" % (stats["reads"], stats["read_bad"]))
    print("  reply ms           p50 %.1f  p99 %.1f" % (pct(stats["read_ms"], 50), pct(stats["read_ms"], 99)))
    print("  versions           %d" % (len(log.splitlines()) if st2 == 200 else -1))
    print("  source             %.1f KB, %d rules" % (len(final.encode()) / 1024, rules))
    print("  grid memory        %d MB" % mem)
    print("  saved              %s" % args.save)
    return 0 if stats["read_bad"] == 0 and stats["error"] == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
