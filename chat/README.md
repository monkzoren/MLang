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
3. **Port** `8080` — what the proxy forwards to inside the container. It
   has to match what the grid is listening on or the proxy answers 502
   from a container reporting itself perfectly healthy. Change both at
   once with the `PORT` variable rather than only this field.
4. **Domain**: point its DNS `A` record at the server first, or the
   certificate cannot be issued.
5. **Persistent volume** at `/data`. Skip it and the bot forgets everything
   it was taught on every redeploy.

   The volume holds two files, and the second one is the point. `chat.ml` is
   what the bot has become; `seed.ml` is the image's program as it stood when
   that began. Because a lesson here is *a line of source*, persisting what
   the bot learned necessarily persists the code it learned in — so seeding
   only when the file is absent, which is the obvious thing to write, pins
   the bot to whatever version first landed on the volume and silently
   ignores every image after it.

   When a new image brings a different seed, those three files are exactly a
   three-way merge, and the loom already knows how to do that: `mlang merge
   base ours theirs` is the same `diff3` a patch goes through. New code
   lands, learned rules survive, and a genuine collision is reported rather
   than guessed at — in which case the volume is left exactly as it was and
   `seed.ml` is not advanced, so the next boot tries again. A bot still
   answering on slightly old code beats a bot that does not come up.
6. **Environment** → `TEACH_TOKEN` = a long random string you invent. It is
   the password for `POST /teach` and has nothing to do with any model; it is
   the whole thing standing between your bot and anyone who finds the URL.
   Optionally `MODEL_API_KEY` too: with it, the learner strand goes and finds
   out what the bot doesn't know; without it the bot still answers everything
   it has been taught, says so plainly when it doesn't, and never talks to
   anything outside your server.

   | variable | default | |
   |---|---|---|
   | `PORT` | `8080` | the grid, the healthcheck and the proxy all follow it |
   | `MLANG_HTTP_DEBUG` | unset | put the reason `⍆`/`⍄` could not connect on stderr |

   Paste the key carefully, or let `boot` do it for you: it strips whitespace,
   because a key copied into a deployment UI usually arrives with a trailing
   newline and an HTTP header cannot hold one. `⍄` now refuses such a value by
   name rather than reporting it as a network failure — which is what it did
   once, for an hour, to the person reading this.
   | `TEACH_TOKEN` | — | the password for `POST /teach`. Not a model key |
   | `MODEL_API_KEY` | — | unset ⇒ the learner is inert, by design |
   | `MODEL_NAME` | `deepseek-flash` | |
   | `MODEL_URL` | `api.deepseek.com/chat/completions` | |
   | `MODEL_DIALECT` | `openai` | or `anthropic` |
   | `MODEL_VERSION` | — | `anthropic` only |

   Each of the four model variables falls back on its own, so setting the
   model does not mean restating the endpoint it belongs to. To move to
   Claude, all four: `claude-opus-5`,
   `https://api.anthropic.com/v1/messages`, `anthropic`, `2023-06-01`.

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
  authentication or a Cloudflare Access policy in front. With a model key set
  this is also a bill: every question the bot does not recognise is one API
  call, and every answer is a new rule welded into the program.

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

Where it asks is `E`, a definition — `⟨url model version dialect⟩` — so the
provider is re-pointable through the loom on a running bot, without a
restart. Two dialects are spoken, because the key alone is not enough: a
provider is an endpoint, a header, a body shape and a place to look for the
answer, and the two families disagree on all four.

| `E3@` | endpoint | auth header | answer at |
|---|---|---|---|
| `«openai»` | `api.deepseek.com/chat/completions` | `Authorization: Bearer` | `choices[0].message.content` |
| `«anthropic»` | `api.anthropic.com/v1/messages` | `x-api-key` + `anthropic-version` | `content[0].text` |

The default is DeepSeek (`deepseek-flash` — note that `DeepSeek-V4.1-Flash`
is the *version* name and will 400 if you send it as the model). `E` is
built at weave time from `G`, the defaults in `chat.ml`, overlaid with
whichever of `⌂3@`–`⌂6@` the deployment passed — so the four environment
variables above change the provider without touching the program. `«openai»`
is not only DeepSeek: it is the shape most providers copied, so a base URL
and a model name are usually all a different one needs.

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

