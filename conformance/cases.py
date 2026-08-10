"""The MLang conformance corpus.

Each case is (name, source, stdin). Expected outputs are recorded from the
reference implementation by record.py into expected.json; run.py verifies
any engine against them byte-for-byte (stdout, stderr, and exit code).
"""

CASES = [
    # ── literals & formatting ──
    ("int", "42⍞", ""),
    ("digit-runs", "1 2 3++⍞", ""),
    ("negative", "¯5⍞ ¯5 3+⍞", ""),
    ("float", "2.5⍞ .5 2×⍞ 3.0⍞", ""),
    ("bignum", "2 100^⍞ 2 100^ 2 100^×⍞", ""),
    ("string", "«Hello, Matrix»⍞ «a⏎b»⍞", ""),
    ("string-raw-print", "«x»⊸«y»⊸", ""),
    ("list", "⟨1 «two» ⟨3 4⟩ 2.5⟩⍞", ""),
    ("nil", "∅⍞ ∅∅=⍞ ∅0=⍞ ∅¬⍞", ""),
    # ── stack ──
    ("dup-swap-drop", "1∂+⍞ 1 2⇅-⍞ 1 2⌫⍞", ""),
    ("over-rot-depth", "1 2⊚⍞⍞⍞ 1 2 3⥀⍞⍞⍞ 7 8≢⍞", ""),
    ("underflow", "+", ""),
    # ── arithmetic ──
    ("arith", "3 4+⍞ 10 3-⍞ 6 7×⍞ 2 10^⍞", ""),
    ("division", "8 2÷⍞ 7 2÷⍞ ¯8 2÷⍞ 1 3÷⍞", ""),
    ("modulo", "7 3%⍞ ¯7 3%⍞ 7 ¯3%⍞ 7.5 2%⍞", ""),
    ("unary-math", "2√⍞ 2.7⌊⍞ 2.1⌈⍞ 5±⍞ ¯2.5⌊⍞", ""),
    ("div-zero", "1 0÷", ""),
    ("mod-zero", "1 0%", ""),
    ("sqrt-negative", "¯1√", ""),
    ("type-error-add", "1«x»+", ""),
    # ── comparison & logic ──
    ("compare", "1 2<⍞ 2 2≤⍞ 3 2>⍞ 1 2≥⍞ 1 1=⍞ 1 2≠⍞ 1 1.0=⍞", ""),
    ("string-compare", "«a»«b»<⍞ «b»«a»≥⍞ «x»«x»=⍞", ""),
    ("list-deep-eq", "⟨1 ⟨2 3⟩⟩⟨1 ⟨2 3⟩⟩=⍞ ⟨1⟩⟨2⟩=⍞", ""),
    ("mixed-compare-glitch", "1«a»<", ""),
    ("logic", "1 0∧⍞ 1 0∨⍞ 0¬⍞ 1 1⊻⍞ 1 0⊻⍞", ""),
    # ── control ──
    ("apply", "[1 2+]!⍞ [[3]!]!⍞", ""),
    ("if", "1[«y»][«n»]?⍞ 0[«y»][«n»]?⍞ 1 5 9?⍞", ""),
    ("while-fib", "0 1[∂1000<][∂⍞⇅⊚+]⟳⌫⌫", ""),
    ("repeat", "0 5[1+]⍣⍞ 0 0[1+]⍣⍞", ""),
    # ── iteration ──
    ("range-map", "5⍸[∂×]∵⍞ 0⍸[∂×]∵⍞", ""),
    ("each", "3⍸[⍞]∀", ""),
    ("filter", "10⍸[2%0=]⌿⍞", ""),
    ("fold", "101⍸ 0[+]⍀⍞", ""),
    ("iterate-string", "«abc»[⍞]∀ «ab»[«!»⧺]∵⍞", ""),
    # ── sequences ──
    ("seq-basics", "«abc»#⍞ ⟨1 2⟩⟨3⟩⧺⍞ «ab»«cd»⧺⍞ ⟨7 8 9⟩1@⍞ «xyz»2@⍞", ""),
    ("slice", "«matrix»1 4⊂⍞ «ab»0 99⊂⍞ ⟨1 2 3 4⟩1 3⊂⍞", ""),
    ("split-join", "«a,b,c»«,»⊆⍞ «ab»«»⊆⍞ ⟨1 2 3⟩«-»⊇⍞", ""),
    ("str-parse", "42⍕«!»⧺⍞ «¯3»⍎⍞ «2.5»⍎⍞ ⟨1 2⟩⍕#⍞", ""),
    ("codepoints", "«A»⌗⍞ 66⍘⍞ «⍳»⌗⍞", ""),
    ("index-oob", "⟨1⟩5@", ""),
    ("parse-garbage", "«nope»⍎", ""),
    # ── bindings ──
    ("define-call", "[∂×]≔² 9²⍞ 3.14≔π π⍞", ""),
    ("locals", "1⇒x x 1+⇒x x⍞", ""),
    ("redefine-glitch", "1≔x 2≔x", ""),
    ("reserved-sigil", "1≔+", ""),
    ("undefined-sigil", "Ω", ""),
    # ── glitches ──
    ("try-catch", "[1 0÷][«got: »⇅⍕⧺⍞]⍥«after»⍞", ""),
    ("try-restores-stack", "7[1 2 3 0÷][⌫]⍥⍞", ""),
    ("raise-custom", "[⟨1 2⟩↯][⍞]⍥", ""),
    ("nested-try", "[[«inner»↯][«re-»⇅⍕⧺↯]⍥][⍞]⍥", ""),
    ("try-disarms", "[1][«no»⍞]⍥⍞", ""),
    ("uncaught-kills-strand", "1 0÷\n«survivor»⍞", ""),
    # ── load errors ──
    ("lone-negative", "¯⍞", ""),
    ("unterminated-string", "«abc", ""),
    ("unclosed-quotation", "[1 2", ""),
    ("stray-dot", "1 .⍞", ""),
    ("double-dot", "1.2.3⍞", ""),
    ("unmatched-close", "1]⍞", ""),
    ("tabs-rejected", "1\t2+⍞", ""),
    ("loose-rain-marker", "⇓«Hello, Matrix»⍞", ""),
    ("loose-divider", "1⇊2", ""),
    ("loose-continuation", "1 ⋮⍞", ""),
    ("loose-newline-glyph", "⏎⍞", ""),
    # ── strands & channels ──
    ("pipeline", "9⍸[1+∂×↥α]∀∅↥α\n[↧α∂∅≠][2×↥β]⟳⌫∅↥β\n[↧β∂∅≠][⍞]⟳⌫", ""),
    ("fifo", "1↥c 2↥c 3↥c\n↧c⍞↧c⍞↧c⍞", ""),
    ("try-recv", "⇂q⍞ 5↥q ⇂q⍞⍞", ""),
    ("strand-id-count", "⍳↥a\n⍳↥b\n↧a⍞↧b⍞≣⍞", ""),
    ("spawn-join", "[42↥r]⚡⋈↧r⍞", ""),
    ("spawn-locals-copy", "7⇒x [x↥r]⚡⋈ 9⇒x ↧r⍞ x⍞", ""),
    ("join-unknown", "99⋈", ""),
    ("deadlock", "↧a\n↧b", ""),
    ("interleave-deterministic", "5⍸[«A»⍞]∀\n5⍸[«B»⍞]∀", ""),
    ("yield", "3⍸[«A»⍞⌛]∀\n3⍸[«B»⍞⌛]∀", ""),
    # ── stream combinators ──
    ("pour-drain", "⟨1 2 3⟩⇈a ⇟a⍞ ⟨⟩⇈b ⇟b⍞ «xy»⇈c ⇟c⍞", ""),
    ("pump-pipeline", "9⍸[1+∂×]∵⇈α\n[2×]⇉αβ\n⇟β[⍞]∀", ""),
    ("pump-empty", "⟨⟩⇈a\n[∂×]⇉ab\n⇟b#⍞", ""),
    ("pump-chained", "5⍸⇈a\n[1+]⇉ab\n[10×]⇉bc\n⇟c⍞", ""),
    ("drain-cross-strand", "3⍸⇈q\n⇟q⍞", ""),
    ("pump-type-error", "5⇉ab", ""),
    ("pour-type-error", "5⇈a", ""),
    # ── boot & forms ──
    ("boot-defs", "[∂×]≔²\n⇊\n7²↥a\n↧a 1+⍞", ""),
    ("boot-glitch-stops", "1 0÷\n⇊\n«never»⍞", ""),
    ("continuation", "1 2\n⋮+⍞", ""),
    ("comment-lines", "※ a comment\n⍳⍞ ※ trailing\n", ""),
    ("rain-hello", "⇓\n«\nH\ni\n»\n⍞", ""),
    ("rain-two-strands", "⇓\n1  «\n⍞  A\n   »\n   ⍞", ""),
    ("rain-comment-column", "⇓\n1\n⍞\n※\n9", ""),
    ("rain-boot", "⇓\n[\n∂\n×\n]\n≔\n²\n⇊\n6\n²\n⍞", ""),
    # ── i/o ──
    ("stdin-echo", "[⌨∂∅≠][⍞]⟳⌫", "x\ny\n"),
    ("stdin-eof", "⌨⍞⌨⍞⌨⍞", "a\nb\n"),
    ("debug-stderr", "1«s»⟨2 3⟩⍟", ""),
    # ── misc semantics ──
    ("quot-identity-eq", "[1]∂=⍞ [1][1]=⍞", ""),
    ("list-build-nested", "⟨1⟨2⟨3⟩⟩⟩#⍞", ""),
    ("unmatched-list-close", "1⟩⍞", ""),
    ("empty-things-falsy", "«»[«t»][«f»]?⍞ ⟨⟩[«t»][«f»]?⍞ 0.0[«t»][«f»]?⍞", ""),
]

# Example programs are also conformance cases, verified as files.
EXAMPLE_FILES = [
    "hello.ml",
    "hello-rain.ml",
    "fibonacci.ml",
    "fizzbuzz.ml",
    "pipeline.ml",
    "pipeline-manual.ml",
    "parallel-sum.ml",
    "spawn.ml",
    "glitch.ml",
]

EXAMPLE_STDIN = {}
