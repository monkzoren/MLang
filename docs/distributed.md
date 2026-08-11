# Distributed streams — `mlang hub` and `mlang worker`

MLang strands share nothing but channels, every value is immutable, and
a stream ends with an in-band `∅`. Those three properties mean a channel
can cross a machine boundary without changing the language: `mlang hub`
and `mlang worker` bridge two designated channels over TCP, and the
programs on both ends are ordinary MLang — the worker is literally the
pump stage you would write in a single-process pipeline.

This is a proof of concept. It is honest about what it preserves and
what it trades (below), but it has no authentication and no encryption:
run it only on a network you trust.

## The model

```
  hub machine                                  worker machines (N of them)
┌─────────────────────────────┐              ┌──────────────────────────┐
│ hub program                 │   work: α    │ worker program           │
│   …⇈α        pour work      │ ═══════════▶ │   [body]⇉αβ   the pump   │
│   ⇟β…        drain results  │ ◀═══════════ │                          │
└─────────────────────────────┘   results: β └──────────────────────────┘
```

* On the **hub**, the work channel (`α` by default) is *exported*:
  values the program sends there go over the wire instead of the local
  queue. The results channel (`β`) is *imported*: values workers send
  back are injected into it, so the program receives them with plain
  `↧β` / `⇟β`.
* On a **worker**, the mirror image: work arrives on its imported `α`,
  and sends to its exported `β` return to the hub.
* `--work G` and `--results G` rename the bridged glyphs on either side.

Everything else about both programs — other channels, other strands,
globals, spawning, glitches — stays local and unchanged.

## Running the prime-finder example

On the serving machine (any reachable address; `0.0.0.0:7777` is the
default):

```
$ mlang hub --workers 2 examples/net-primes-hub.ml
⇅ hub listening on 0.0.0.0:7777 (work α → workers → results β)
⇅ waiting for 2 workers…
```

On each worker machine:

```
$ mlang worker --connect HUB-ADDRESS:7777 examples/net-primes-worker.ml
⇅ joined hub at HUB-ADDRESS:7777
```

When the last range is answered:

```
π(<50000) = 5133
largest: 49999
```

The hub takes the limit and chunk size as program arguments
(`mlang hub … net-primes-hub.ml 200000 5000`). Workers can join at any
time, including mid-run — a late joiner simply starts receiving ranges.
`--workers N` (default 1) is only the starting gate: the program begins
once N workers have joined.

## Work distribution and the ∅ convention

Distribution is demand-driven: each worker holds at most 2 unanswered
items; the next item goes to the least-loaded worker; a result frees a
slot. A worker that computes twice as fast ends up with twice the items
— no static partitioning.

The stream protocol is the language's own: the hub program ends its
pour with `∅` (`⇈` does that automatically). The hub holds that `∅`
until every dispatched item has its result, then forwards it — to every
worker, whose pumps stop, and onto its own results channel, so its
drain finishes. Because a pump is one-in-one-out in order, result k on
a connection acknowledges item k, which is what lets the hub know what
a lost worker still owed.

**Failure is the language's failure model, over a socket.** If a
worker's pump body glitches, the worker strand dies exactly as it would
locally (let-it-crash), the process exits 1, and the closed socket
tells the hub to requeue that worker's unanswered items on the
survivors. Kill a worker machine mid-run and the job still completes:

```
⇅ worker 1 lost — 2 items requeued
⇅ worker 2 finished
```

If *every* worker is gone, pending work simply waits for the next one
to join — the hub is a server.

## What is preserved, and what is traded

Preserved, and covered by `compiler/tests/net.rs`:

* **Per-sender FIFO and blocking receive.** TCP keeps each connection
  ordered; `↧`/`⇟` on an imported channel block exactly as locally.
* **Glitch isolation and crash recovery** as above.
* **Deadlock detection stays sound.** A wait on a network-fed channel
  is exempt from the deadlock verdict while the wire may still deliver
  (the remote side is not provably stuck); once the channel's `∅`
  arrives the exemption ends and the ordinary proof applies. Waits on
  purely local channels are proven exactly as before.
* **The ∅ end-of-stream convention**, end to end.

Traded, knowingly:

* **Global determinism.** Results arrive in real network order, so the
  interleaving on the results channel varies run to run — the same
  concession `mlang run --parallel` already makes, for the same reason.
  A program that reduces order-independently (a sum, a max, a count, a
  sort) still prints byte-identical output for any worker count and any
  timing; the example does exactly that. The deterministic sequential
  engine and the conformance corpus are untouched.
* **Backpressure.** `↥` never blocks, so an unbounded pour buffers at
  the hub. The per-worker credit bounds what is on the wire, not what
  the hub holds.
* **Quotations stay home.** Code is not data on the wire: a quotation
  (or an unfinished list mark) sent on a bridged channel is a fatal net
  error. Ints (arbitrary precision), floats, strings, nil, and nested
  lists all cross losslessly.

## The wire, if you want to speak it

One value per line, UTF-8, in the language's own literal syntax —
`∅`, `¯5`, `2.5`, `«text»` (newlines inside strings travel as `⏎`),
`⟨1 «a» ⟨2⟩⟩` — after a one-line hello on each side (`⇓ mlang-hub 1` /
`⇓ mlang-worker 1`). A worker is ~40 lines of any language that can
read lines from a socket; it need not be MLang at all.
