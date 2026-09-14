# A chatbot that is a grid

Every reply this bot gives is computed by [`chat.ml`](chat.ml). So is every
lesson it learns. There is no model, no interpreter, no web server and no
sidecar — the program *is* the application, and it rewrites itself while it
is answering.

```
$ curl -XPOST --data 'what is coolify' https://your.domain/say
I do not know that one yet. You can teach me it.

$ curl -XPOST --data "$TOKEN|coolify|A self-hosted PaaS. It runs Docker." https://your.domain/teach
⟡ v1: 1 definition rebound (R)

$ curl -XPOST --data 'What is Coolify then' https://your.domain/say
A self-hosted PaaS. It runs Docker.
```

Nothing restarted between the second and third command, and nothing outside
the grid took part in the second.

## How it rewrites itself

`⟐` hands a strand the program the grid is currently running. `⟡` weaves a
new one in (SPEC §4.7). So teaching is four steps, all of them MLang:

```
« ⟨»L⧺p⧺Y⧺« »⧺L⧺w⧺Y⧺«⟩⏎»⧺M⧺⇒c      the new rule, as a line of me
⟐M⊆c⊇⇒n                             my source with it spliced in
n⟡⇒v                                weave it — ⟨status report⟩ back
```

Two details are the whole trick. `L` and `Y` are `171⍘` and `187⍘`, because a
string literal cannot contain the guillemets that delimit it — the bot has to
build `«` and `»` from their code points to write a rule. And `M`, the marker
naming the end of the rule table, is `«⟩≔»«R»⧺`: assembled at runtime so the
marker never appears contiguously in the source that searches for it.
Splitting on a literal would have rewritten the code doing the splitting.

Everything the loom does to a patch arriving from outside it does to this
one: the whole text is woven first, a text that does not weave is refused and
**the running grid is untouched**, and the strand keeps its old code until its
next seam. A bad lesson cannot half-land.

## Remembering

After every answer the bot compares what it *is* to what it last wrote down,
and writes itself to `⌂1@` if they differ:

```
⌂#2≥[⟐∂[y][⌫«»]⍥≠[∂⌂1@⍈⇒y][⌫]?][]?
```

**That file is its memory.** A restart boots from what the bot has become,
and that becomes the new `v0`.

It checks after every request rather than only when taught, because a rule
can arrive from another agent on `/.loom` and the bot is never told. Before
this, every one of the 3,750 rules from the co-development storm went
unpersisted.

The local is read through `⍥` for a reason worth knowing before you patch
anything: **a strand woven into a running grid resumes *inside* its outermost
loop, so its prelude never runs again.** Initialising the bookkeeping local
before the loop killed the strand with `undefined sigil` the moment the patch
landed. `⟲` migrations exist for exactly that; `⍥` avoids needing one, by
giving the local its first value where it is read.

The other way to lose a grid is quieter. **A patch is a whole-file statement
of intent.** Build one from a stale copy while stamping it with a recent
base, and `diff3` reads it as deleting every line added since — one rule
vanished that way, with no conflict and no error, because the loom did
exactly what it was told. The stamp has to match the text you edited.

## Checking it

```sh
sh chat/test.sh          # replay-mode checks: no network, no container
```

`⎆` reads `▷` frames from stdin, so the whole bot is a pure function of its
input and the tests need nothing running.

## Running it

```sh
mlang serve chat.ml 8080 TOKEN /data/chat.ml
```

* `⌂0@` — the token `POST /teach` must carry.
* `⌂1@` — where to save the program it becomes, so a restart remembers.
* `MLANG_HOST=0.0.0.0` — bind the public interface.

| | |
|---|---|
| `GET /` | the chat page |
| `POST /say` | a message in, a reply out |
| `POST /teach` | `token|pattern|reply` |

## Getting a URL

Coolify *is* Docker — it orchestrates Docker Compose — so a Dockerfile is the
normal path. It also runs a reverse proxy in front of your containers, assigns
the domain, and gets a Let's Encrypt certificate. You do not configure TLS and
you do not publish a port on the host.

1. **New Resource → your Git repository**.
2. **Build Pack: Dockerfile**, file `chat/Dockerfile`, **build context the
   repository root** — the build welds `std/*.ml` into the binary with
   `include_str!`, so it needs `compiler/` *and* `std/`.
3. **Port** `8080` — what the proxy forwards to inside the container.
4. **Domain**: point its DNS `A` record at the server first, or the
   certificate cannot be issued.
5. **Persistent volume** at `/data`. Skip it and the bot forgets everything
   it was taught on every redeploy.
6. **Environment** → `TEACH_TOKEN` = a long random string. Optionally
   `ANTHROPIC_API_KEY` too: with it, the learner strand goes and finds out
   what the bot doesn't know; without it the bot still answers everything it
   has been taught and simply never learns on its own.

