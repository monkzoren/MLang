"""The public face of the grid, and the only thing that may re-weave it.

`mlang serve` binds 127.0.0.1 on purpose (SPEC 5.5), so the grid and the
loom it opens are unreachable from outside the container. That is the whole
security model here: this process is the only door.

    the world  ──▶  :PORT  this process  ──▶  127.0.0.1:GRID_PORT  the grid
                              │                       │
                              └── POST /teach ────────▶ /.loom   (never proxied)

Everything a visitor says is answered by the grid — no model in the hot
path, just a substring match over the rule table. Teaching is the slow path:
it pulls the live source, adds one line to the rule table, and weaves it back
while the grid keeps serving. An accepted patch is also written to disk, so a
container restart resumes from what the bot has learned rather than from v0.
"""

import http.server
import json
import os
import shutil
import socketserver
import subprocess
import sys
import time
import urllib.error
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
PORT = int(os.environ.get("PORT", "8080"))
GRID_PORT = int(os.environ.get("GRID_PORT", "4321"))
MLANG = os.environ.get("MLANG_BIN", "mlang")
SEED = os.environ.get("SEED_SOURCE", os.path.join(HERE, "chat.ml"))
# Persist the evolved program, not the seed: a restart should keep what it learned.
LIVE = os.environ.get("LIVE_SOURCE", "/data/chat.ml")
TOKEN = os.environ.get("TEACH_TOKEN", "")
GRID = "http://127.0.0.1:%d" % GRID_PORT
MAX_FIELD = 300


def grid(path, method="GET", body=None, timeout=10):
    req = urllib.request.Request(GRID + path, data=body, method=method)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return r.status, r.read()
    except urllib.error.HTTPError as e:
        return e.code, e.read()


def start_grid():
    """Boot the grid from what it has learned, falling back to the seed."""
    os.makedirs(os.path.dirname(LIVE), exist_ok=True)
    if not os.path.exists(LIVE):
        shutil.copy(SEED, LIVE)
        print("seeded %s from %s" % (LIVE, SEED), flush=True)
    p = subprocess.Popen([MLANG, "serve", LIVE, str(GRID_PORT)])
    for _ in range(100):                       # wait for the listener
        if p.poll() is not None:
            raise SystemExit("the grid exited at startup (code %s)" % p.returncode)
        try:
            if grid("/")[0]:
                return p
        except Exception:
            time.sleep(0.1)
    raise SystemExit("the grid never came up on %s" % GRID)


def clean(text):
    """A rule is spliced into MLang string literals, so « and » cannot pass."""
    t = " ".join(text.split())
    if not t or len(t) > MAX_FIELD:
        return None
    if "«" in t or "»" in t or "⏎" in t:
        return None
    return t


def teach(pattern, reply):
    """Add one rule to the table on the running grid, then persist it."""
    pattern, reply = clean(pattern), clean(reply)
    if not pattern or not reply:
        return 400, "a pattern and a reply are required, without « » or ⏎"

    status, src = grid("/.loom")
    if status != 200:
        return 502, "could not read the live program"
    text = src.decode()

    marker = "⟩≔R"
    if marker not in text:
        return 500, "the rule table is not where this expects it"
    if ("«%s»" % pattern.lower()) in text:
        return 409, "there is already a rule for %r" % pattern

    line = " ⟨«%s» «%s»⟩\n" % (pattern.lower(), reply)
    patched = text.replace(marker, line + marker, 1)

    # The loom weaves the whole file before accepting it, so a patch that
    # would not compile changes nothing and says why (422).
    status, report = grid("/.loom", "POST", patched.encode(), timeout=30)
    detail = report.decode(errors="replace").strip()
    if status != 200:
        return status, detail

    with open(LIVE, "w") as f:                 # survive a restart
        f.write(patched)
    return 200, detail


class Handler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def send(self, status, body, ctype="text/plain; charset=utf-8"):
        raw = body.encode() if isinstance(body, str) else body
        self.send_response(status)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(raw)))
        self.end_headers()
        self.wfile.write(raw)

    def authorised(self):
        if not TOKEN:
            self.send(503, "teaching is disabled: set TEACH_TOKEN")
            return False
        if self.headers.get("X-Teach-Token") != TOKEN:
            self.send(403, "bad or missing X-Teach-Token")
            return False
        return True

    def proxy(self, method):
        # The loom is container-internal. It is never reachable from here.
        if self.path.startswith("/.loom"):
            return self.send(404, "not found")
        n = int(self.headers.get("Content-Length") or 0)
        body = self.rfile.read(n) if n else None
        try:
            status, out = grid(self.path, method, body)
        except Exception as e:
            return self.send(502, "the grid is not answering (%s)" % type(e).__name__)
        ctype = "text/html; charset=utf-8" if self.path == "/" else "text/plain; charset=utf-8"
        self.send(status, out, ctype)

    def do_GET(self):
        if self.path == "/versions":                 # what the bot has learned
            if not self.authorised():
                return
            status, out = grid("/.loom/log")
            return self.send(status, out)
        self.proxy("GET")

    def do_POST(self):
        if self.path == "/teach":
            if not self.authorised():
                return
            n = int(self.headers.get("Content-Length") or 0)
            try:
                payload = json.loads(self.rfile.read(n) or b"{}")
            except ValueError:
                return self.send(400, "expected JSON: {\"pattern\": …, \"reply\": …}")
            status, detail = teach(payload.get("pattern", ""), payload.get("reply", ""))
            return self.send(status, detail)
        self.proxy("POST")

    def log_message(self, fmt, *args):
        sys.stderr.write("%s %s\n" % (self.address_string(), fmt % args))


class Server(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True


if __name__ == "__main__":
    child = start_grid()
    print("grid on %s — public on :%d — teaching %s"
          % (GRID, PORT, "on" if TOKEN else "OFF (set TEACH_TOKEN)"), flush=True)
    try:
        Server(("0.0.0.0", PORT), Handler).serve_forever()
    finally:
        child.terminate()
