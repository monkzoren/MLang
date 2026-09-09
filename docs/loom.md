# The loom — patching a grid that never stops

A served MLang program is a grid that keeps running. The loom lets any
number of agents rewrite it in place: pull the live source, edit it,
weave it back. This walkthrough patches
[`examples/hot-counter.ml`](../examples/hot-counter.ml) four times
without restarting it. Every output below is what the toolchain
printed; the same session, replayed from stdin frames, is pinned
byte-for-byte as the `example:hot-counter.ml` conformance golden.

The program is one strand and two definitions:

```
«hello »≔G
G«!»⧺≔E
⇊
0⇒n 1⇒g[g][⎆∂∅=[⌫0⇒g][⇒r n1+⇒n ⟨r0@ 200 «text/plain» G n⍕⧺⟩⍅]?]⟳
```

The strand-local `n` counts requests; every answer is the greeting `G`
followed by the count.

```sh
$ ./mlang serve examples/hot-counter.ml 4321
⇓ the grid is listening on http://127.0.0.1:4321
⟡ the loom is open at http://127.0.0.1:4321/.loom
$ curl 127.0.0.1:4321/ ; curl 127.0.0.1:4321/
hello 1
hello 2
```

## 1. Rebind a definition — and the one computed from it

`mlang pull` prints the live source with a stamp on its first line
naming the version and the server. The stamp is a comment, so the file
is still a program.

```sh
$ ./mlang pull 4321 > c.ml
⟡ pulled v0 from http://127.0.0.1:4321/.loom
$ head -1 c.ml
※ loom v0 http://127.0.0.1:4321
$ sed -i 's|«hello »≔G|«served »≔G|' c.ml
$ ./mlang patch c.ml
⟡ v1: 2 definitions rebound (G E)
$ curl 127.0.0.1:4321/
served 3
```

`E` was never edited. It is defined as `G«!»⧺` — a pure expression of
`G` — so when `G` changed, the loom recomputed `E` and rebound it too.
Any definition whose expression is pure is hot; a boot line with an
effect (a file read, a print) is refused if a patch changes it, because
it ran once at start and cannot honestly run again.

The count went on from 2 to 3: the strand was not touched.

## 2. Replace the strand, with a `⟲` migration

Now the strand itself changes: a second counter `m`. The new code
references a local the old strand never had, so the patch carries a
**migration** — a `⟲` line just above the strand — that runs once at
the strand's seam, on its old stack and locals, before the new code
takes over. Here it seeds `m` from `n`:

```sh
$ ./mlang pull 4321 > c.ml
⟡ pulled v1 from http://127.0.0.1:4321/.loom
```

Edit the strand line into these two lines:

```
⟲ n 1000×⇒m
0⇒n 0⇒m 1⇒g[g][⎆∂∅=[⌫0⇒g][⇒r n1+⇒n m1+⇒m ⟨r0@ 200 «text/plain» G n⍕⧺« of »⧺m⍕⧺⟩⍅]?]⟳
```

```sh
$ ./mlang patch c.ml
⟡ v2: 1 strand replaced
  strand 0 continues as row 20 at its next seam, after its ⟲ migration
$ curl 127.0.0.1:4321/ ; curl 127.0.0.1:4321/
served 4 of 3001
served 5 of 3002
```

Three things happened at the seam. The migration ran (`m` is 3000, from
`n` = 3). The prelude of the new code — `0⇒n 0⇒m 1⇒g` — did **not** run:
a replaced strand resumes inside its new outermost loop with stack and
locals intact, which is why `n` continued from 3 instead of resetting.
And the seam was immediate: a server parked at `⎆` waiting for its next
request is at a seam.

The loom stores the migration as a comment, so the history shows it and
line numbers stay put:

```sh
$ curl -s 127.0.0.1:4321/.loom/v2 | grep '^※ ⟲'
※ ⟲ n 1000×⇒m
```

## 3. Ship a bug — the grid holds its port

```sh
$ ./mlang pull 4321 > c.ml
$ sed -i 's|m1+⇒m|m1 0÷+⇒m|' c.ml       # a division by zero on every request
$ ./mlang patch c.ml
⟡ v3: 1 strand replaced
  strand 0 continues as row 20 at its next seam
$ curl -i 127.0.0.1:4321/
HTTP/1.1 503 Service Unavailable
…
the grid has stopped (dead: strand 0 (row 20)) — mend it: mlang pull / mlang patch
```

The server's log has the fault, with the version its code came from and
that version's line excerpted:

```
✗ glitch in strand 0 (row 20) at v3 20:40: ÷ by zero
  v3 20│ …⇒g[g][⎆∂∅=[⌫0⇒g][⇒r n1+⇒n m1 0÷+⇒m ⟨r0@ 200 «text/plain» G n⍕…
                                        ↑ v3 20:40
  stack: 3002
⟡ the grid has stopped (dead: strand 0 (row 20)) — holding the port for a patch
```

The only strand is dead, and an ordinary run would exit 1 here. A served
grid with its loom open does not: it keeps the port, answers 503 with
the reason, and waits to be mended.

## 4. Mend it — the strand comes back with its state

```sh
$ ./mlang pull 4321 > c.ml
⟡ pulled v3 from http://127.0.0.1:4321/.loom
$ sed -i 's|m1 0÷+⇒m|m1+⇒m|' c.ml
$ ./mlang patch c.ml
⟡ v4: 1 strand replaced
  strand 0 continues as row 20 at its next seam
$ curl 127.0.0.1:4321/ ; curl 127.0.0.1:4321/
served 7 of 3003
served 8 of 3004
```

A dead strand is a seam. It revived on the new code with an empty stack
but its locals intact: `n` was 6 (the fatal request had already counted
itself) and `m` 3002, and the count went on. Nothing was restarted;
nothing was lost.

```sh
$ ./mlang loom 4321
v0  as started
v1  2 definitions rebound (G E)
v2  1 strand replaced
v3  1 strand replaced
v4  1 strand replaced
```

## N agents

Each patch carries the version it was written against. If the live
version has moved on, the loom three-way merges the patch onto it, line
by line — a line is a strand or a definition, so agents editing
different lines (even adjacent ones) never block each other. Only a line
two agents changed differently conflicts:

```
⟡ 409
✗ patch conflicts with v2 (written against v0) — pull v2 and reapply your change:
  v2 lines 1-1:
    │ «served »≔G
  yours:
    │ «hi »≔G
```

The grid is untouched by a conflicting patch; the agent pulls and tries
again. Every accepted version stays available at `/.loom/vN`.

## Replaying it

Patches travel in the request stream, so the whole session above is a
deterministic function of its input. In replay mode a patch is a frame
on stdin — `⟡ base nbytes` followed by the source — and the runtime
writes `⟡ status nbytes` and its report to stdout. That is how the
conformance corpus pins this walkthrough. SPEC §4.7 has the rules.
