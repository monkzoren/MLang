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
v0@200=[[n⌂1@⍈][⌫]⍥][]?             and, if it took, remember it
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
6. **Environment** → `TEACH_TOKEN` = a long random string.

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

**Memory is the limit, and it grows with the square of the lessons.** Every
version is kept whole, twice over: once as text, so `GET /.loom/vN` can
still answer, and once as lines, so a fault in code that arrived with
version 3 can excerpt *version 3's* line. Both are the loom working as
specified (§4.7). But a program that grows by a line per patch, kept once
per patch, is quadratic — 190 MB at 1600 lessons, and something like
2 GB by 5000.

**A restart is the compaction, and it costs nothing.** The bot boots from
the program it has become, which is the new `v0`: every rule is kept, the
version history is dropped, and memory returns to a few megabytes. For a
bot taught a handful of things a day this never comes up; if you are
teaching it thousands, redeploy occasionally and it stays small. What you
lose is the ability to read old versions back — the rules themselves are
all in the source.

One other ceiling: a patch arriving over HTTP is subject to the 16 MiB
request-body cap (§5.5). `⟡` does not go through HTTP, so the bot teaching
itself is not subject to it.

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
* **One scheduler.** The loom needs the deterministic one, so `--parallel` is
  out. For a personal bot that costs nothing.