### The first skill in the seed

The rule table can now hold a skill directly: a reply that is a **quotation**
is run with the message, and what it leaves is the answer. The seed carries
one — `C`, a calculator — routed to by six rows, one per operator:

```
 ⟨«+» [C]⟩ ⟨«-» [C]⟩ ⟨«×» [C]⟩ ⟨«*» [C]⟩ ⟨«÷» [C]⟩ ⟨«/» [C]⟩
```

It keeps only the characters arithmetic is made of, so *what is 120+50* is
`120+50`, and answers in under a millisecond with no API call — for every
sum, including the ones nobody has asked yet. What it cannot read it
declines with `∅`, meaning *not mine*, and the next matching rule gets the
turn; `∅` from every rule means the grid does not know and the learner is
asked. A skill that glitches counts as *not mine* too, not as the end of the
conversation.

The live comparison that motivated it, on a deployed bot with a model key:

```
5+5       → learned the fact «5+5 equals 10»       one API call, one row
120+50    → I do not know that yet                  another call, another row
```

Three facts and a claim it can do arithmetic, and it could not. With `C`,
the learned `5+5` row is never reached: the seed comes before anything
learned, and a skill before a fact about the same thing. (`5+5+5` still
falls through to that stale fact — a fact answering where a skill was
needed is precisely the failure the distinction is about.)

### Learning a skill on its own

With a model key, the learner no longer asks *answer this*; it asks **is
this a fact or a skill?** The model replies in JSON — `{"kind":"fact",
"answer":…}` or `{"kind":"skill","pattern":…,"code":…,"tests":[…]}` — with
the language reference (`mlang ops`, dropped into the image at build time)
in its system prompt. A fact becomes a row, as before. A skill goes through
`S`:

1. a row whose reply is the model's quotation is woven in with `⟡`
   (refused outright if it does not weave);
2. the model's own three tests are run against the **live grid**;
3. if any fails, the previous program is woven straight back.

The loom makes step 3 a transaction. A rejected skill leaves nothing behind
but a version in the log; the program is byte-identical to before. A skill
that passes then answers inputs it was never tested on — `reverse
parliament` after being tested on `reverse qwerty` — which is the whole
point, and the thing no number of rows could do.

`chat/learn_test.py` drives every path against a model that is a
dictionary: a fact, a skill that passes, one that fails its tests, one that
does not weave, and garbage. What it cannot tell you is how often a real
model writes a working skill in a language it has never seen. The gate makes
a bad answer cost nothing; it does not make good ones likely. Watch
`diff /data/seed.ml /data/chat.ml` after a few questions and see.

There is deliberately **no sandbox** on what a learned skill may do — it
runs with the grid's full powers, including `⍇`, `⍄` and `⌂` — because the
deployment this was built for sits behind an access whitelist. On an open
URL that would be a mistake: anyone who could chat could ask for a skill
that posts `⌂2@` somewhere. The place to add one is `S`, before `⟡`.

### Where lessons live

Learned rows — facts, skills, and `/teach` alike — go in `Q`, a second table
the image ships **empty and never writes inside**. `A` searches `R` then
`Q`. This is not tidiness: the seed and the memory must own different lines
or a new image cannot merge with an old volume. They did once share an
insertion point (both after `«bye»`), and `diff3` rightly called the
calculator and a volume's three lessons a conflict.

A volume from before `Q` existed carries its lessons inside `R`, and the
first redeploy after this change will conflict for exactly that reason;
`boot` keeps the old program and says so. Once: `rm /data/seed.ml`, then
restart. It backs up the old program, re-seeds, and every merge after that
is clean.

Two things a skill author has to know. **Locals belong to the calling
strand**: the body strand keeps its request in `r` and its answer in `a`,
so a definition that stores there answers the wrong request — `C` uses
`u v l f i j`. And **a local holding a quotation runs when named**, so a
quotation to be inspected or handed on stays on the stack (`∂⍙`), never in
a `⇒` local.

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
