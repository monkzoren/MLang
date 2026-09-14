# Teaching a skill, not a fact

`POST /teach` adds a row to the rule table. That is a **fact**: it answers
exactly one input, and nothing else. A thousand of them leave the bot exactly
as capable as it was.

A **skill** is a definition that *computes*. It answers an open set of inputs,
including ones that did not exist when it was taught. You cannot add one
through `/teach`, because `/teach` only writes data — you patch the program.

This is the whole reason the medium is a program rather than a database. You
can add a fact to a key-value store. You cannot add *the ability to read a
file* to one.

## The patch

Two pure definitions, so the loom rebinds them hot: no strand moves, nothing
restarts, and it works under `--parallel` and `mlang hub` too.

```
[« »⊆⇒f f#2= f0@«read»= ∧[f1@⇒q[q⍇][⌫«I cannot read »q⧺]⍥][∅]?]≔S
[∂S⇒k k∅≠[⌫k][⇩⇒s R[⇒p s p0@∈]⌿⇒m m#0>[m0@1@][D]?]?]≔A
```

`S` is the skill: split the message, and if it is `read <path>`, actually
read it — `⍥` catches an unreadable path so a bad request cannot kill the
strand. `A` is the dispatcher, rewritten to try the skill before the lookup
table and fall back to it.

## Doing it

```sh
mlang pull 8088 > patched.ml        # …apply the two lines above…
mlang patch patched.ml
⟡ v1: 1 definition rebound (A), 1 definition added (S)
```

Before the patch, `read /tmp/note.txt` got the fallback. After it, with no
restart:

```
read /tmp/note.txt          → hello from disk
read /tmp/made-just-now     → 1789405062          ← never mentioned in the lesson
read /nope/missing          → I cannot read /nope/missing
hello                       → Hello. I am a grid. …   ← everything it knew, intact
```

The third line is the test. A fact taught as `zebra → Striped.` does not
answer `quagga`. The skill reads files nobody has ever mentioned to it,
including ones created after it was taught.

## Why this is not in `chat.ml`

**A bot that reads arbitrary paths, on a public URL, hands out your
filesystem.** This is an example of the shape of a skill, not something to
deploy. A real one takes a whitelist — a fixed directory, a fixed set of
names — and the whitelist belongs in the patch, not in a request.

The same caution scales: `⍆` would give it the ability to fetch URLs, `⍈` to
write files. Each is one small definition, and each is a capability you are
granting to whoever can reach the port.
