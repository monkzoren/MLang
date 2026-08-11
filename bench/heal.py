"""The self-repair benchmark: can an LLM heal a broken program from its failure?

For each mutant the model sees the broken program, the byte-exact expected
output, and the failure exactly as the runtime announced it — a weave error
with coordinates, a glitch with coordinates, a proven deadlock wait-graph
(MLang), or a traceback / wrong bytes / hang note (Python). It replies with
a corrected program; the harness re-runs it against the identical golden;
repeat up to --rounds. A mutant is healed when stdout, stderr, and exit
code match the golden byte for byte. Deterministic runtime means a fix is
verified against the identical failure — no flaky reruns.

    python3 bench/heal.py --arm both --provider claude-cli \
        --model claude-haiku-4-5-20251001 --mutants 80 --rounds 3

Providers:
    claude-cli          headless `claude -p` (uses your Claude Code login)
    anthropic           Messages API, needs ANTHROPIC_API_KEY
    openai              Chat Completions API, needs OPENAI_API_KEY
    cmd:<shell-cmd>     prompt on stdin, completion on stdout

Results land in bench/results/heal-<arm>-<model>.json; render the table
with bench/report.py.
"""

import argparse
import json
import os
import re
import subprocess
import time
import urllib.request
from concurrent.futures import ThreadPoolExecutor

import common
import mutate

MLANG_PRIMER_NOTES = """MLang in brief: a concatenative stack language.
Values are pushed; every operation is ONE glyph that transforms the stack.
Numbers: 42, 2.5, negatives written ¯5 (¯ is part of the literal; - is
always binary subtraction). Strings: «…» (⏎ inside = newline). Lists:
⟨1 2 3⟩. ∅ is nil (also the end-of-stream marker on channels).
[…] quotes code as a value — control flow takes quotations.
Each LINE of the program is one strand (an independent machine); all
strands run concurrently and share nothing, communicating only over
one-letter named channels. A ⇊ line separates a boot section (runs first,
holds shared ≔ definitions) from the strands below. A line starting ⋮
continues the previous strand. ※ comments to end of line. A first line ⇓
means rain form: columns are strands, read downward.
"""


def mlang_primer():
    ops = subprocess.run([common.ensure_mlang(), "ops"],
                         capture_output=True, check=True)
    return MLANG_PRIMER_NOTES + "\nThe complete operation reference:\n" \
        + ops.stdout.decode()


def build_prompt(arm, mutant, attempts, primer):
    lang = "MLang" if arm == "mlang" else "Python 3"
    p = [f"You are repairing a broken {lang} program.\n"]
    if primer:
        p.append(primer)
    p.append("The program below is intended to produce EXACTLY the expected "
             "output (byte for byte, exit code 0), but it has a bug.\n")
    p.append(f"--- program ---\n{mutant['source']}\n")
    if mutant["stdin"]:
        p.append(f"--- stdin fed to the program ---\n{mutant['stdin']}")
    p.append(f"--- expected stdout ---\n{mutant['expected']['stdout']}")
    if mutant["expected"]["stderr"]:
        p.append(f"--- expected stderr ---\n{mutant['expected']['stderr']}")
    p.append(f"--- what actually happened ---\n{attempts[0]['failure']}\n")
    for i, a in enumerate(attempts[1:], 1):
        p.append(f"--- your attempt {i} ---\n{a['source']}\n")
        p.append(f"--- which produced ---\n{a['failure']}\n")
    p.append("Reply with the complete corrected program in a single fenced "
             "code block and nothing else.")
    return "\n".join(p)


FENCE = re.compile(r"```[^\n]*\n(.*?)```", re.DOTALL)


def extract_program(reply):
    blocks = FENCE.findall(reply)
    text = blocks[-1] if blocks else reply
    return text.strip("\n")


# ------------------------------------------------------------------ providers