The image is two stages: `rust:1-slim` builds the toolchain, and the runtime
is `debian:stable-slim` carrying one ~4 MB binary and one `.ml` file. Even the
healthcheck is MLang (`mlang eval '«http://127.0.0.1:8080/»⍆⌫'`).

## Why binding 0.0.0.0 is safe here

`mlang serve` binds the loopback by default, and `MLANG_HOST` widens that. A
grid reachable from off the machine keeps its loom's HTTP routes **shut**
unless `MLANG_LOOM=1` asks for them by name — publishing a port should not
quietly publish the power to replace the program behind it. `/.loom` answers
404 from outside no matter what the domain is.

`⟐` and `⟡` are unaffected, which is exactly the shape wanted: **the bot can
rewrite itself, and nobody else can rewrite it.**

What still needs care:

* **Chatting is open** to anyone with the URL. Teaching is not, and the token
  is the whole defence. Rotate it by changing the environment variable and
  redeploying.
* **The token lives in the browser** once typed into the teach form
  (`localStorage`) — the trade that makes teaching from a phone practical.
  Use a private device, or teach with `curl`.
* **No rate limiting.** If the URL leaks somewhere noisy, put Coolify's proxy
  authentication or a Cloudflare Access policy in front.

## Is it *entirely* MLang?

The bot is. One line in the `Dockerfile` is not: `sh -c` copies the seed
program into `/data` on first boot, because a volume is mounted over whatever
the image put there. A program cannot seed the file it is about to be started
from. Everything after that first copy — serving, answering, validating,
rewriting itself, persisting — happens in `chat.ml`.

## How much can it learn?

There is no cap on how many times a grid can be re-woven — faults are
capped at 64, versions are not — and time is not the limit either. Teaching
400 rules and then 1600, measured on the live bot:

| lessons | source | memory | weave | reply |
|---|---|---|---|---|
| 100 | 8 KB | 6 MB | 1.6 ms | 0.5 ms |
| 400 | 19 KB | 21 MB | 2.3 ms | 0.8 ms |
| 800 | 33 KB | 57 MB | 2.8 ms | 1.0 ms |
| 1600 | 61 KB | 190 MB | 6.1 ms | 1.3 ms |

A reply stays about a millisecond with 1600 rules, and a lesson lands in
single-digit milliseconds. The source grows linearly, ~36 bytes a rule.

Memory used to be the limit and used to grow with the square of the lessons,
because every version was kept whole. Only a window of recent versions keeps
its source now (`MLANG_LOOM_KEEP`, default 64), which makes it linear:

| lessons | before | after |
|---|---|---|
| 400 | 21 MB | 10 MB |
| 800 | 57 MB | 15 MB |
| 1600 | 190 MB | **26 MB** |

Nothing the bot *knows* is lost by that — its rules are in its current
source. What is bounded is reading back what it used to be: `GET /.loom/log`
still lists every version it has ever been, but `GET /.loom/vN` outside the
window answers 404, and a patch written against a version that old is
refused with a note to pull and reapply.

**A restart still compacts it completely.** The bot boots from the program
it has become, which is the new `v0`: every rule kept, the version log
dropped, memory back to a few megabytes.

One other ceiling: a patch arriving over HTTP is subject to the 16 MiB
request-body cap (§5.5). `⟡` does not go through HTTP, so the bot teaching
itself is not subject to it.

## Asking when it does not know

The bot has two strands. The body serves; the **learner** waits on a channel:

```
body     ⎆ → dispatch → ⍅ ;  nothing matched → ↥λ the question
learner  [↧λ … ⍄ the model … ⟡ weave the answer in …]⟳
```

Ask it something it has never heard and it answers *at once* — "I am finding
out, ask me again shortly" — hands the question to the learner over `λ`, and
goes back to talking. The learner asks the model with `⍄`, scrubs the answer,
and weaves it in as a new rule. Ask again and it knows.

```
what is a pangolin   → I do not know that yet. I am finding out.      4 ms
  (meanwhile)  hello → Hello. I am a grid…                            1 ms
  (meanwhile)  hello → Hello. I am a grid…                            1 ms
what is a pangolin   → (the answer, now woven in)                     1 ms
```

**Run it on threads or that table is a lie.** Under the deterministic
scheduler every strand shares one thread, so the learner's blocking call
stalls the grid: the same experiment shows the first "meanwhile" reply taking
**6003 ms** — exactly the model's latency. On threads it takes 1 ms. This is
the whole reason the two-strand split exists, and the whole reason the loom
had to be made to work on threads: the learner patches the grid from its own
thread while the body keeps answering.

