"""The world the machine lives in — a deterministic gridworld delivery task.

The world speaks the MLang replay protocol (SPEC 5.5) over a pipe: it writes
a sensor tick as a request frame on the grid's stdin and reads the action
back as a response frame on its stdout. Because that is the *same* stream
the conformance corpus pins, an episode driven interactively here replays
byte-for-byte offline afterwards -- which is what makes the gate of M3
possible. Run to live, replay to prove.

Sensor tick (the request body), eleven space-separated integers:

    x y carrying  north east south west  px py  dx dy

The compass fields are 1 when that neighbour is a wall. The robot is NOT
told about slip tiles: they are the world's latent structure, the thing a
naive OS does not model and a grown one might.

Action (the response body): one of N S E W G D X.
"""

import argparse
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
MLANG = os.environ.get("MLANG_BIN", os.path.join(ROOT, "compiler", "target", "release", "mlang"))

MAP = [
    "########",
    "#......#",
    "#.##.#.#",
    "#....#.#",
    "#.#....#",
    "#.#.##.#",
    "#......#",
    "########",
]

# Latent structure: stepping onto one of these tiles carries the machine one
# further cell in the same direction (if that cell is free). Nothing in the
# sensor frame reveals them; only the consequences are observable.
SLIP = {(4, 1), (4, 3), (2, 6), (5, 6)}

# The job queue, cycled. Fixed, so an episode is a pure function of the OS.
JOBS = [((6, 6), (1, 1)), ((1, 6), (6, 1)), ((4, 4), (6, 6)),
        ((1, 3), (3, 6)), ((6, 4), (1, 1)), ((3, 6), (6, 4))]

START = (1, 1)
DELTA = {"N": (0, -1), "S": (0, 1), "E": (1, 0), "W": (-1, 0)}


def wall(x, y):
    return not (0 <= y < len(MAP) and 0 <= x < len(MAP[y])) or MAP[y][x] == "#"


def _validate():
    """Every job cell and slip tile must be a free cell reachable from START.

    A job the machine cannot reach stalls the episode at score zero and looks
    exactly like a policy failure, so the world checks itself at import.
    """
    from collections import deque
    seen = {START}
    q = deque([START])
    while q:
        x, y = q.popleft()
        for dx, dy in DELTA.values():
            n = (x + dx, y + dy)
            if not wall(*n) and n not in seen:
                seen.add(n)
                q.append(n)
    for cell in SLIP:
        assert cell in seen, "slip tile %r is not a reachable free cell" % (cell,)
    for pick, drop in JOBS:
        for cell in (pick, drop):
            assert cell in seen, "job cell %r is not a reachable free cell" % (cell,)


_validate()


class World:
    def __init__(self, slip=True):
        self.x, self.y = START
        self.carrying = 0
        self.job = 0
        self.slip = slip
        self.delivered = 0
        self.bumps = 0
        self.bad = 0          # a grip/drop in the wrong place, or an unknown action
        self.ticks = 0
        self.slips = 0

    def pickup(self):
        return JOBS[self.job % len(JOBS)][0]

    def dropoff(self):
        return JOBS[self.job % len(JOBS)][1]

    def sense(self):
        px, py = self.pickup()
        dx, dy = self.dropoff()
        return "%d %d %d %d %d %d %d %d %d %d %d" % (
            self.x, self.y, self.carrying,
            int(wall(self.x, self.y - 1)), int(wall(self.x + 1, self.y)),
            int(wall(self.x, self.y + 1)), int(wall(self.x - 1, self.y)),
            px, py, dx, dy)

    def act(self, a):
        self.ticks += 1
        if a in DELTA:
            dx, dy = DELTA[a]
            nx, ny = self.x + dx, self.y + dy
            if wall(nx, ny):
                self.bumps += 1
                return
            self.x, self.y = nx, ny
            if self.slip and (nx, ny) in SLIP:
                sx, sy = nx + dx, ny + dy
                if not wall(sx, sy):
                    self.x, self.y = sx, sy
                    self.slips += 1
        elif a == "G":
            if not self.carrying and (self.x, self.y) == self.pickup():
                self.carrying = 1
            else:
                self.bad += 1
        elif a == "D":
            if self.carrying and (self.x, self.y) == self.dropoff():
                self.carrying = 0
                self.delivered += 1
                self.job += 1
            else:
                self.bad += 1
        elif a == "X":
            pass
        else:
            self.bad += 1

    def score(self):
        """Deliveries are the point; wasted motion and bad grips are the cost."""
        return self.delivered * 100 - self.bumps - self.bad * 5

    def summary(self):
        return ("ticks=%d delivered=%d bumps=%d bad=%d slips=%d score=%d"
                % (self.ticks, self.delivered, self.bumps, self.bad, self.slips, self.score()))


