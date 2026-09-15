#!/usr/bin/env python3
"""The learner, end to end, against a model that is a dictionary.

Every path the model's reply can take: a fact becomes a row; a skill that
passes its own tests is installed and then answers inputs it was never tested
on; a skill that fails them leaves the program byte-identical; a skill that
does not weave is refused by the loom; garbage teaches nothing and the grid
keeps answering. Run: python3 chat/learn_test.py
"""
import http.server, json, os, shutil, subprocess, sys, tempfile, threading, time, urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
MLANG = os.environ.get("MLANG_BIN", os.path.join(ROOT, "compiler", "target", "release", "mlang"))

# What the "model" says for each question it is asked.
SCRIPT = {
    "what is a pangolin": {"kind": "fact", "answer": "A scaly anteater."},
    "reverse qwerty": {
        "kind": "skill", "pattern": "reverse ",
        "code": "[⇒u u«reverse »⊆⇒v v#2=[v1@⌽][∅]?]",
        "tests": [["reverse qwerty", "ytrewq"], ["reverse abc", "cba"], ["reverse mlang", "gnalm"]],
    },
    "shout qwerty": {                      # claims to upper-case, but its tests lie
        "kind": "skill", "pattern": "shout ",
        "code": "[⇒u u«shout »⊆1@]",
        "tests": [["shout qwerty", "QWERTY"], ["shout a", "A"], ["shout ok", "OK"]],
    },
    "twist qwerty": {                    # does not weave: an unclosed quotation
        "kind": "skill", "pattern": "twist ",
        "code": "[⇒u u⌽",
        "tests": [["twist ab", "ba"], ["twist x", "x"], ["twist yz", "zy"]],
    },
    "gibberish please": "I am not JSON at all, and no fences either",
}
asked = []

class Model(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["content-length"])))
        q = body["messages"][-1]["content"]
        asked.append(q)
        reply = SCRIPT.get(q, {"kind": "fact", "answer": "No idea."})
        text = reply if isinstance(reply, str) else "```json\n" + json.dumps(reply, ensure_ascii=False) + "\n```"
        out = json.dumps({"choices": [{"finish_reason": "stop", "message": {"content": text}}]}).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(out)))
        self.end_headers()
        self.wfile.write(out)
    def log_message(self, *a):
        pass

def main():
    srv = http.server.HTTPServer(("127.0.0.1", 0), Model)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    d = tempfile.mkdtemp()
    shutil.copy(os.path.join(ROOT, "chat", "chat.ml"), os.path.join(d, "chat.ml"))
    env = dict(os.environ, MLANG_HOST="127.0.0.1")
    for k in ("HTTP_PROXY", "http_proxy", "HTTPS_PROXY", "https_proxy"):
        env.pop(k, None)
    port = 8400 + os.getpid() % 100
    p = subprocess.Popen([MLANG, "serve", "--parallel", os.path.join(d, "chat.ml"), str(port), "tok",
                          os.path.join(d, "chat.ml"), "sk-fake", "m", f"http://127.0.0.1:{srv.server_port}/v", "openai", ""],
                         env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    time.sleep(1.5)
    fail = 0
    def say(q):
        r = urllib.request.Request(f"http://127.0.0.1:{port}/say", data=q.encode(), method="POST")
        return urllib.request.urlopen(r, timeout=10).read().decode().strip()
    def check(name, want, got):
        nonlocal fail
        ok = want in got
        fail += not ok
        print("  %-4s %s" % ("ok" if ok else "FAIL", name))
        if not ok:
            print("       wanted: %s\n       got:    %s" % (want, got[:160].replace("\n", " ")))
    def source():
        return open(os.path.join(d, "chat.ml"), encoding="utf-8").read()
    try:
        # a fact
        say("what is a pangolin"); time.sleep(1.0)
        check("a fact becomes a row", "A scaly anteater.", say("what is a pangolin"))

        # a skill that passes its tests
        # (inputs avoid the seed's patterns: «hello» and «hi» match inside
        # longer words — the greediness chat/README.md warns about)
        say("reverse qwerty"); time.sleep(1.0)
        check("a skill is installed", "ytrewq", say("reverse qwerty"))
        check("and generalises to an input it was never tested on", "tnemailrap", say("reverse parliament"))
        check("a fact is one row in the table", "«what is a pangolin»", source())
        check("a skill is a quotation in the table", "[⇒u u«reverse »⊆", source())

        # a skill that fails its tests: nothing changes
        before = source()
        say("shout qwerty"); time.sleep(1.0)
        check("a skill that fails its own tests is not kept", "yes", "yes" if "shout" not in source() else "no")
        check("and the program is byte-identical to before", "yes", "yes" if source() == before else "no: differs")

        # a skill that does not weave
        say("twist qwerty"); time.sleep(1.0)
        check("a skill that does not weave is refused", "yes", "yes" if "twist" not in source() else "no")

        # garbage
        say("gibberish please"); time.sleep(1.0)
        check("garbage teaches nothing", "yes", "yes" if "gibberish" not in source() else "no")
        check("and the grid keeps answering", "Hello. I am a grid", say("hello"))
        check("the calculator still outranks everything learned", "170", say("120+50"))
    finally:
        p.terminate(); p.wait()
        err = p.stderr.read().decode()
        if "glitch" in err or "deadlock" in err:
            print("STDERR:\n" + err[:1500]); fail += 1
    print("%d asked of the model: %s" % (len(asked), asked))
    return fail

if __name__ == "__main__":
    sys.exit(main())
