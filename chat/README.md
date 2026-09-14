# A chatbot that is a grid

Every reply this bot gives is computed by [`chat.ml`](chat.ml) — one MLang
program, a substring match over a table of rules. **There is no model in the
hot path.** A turn costs a few microseconds and nothing per message.

What makes it more than a lookup table is that the rule table is a
*definition*, and definitions are hot: the loom can rebind `R` on a grid that
is mid-conversation. Teaching the bot something new does not restart it, does
not drop a connection, and does not lose a conversation. The bot gains a
reflex the way `docs/loom.md` rebinds a greeting.

```
$ curl -XPOST --data 'what is coolify' :8080/say
I do not know that one yet. You can teach me it.

$ curl -XPOST -H "X-Teach-Token: $TOKEN" :8080/teach \
       -d '{"pattern":"coolify","reply":"A self-hosted PaaS. It runs Docker."}'
⟡ v1: 1 definition rebound (R)

$ curl -XPOST --data 'What is Coolify then' :8080/say
A self-hosted PaaS. It runs Docker.
```

## Shape

```
   the world ──▶ :8080  serve.py ──▶ 127.0.0.1:4321  the grid (chat.ml)
                           │                  │
                           └ POST /teach ─────▶ /.loom      never proxied
                           │
                           └ writes the accepted program to /data/chat.ml
```

`mlang serve` binds **127.0.0.1 only** — that is deliberate in the language
(SPEC §5.5), and here it is the entire security model. The grid and the loom
it opens cannot be reached from outside the container at all. `serve.py` is
the only door: it proxies chat, answers `/teach` behind a token, and returns
404 for anything under `/.loom`.

An accepted patch is written back to `/data/chat.ml`. **That volume is the
bot's memory.** Without it a redeploy resets it to the seed program; with it,
a restart boots from what the bot has become, and the learned program is the
new `v0`.

## Deploying with Coolify

Coolify *is* Docker — it orchestrates Docker Compose under the hood — so a
Dockerfile is the normal path, not something to avoid.

1. **New Resource → your Git repository**, branch as you like.
2. **Build Pack: Dockerfile**, path `chat/Dockerfile`, **build context the
   repository root** (the build welds `std/*.ml` into the binary with
   `include_str!`, so it needs `compiler/` *and* `std/`).
3. **Port** `8080`.
4. **Persistent volume** → mount at `/data`. Skip this and the bot forgets
   everything on every deploy.
5. **Environment** → `TEACH_TOKEN` = a long random string. Without it the bot
   serves normally and refuses all teaching, which is a safe default.

`chat/docker-compose.yml` is there if you prefer Coolify's Compose build pack;
it declares the same volume and port.

The image is two stages: `rust:1-slim` builds the toolchain, and the runtime
carries a single ~4 MB self-contained binary plus `serve.py`. The standard
library is welded in, so no MLang source ships except the bot's own program.

## Teaching it

```sh
curl -XPOST -H "X-Teach-Token: $TOKEN" https://your.host/teach \
     -d '{"pattern":"office hours","reply":"Tuesdays, 14:00–16:00."}'

curl -H "X-Teach-Token: $TOKEN" https://your.host/versions   # what it has learned
```

A pattern is matched as a substring of the lowercased message, so `coolify`
answers "What is Coolify then?". Patterns are tried in order and the newest
is tried first, so a later rule can shadow an earlier one.

The loom weaves the whole program before accepting anything, so a patch that
would not compile is refused with 422 and **the running grid is untouched** —
there is no state in which a bad rule has half-landed.

## Where a model would go, and why there isn't one yet

A model is not needed to *run* this and never will be. Where one would help is
writing the reply, not serving it: `/teach` would become "here is what someone
asked that I could not answer — propose a rule", and the proposal would go
through the identical loom patch and the identical gate. That is the slow
path, called once per lesson rather than once per message, and it is the only
place an API key would ever appear.

Deliberately not built yet, for a reason this repository has already measured.
`mos/README.md` §M3–M4: self-modification without a fitness signal **drifts** —
ungated, it accumulated structure that cost and did nothing, and lost score.
Right now the fitness signal is a human deciding a rule is worth having, which
is cheap, exact, and already wired up. Adding a model that proposes rules with
nobody judging them would reproduce the ungated arm.

## What this is not

* **Not autonomous.** It learns what it is taught. That is a feature until
  there is something better than a person to say whether a reply was good.
* **Anything that can teach can poison.** The token is the whole defence.
  Treat it like a password and do not put it in a browser.
* **Substring matching is dumb**, on purpose. It is legible, it is fast, and
  every rule it holds can be read in the source. Ranking, stemming or
  embeddings can be added later — as strands, which is the interesting way.
* **One scheduler.** The loom requires the deterministic scheduler, so
  `--parallel` is out. For a personal bot this costs nothing.