def retrying(fn, tries=3, base_delay=5):
    for attempt in range(tries):
        try:
            return fn()
        except Exception:
            if attempt == tries - 1:
                raise
            time.sleep(base_delay * (attempt + 1))


def complete_claude_cli(model, prompt):
    def call():
        p = subprocess.run(["claude", "-p", "--model", model],
                           input=prompt.encode(), capture_output=True,
                           timeout=600)
        if p.returncode != 0:
            raise RuntimeError(f"claude -p failed: {p.stderr.decode()[:500]}")
        return p.stdout.decode()
    return retrying(call)


def http_json(url, headers, body):
    def call():
        req = urllib.request.Request(
            url, json.dumps(body).encode(),
            {"content-type": "application/json", **headers})
        with urllib.request.urlopen(req, timeout=600) as r:
            return json.load(r)
    return retrying(call)


def complete_anthropic(model, prompt):
    data = http_json(
        "https://api.anthropic.com/v1/messages",
        {"x-api-key": os.environ["ANTHROPIC_API_KEY"],
         "anthropic-version": "2023-06-01"},
        {"model": model, "max_tokens": 4096,
         "messages": [{"role": "user", "content": prompt}]})
    return "".join(b.get("text", "") for b in data["content"])


def complete_openai(model, prompt):
    data = http_json(
        "https://api.openai.com/v1/chat/completions",
        {"authorization": "Bearer " + os.environ["OPENAI_API_KEY"]},
        {"model": model,
         "messages": [{"role": "user", "content": prompt}]})
    return data["choices"][0]["message"]["content"]


def completer(provider, model):
    if provider == "claude-cli":
        return lambda prompt: complete_claude_cli(model, prompt)
    if provider == "anthropic":
        return lambda prompt: complete_anthropic(model, prompt)
    if provider == "openai":
        return lambda prompt: complete_openai(model, prompt)
    if provider.startswith("cmd:"):
        cmd = provider[4:]
        def run_cmd(prompt):
            p = subprocess.run(cmd, shell=True, input=prompt.encode(),
                               capture_output=True, timeout=600)
            return p.stdout.decode()
        return run_cmd
    raise SystemExit(f"unknown provider {provider}")


# ---------------------------------------------------------------- the corpus

def pick_mutants(arm, count, seeds, jobs, cases_filter=None, max_chars=400):
    """A stratified corpus of verified-broken mutants: round-robin across
    cases so no single program dominates."""
    if arm == "mlang":
        cases = common.load_corpus(max_source_chars=None if cases_filter
                                   else max_chars)
        run, classify = common.run_mlang, common.classify_mlang
    else:
        cases = common.load_python_corpus()
        run, classify = common.run_python, common.classify_python
    if cases_filter:
        cases = [c for c in cases if c["name"] in cases_filter]
        assert cases, f"no cases match {sorted(cases_filter)}"

    candidates = mutate.make_mutants(arm, cases, seeds)

    def label(m):
        r = run(m["source"], m["stdin"], timeout=3.0)
        m["first_outcome"] = classify(r, m["expected"])
        m["first_failure"] = common.failure_report(r)
        return m

    with ThreadPoolExecutor(max_workers=jobs) as pool:
        labeled = list(pool.map(label, candidates))
    broken = [m for m in labeled if m["first_outcome"] != "pass"]

    by_case = {}
    for m in broken:
        by_case.setdefault(m["case"], []).append(m)
    picked, rank = [], 0
    while len(picked) < count:
        added = False
        for name in sorted(by_case):
            if rank < len(by_case[name]) and len(picked) < count:
                picked.append(by_case[name][rank])
                added = True
        if not added:
            break
        rank += 1
    return picked


# --------------------------------------------------------------- the loop

