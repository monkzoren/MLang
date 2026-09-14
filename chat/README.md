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

## Getting a URL

Coolify *is* Docker — it orchestrates Docker Compose under the hood — so a
Dockerfile is the normal path, not something to avoid. It also does the part
you actually want here: it runs a reverse proxy in front of your containers,
assigns the domain, and gets a Let's Encrypt certificate for it. You do not
set up TLS yourself and you do not publish a port on the host.

1. **New Resource → your Git repository**, branch as you like.
2. **Build Pack: Dockerfile**, file `chat/Dockerfile`, and set the **build
   context to the repository root** — the build welds `std/*.ml` into the
   binary with `include_str!`, so it needs `compiler/` *and* `std/`.
3. **Port** `8080`. This is the port Coolify's proxy forwards *to*, inside
   the container. Nothing is published on the host.
4. **Domain**: give the resource a domain (a subdomain you point at the
   server, or the free wildcard host Coolify offers). Point its DNS `A`
   record at your server first, or the certificate cannot be issued.
5. **Persistent volume** → mount at `/data`. Skip this and the bot forgets
   everything it was taught on every redeploy.
6. **Environment** → `TEACH_TOKEN` = a long random string.

Deploy, and `https://your.domain` is the bot, reachable from anywhere. The
page is responsive and works on a phone, which is the point — teaching it
from a laptop and talking to it from a train should both be one tap.

`chat/docker-compose.yml` is there if you prefer Coolify's Compose build
pack. It deliberately has no `ports:` mapping, for the reason above.

Coolify renames UI fields between versions; if one of these is not where this
says, its own documentation is authoritative.

### The URL is public

Anyone who has it can chat. That is usually fine — the bot only says what it
has been taught — but be clear that it is true:

* **Chatting is open. Teaching is not**, and the token is the whole defence.
  Use a long random one, and rotate it by changing the environment variable
  and redeploying.
* **The token lives in the browser** once you type it into the teach form
  (`localStorage`), which is the trade that makes teaching from a phone
  practical. Use a private device, or teach with `curl` instead.
* **There is no rate limiting.** If the URL leaks somewhere noisy, put
  Coolify's proxy authentication or a Cloudflare Access policy in front — the
  bot does not need to know about either.
* `/.loom` is never routed from outside, whatever the domain. The grid binds
  127.0.0.1 and the front process returns 404 for it.

## Teaching it

Open the page and use the **teach me something** form at the bottom — it
remembers the token, so it is one tap from a phone thereafter. Or from a
shell:

```sh
curl -XPOST -H "X-Teach-Token: $TOKEN" https://your.domain/teach \
     -d '{"pattern":"office hours","reply":"Tuesdays, 14:00-16:00."}'

curl -H "X-Teach-Token: $TOKEN" https://your.domain/versions  # what it has learned
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