Where it asks is `E`, a definition — so the endpoint and the model are
re-pointable through the loom on a running bot, without a restart.

### The gate, and the absence of one

The API key reaches the bot as `⌂2@`. **Without it the learner is inert** —
that is the off switch, and the default.

With it, the bot asks a model and writes the answer into itself with nobody
judging the answer. Be clear-eyed about what that is: **the ungated arm of
`mos/README.md` §M4**, the one that drifted, with text in place of strands.
Two cheap gates, if you want one:

* require confirmation — the learner replies but does not weave until a
  person says so through `/teach`, which exists and already has a token;
* keep answers from a model in their own block of the table, so they can be
  pruned as a group and never shadow a rule a person wrote.

Neither is built. What is built is the off switch.

## Information, knowledge, and skill

It is worth being exact about what `/teach` does, because it is easy to
oversell.

* **Information** is the bytes — what a file says, what a sensor reads.
* **Knowledge** is a fact in the table: *when you hear `coolify`, say this*.
  It answers exactly one input and nothing else.
* **A skill** is a definition that *computes*. It answers an open set of
  inputs, including ones that did not exist when it was taught.

`POST /teach` writes knowledge. A thousand lessons leave the bot exactly as
capable as it started — richer, not abler. That is the same distinction
`mos/README.md` §M4 arrived at from the other end: a spliced pump adds
topology and not function, and a row in a table adds a fact and not an
ability.

A skill is one small patch away, and **cannot** go through `/teach`, because
`/teach` only writes data. You patch the program.
[`skills/read-file.md`](skills/read-file.md) walks through giving the bot the
ability to read a file — two pure definitions, woven while it serves. The
test that separates the two:

```
read /tmp/made-after-the-lesson   → (its contents)   a skill generalises
quagga                            → I do not know    a fact does not
```

This is the whole reason the medium is a program rather than a database. You
can add a fact to a key-value store; you cannot add *the ability to read a
file* to one.

## The co-development storm

`codev.py` puts the loom's headline claim under load: many agents rewriting
one running grid, with readers chatting throughout against a program being
rewritten under them. 12 writers, 6 readers, 180 seconds each:

| agents insert | rules gained | conflicts | memory |
|---|---|---|---|
| at a shared end | 1,789 | **90.8%** | 31 MB |
| each in its own block | **3,750** | **1.2%** | 56 MB |

Not one reply was wrong in either arm, across 12,000 of them, and not one
patch was lost — every 409 was retried and landed.

**The contention was this program's, not the loom's.** When every agent
inserts before the same `⟩≔R`, `diff3` sees both sides changing the same
region and refuses the loser every time. Give each agent its own block and
conflicts fall to near nothing and throughput doubles. Concurrency here
scales with the number of distinct *lines* being edited, not with the number
of agents — which is exactly what `docs/loom.md` claims, now measured.

[`grown.ml`](grown.ml) is what the grid became: 3,788 lines, 3,750 rules
learned from twelve agents at once. Serve it like any other program —

```sh
mlang serve chat/grown.ml 8080 "$TOKEN" /tmp/grown.ml
curl -XPOST --data 'w03r00100' localhost:8080/say   # → agent 3 rule 100
```

— and it keeps learning from there.

## Where a model would go, and why there isn't one

Not in the hot path, ever: a turn is a substring match and costs nothing.
Where one would help is *writing* the reply — `/teach` becoming "here is
something I could not answer, propose a rule" — through the identical `⟡` and
the identical refusals. Once per lesson, not once per message.

Deliberately absent, for a reason measured in `mos/README.md` §M3–M4:
self-modification without a fitness signal **drifts**. Ungated, it accumulated
structure that cost and did nothing, and lost score. Here the fitness signal
is a person deciding a rule is worth having. A model proposing rules with
nobody judging them reproduces the ungated arm.

## What this is not

* **Not autonomous.** It learns what it is taught — a feature until there is
  something better than a person to say whether a reply was good.
* **Substring matching is dumb**, on purpose. It is legible, it is fast, and
  every rule it holds can be read in its source. Ranking or embeddings can be
  added later — as strands, which is the interesting way.

  It is dumb in a way that will bite you, so know it up front: a short
  pattern matches *inside* longer words. The seed rule `hi` answers
  "anyt**hi**ng". Patterns are tried in table order and the newest is added
  last, so an old short rule shadows a new specific one. Teach whole words.

* **A reply cannot contain `|`**, which is the field separator `POST /teach`
  splits on — such a body is refused as malformed.
* **Run it on threads.** `--parallel` is not optional here: the learner
  blocks for the length of a model call, and on the deterministic scheduler
  that stalls every other conversation with it. The loom works on either.