def heal_one(arm, mutant, rounds, complete, primer):
    run = common.run_mlang if arm == "mlang" else common.run_python
    classify = common.classify_mlang if arm == "mlang" else common.classify_python
    attempts = [{"source": mutant["source"],
                 "outcome": mutant["first_outcome"],
                 "failure": mutant["first_failure"]}]
    healed_round = None
    for r in range(1, rounds + 1):
        prompt = build_prompt(arm, mutant, attempts, primer)
        try:
            reply = complete(prompt)
        except Exception as e:
            attempts.append({"source": "", "outcome": "provider-error",
                             "failure": str(e)[:500]})
            break
        patch = extract_program(reply)
        result = run(patch, mutant["stdin"])
        outcome = classify(result, mutant["expected"])
        attempts.append({"source": patch, "outcome": outcome,
                         "failure": common.failure_report(result)})
        if outcome == "pass":
            healed_round = r
            break
    return {"id": mutant["id"], "case": mutant["case"], "op": mutant["op"],
            "first_outcome": mutant["first_outcome"],
            "healed": healed_round is not None,
            "rounds": healed_round,
            "attempts": attempts}


def run_arm(arm, args, complete):
    primer = mlang_primer() if arm == "mlang" else None
    cases_filter = set(args.cases.split(",")) if args.cases else None
    corpus = pick_mutants(arm, args.mutants, args.seeds, args.jobs,
                          cases_filter, args.max_chars)
    print(f"[{arm}] healing {len(corpus)} mutants "
          f"(≤{args.rounds} rounds, {args.model})")
    done = [0]

    def work(m):
        rec = heal_one(arm, m, args.rounds, complete, primer)
        done[0] += 1
        state = f"healed r{rec['rounds']}" if rec["healed"] else "NOT healed"
        print(f"[{arm}] {done[0]:3d}/{len(corpus)} {rec['id']:40s} "
              f"{rec['first_outcome']:13s} {state}", flush=True)
        return rec

    with ThreadPoolExecutor(max_workers=args.llm_jobs) as pool:
        records = list(pool.map(work, corpus))

    healed = [r for r in records if r["healed"]]
    out = {"meta": {"arm": arm, "provider": args.provider, "model": args.model,
                    "rounds_max": args.rounds, "seeds": args.seeds,
                    "mutants": len(records)},
           "healed": len(healed),
           "records": records}
    os.makedirs(os.path.join(common.BENCH, "results"), exist_ok=True)
    slug = re.sub(r"[^a-z0-9.-]+", "-", args.model.lower())
    if args.tag:
        slug = f"{args.tag}-{slug}"
    path = os.path.join(common.BENCH, "results", f"heal-{arm}-{slug}.json")
    with open(path, "w") as f:
        json.dump(out, f, indent=1)
        f.write("\n")
    print(f"[{arm}] {len(healed)}/{len(records)} healed → {path}")
    return path


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--arm", choices=["mlang", "python", "both"], default="both")
    ap.add_argument("--provider", default="claude-cli")
    ap.add_argument("--model", default="claude-haiku-4-5-20251001")
    ap.add_argument("--mutants", type=int, default=80)
    ap.add_argument("--rounds", type=int, default=3)
    ap.add_argument("--seeds", type=int, default=6,
                    help="mutation seeds per program when building the corpus")
    ap.add_argument("--cases", default=None,
                    help="comma-separated case names to target (e.g. "
                         "example:oracle.ml for the MLang arm, oracle for "
                         "the Python arm); lifts --max-chars")
    ap.add_argument("--max-chars", type=int, default=400,
                    help="MLang arm: skip programs longer than this "
                         "(ignored when --cases is given)")
    ap.add_argument("--tag", default=None,
                    help="prefix for the results file name, e.g. oracle")
    ap.add_argument("--jobs", type=int, default=8,
                    help="parallel program runs (labeling/verification)")
    ap.add_argument("--llm-jobs", type=int, default=4,
                    help="parallel LLM calls")
    args = ap.parse_args()

    common.ensure_mlang()
    complete = completer(args.provider, args.model)
    arms = ["mlang", "python"] if args.arm == "both" else [args.arm]
    for arm in arms:
        run_arm(arm, args, complete)


if __name__ == "__main__":
    main()
