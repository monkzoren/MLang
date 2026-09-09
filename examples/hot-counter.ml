※ hot-counter — the smallest server worth patching while it runs.
※
※ One strand accepts requests and answers each with the number of
※ requests it has served so far; the count lives in the strand-local n.
※ The greeting is a definition, G. Serve it, then rewrite it live:
※
※   ./mlang serve examples/hot-counter.ml 4321
※   curl 127.0.0.1:4321/          → «hello 1»
※   ./mlang pull 4321 > c.ml       … edit … ./mlang patch c.ml
※
※ docs/loom.md walks through three patches: rebinding G (and a
※ definition computed from it), replacing this strand with one that
※ keeps a second counter — with a ⟲ migration that gives the new local
※ its first value, so n is not lost — and reviving the strand after a
※ patch that killed it.
«hello »≔G
G«!»⧺≔E
⇊
0⇒n 1⇒g[g][⎆∂∅=[⌫0⇒g][⇒r n1+⇒n ⟨r0@ 200 «text/plain» G n⍕⧺⟩⍅]?]⟳