def read_frame(out):
    """Read one frame off the grid's stdout.

    Two kinds arrive on the same stream, which is the point: an action
    (`◁ id status type nbytes`, the body, a newline) and the loom's verdict
    on a patch (`⟡ status nbytes`, the report). Sensation and plasticity
    share a channel.
    """
    line = out.readline()
    if not line:
        return None, None
    parts = line.decode().split()
    if parts and parts[0] == "⟡":                      # a patch verdict
        n = int(parts[2])
        return "patch", (int(parts[1]), out.read(n).decode())
    if len(parts) < 5 or parts[0] != "◁":
        raise SystemExit("bad response frame: %r" % line)
    n = int(parts[4])
    body = out.read(n)
    out.read(1)                                        # the newline after the body
    return "act", body.decode()


def read_response(out):
    kind, v = read_frame(out)
    return None if kind is None else v


def episode(program, ticks, slip=True, record=None, patches=()):
    """Drive `program` through one episode.

    `patches` is a sequence of (tick, path): just before that tick the whole
    file at `path` is woven into the running grid as a \u27e1 frame, stamped with
    the version it was written against. The grid is never stopped.
    """
    p = subprocess.Popen([MLANG, "run", program], stdin=subprocess.PIPE,
                         stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    w = World(slip=slip)
    frames = bytearray()
    pending = {t: path for t, path in patches}
    base = 0
    w.versions = []
    try:
        for t in range(ticks):
            if t in pending:
                src = open(pending[t], "rb").read()
                frame = b"\xe2\x9f\xa1 %d %d\n%s" % (base, len(src), src)
                frames += frame
                p.stdin.write(frame)
                p.stdin.flush()
                kind, v = read_frame(p.stdout)
                if kind != "patch":
                    raise SystemExit("expected a patch verdict, got %r" % (v,))
                status, report = v
                w.versions.append((t, status, report.strip()))
                if status == 200:
                    base += 1
            body = w.sense().encode()
            frame = b"\xe2\x96\xb7 POST /tick %d\n%s\n" % (len(body), body)
            frames += frame
            p.stdin.write(frame)
            p.stdin.flush()
            a = read_response(p.stdout)
            if a is None:                       # the grid died mid-episode
                break
            w.act(a.strip())
        p.stdin.close()
    except BrokenPipeError:
        pass
    rest = p.stdout.read()
    err = p.stderr.read()
    p.wait()
    if record:
        os.makedirs(record, exist_ok=True)
        open(os.path.join(record, "episode.frames"), "wb").write(bytes(frames))
    return w, bytes(frames), err, p.returncode


def replay(program, frames_path):
    """Re-run the recorded stream offline. This must be byte-identical."""
    with open(frames_path, "rb") as f:
        r = subprocess.run([MLANG, "run", program], stdin=f, capture_output=True)
    return r.stdout, r.stderr, r.returncode


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("program", nargs="?", default=os.path.join(ROOT, "mos", "os0.ml"))
    ap.add_argument("--ticks", type=int, default=200)
    ap.add_argument("--no-slip", action="store_true", help="disable the world's latent structure")
    ap.add_argument("--record", metavar="DIR")
    ap.add_argument("--verify", metavar="DIR", help="record, then replay and require byte-identity")
    args = ap.parse_args()

    rec = args.verify or args.record
    w, frames, err, rc = episode(args.program, args.ticks, slip=not args.no_slip, record=rec)
    print(w.summary())
    if err:
        sys.stderr.write(err.decode(errors="replace"))

    if args.verify:
        fp = os.path.join(args.verify, "episode.frames")
        out, rerr, rrc = replay(args.program, fp)
        # The live drive interleaves reads and writes; the replay is one shot.
        # Determinism says the grid's own output must be identical either way.
        live = subprocess.run([MLANG, "run", args.program], stdin=open(fp, "rb"),
                              capture_output=True)
        open(os.path.join(args.verify, "episode.out"), "wb").write(out)
        ok = (out == live.stdout and rerr == live.stderr and rrc == live.returncode)
        print("replay: %s (%d bytes stdout, exit %d)"
              % ("byte-identical" if ok else "DIVERGED", len(out), rrc))
        return 0 if ok else 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
