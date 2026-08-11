※ MatrixPad — Notepad in the Matrix: a full-screen text editor.
※
※ This is a real editor, not a command shell: the document fills the
※ screen, you type to insert, and the cursor keys move you around.
※   ↑ ↓ ← → ⇱(Home) ⇲(End) ⇞(PgUp) ⇟(PgDn)   navigate
※   Enter / Backspace / Delete                edit
※   ^S save (asks for a name)   ^O open      ^Z undo   ^Y redo
※   ^F find (wraps around)      ^X exit (warns once if unsaved)
※ Open a file by argument — `mlang run examples/editor.ml note.txt`,
※ or weld it (`mlang build examples/editor.ml -o matrixpad.exe`) and
※ drop a .txt onto the executable.
※
※ Three strands, a real editor's event loop:
※   strand 0  keyboard — ⌦ reads raw keys           → k
※   strand 1  the editor core: document + dispatch  → o
※   strand 2  screen — ⊸ prints ANSI frames from o
※ ⌦ and ⌨ have the lowest scheduling priority (SPEC §4.2), so every
※ frame reaches the screen before the program waits on the next key —
※ the pipeline is interactive and still fully deterministic: the same
※ key bytes always produce the same screens, which is how the recorded
※ conformance golden drives this editor.
※
※ The document is an immutable list of lines in b; every edit is
※ slice-and-concat, so undo/redo (h and z) is a list of old
※ ⟨buffer cursor⟩ snapshots — grouped per line, like a real editor.
※ Dispatch runs inside ⍥: a glitch becomes a status-bar message, and
※ the document survives by construction.
27⍘≔⎋                                     ※ the escape character
[⎋«[»⧺⇅⧺]≔E                               ※ «7m»E → ␛[7m
[⇒j⇒i«»j[i⧺]⍣]≔J                          ※ repeat:  c n J → ccc…
[⇒u∂# u⇅- 0⊔« »⇅J⧺]≔K                     ※ pad:  s u K → s␣␣… (width u)
[⇒i∂⍕« »⧺i⧺⇅1=[][«s»⧺]?]≔Z                ※ 3«line»Z → «3 lines»
[⇒j⇒i ⎋«[»⧺i⍕⧺«;»⧺j⍕⧺«H»⧺]≔G              ※ i j G → ␛[i;jH (goto)
[∂#[∂⌷«⏎»=[∂#1- 0⇅⊂][]?][]?]≔T            ※ trim one trailing ⏎
[h⟨⟨b y x⟩⟩⧺⇒h ⟨⟩⇒z]≔N                    ※ snapshot ⟨buffer cursor⟩
[h#[z⟨⟨b y x⟩⟩⧺⇒z h⌷∂0@⇒b ∂1@⇒y 2@⇒x h 0 h#1-⊂⇒h 1⇒d «undid»⇒m][«nothing to undo»⇒m]?]≔U
[z#[h⟨⟨b y x⟩⟩⧺⇒h z⌷∂0@⇒b ∂1@⇒y 2@⇒x z 0 z#1-⊂⇒z 1⇒d «redid»⇒m][«nothing to redo»⇒m]?]≔Y
[[n⍇T∂«»=[⌫⟨«»⟩][«⏎»⊆]?⇒b «opened »n⧺« · »⧺b#«line»Z⧺⇒m][⌫⟨«»⟩⇒b«new file »n⧺⇒m]⍥
0⇒y 0⇒x 0⇒v 0⇒d ⟨⟩⇒h ⟨⟩⇒z ¯1⇒e]≔O         ※ open file n
[b«⏎»⊇«⏎»⧺ n⍈ 0⇒d «saved »n⧺« · »⧺b#«line»Z⧺⇒m]≔V
[⇒t«»⇒u 1⇒f [f][r 1 G«7m»E⧺t⧺u⧺«K»E⧺«0m»E⧺↥o ↧k⇒a
a∅=[∅⇒u 0⇒f 0⇒g][a«⏎»=[0⇒f][a«⎋»=[∅⇒u 0⇒f][a«⌫»=[u«»≠[u∂#1- 0⇅⊂⇒u][]?][a#1=[u a⧺⇒u][]?]?]?]?]?]⟳ u]≔A
[n«»=[«save as: »A ∂∅=[⌫«save cancelled»⇒m][∂«»=[⌫«save cancelled»⇒m][⇒n S]?]?][[V][⍕⇒m]⍥]?]≔S
[«open: »A ∂∅=[⌫][∂«»=[⌫][⇒n O]?]?]≔Q     ※ ^O: prompt and open
[«find: »A ∂∅=[⌫][∂«»=[⌫][⇒t ¯1⇒p b#⍸[⇒i y 1+i+b#%⇒j p ¯1=[b j@ t⍷ ¯1≠[j⇒p][]?][]?]∀
p ¯1≠[p⇒y b y@ t⍷ 0⊔⇒x «found: »t⧺⇒m][«not found: »t⧺⇒m]?]?]?]≔X
[y e≠[N y⇒e][]? b y@⇒t t 0 x⊂L⧺ t x t#⊂⧺⇒t b 0 y⊂⟨t⟩⧺ b y 1+ b#⊂⧺⇒b x 1+⇒x 1⇒d]≔I
[N b y@⇒t b 0 y⊂⟨t 0 x⊂⟩⧺⟨t x t#⊂⟩⧺ b y 1+ b#⊂⧺⇒b y 1+⇒y 0⇒x 1⇒d ¯1⇒e]≔R
[x 0>[y e≠[N y⇒e][]? b y@⇒t t 0 x 1-⊂ t x t#⊂⧺⇒t b 0 y⊂⟨t⟩⧺ b y 1+ b#⊂⧺⇒b x 1-⇒x 1⇒d]
[y 0>[N b y 1-@⇒t t#⇒x t b y@⧺⇒t b 0 y 1-⊂⟨t⟩⧺ b y 1+ b#⊂⧺⇒b y 1-⇒y 1⇒d ¯1⇒e][]?]?]≔B
[b y@⇒t x t#<[y e≠[N y⇒e][]? t 0 x⊂ t x 1+ t#⊂⧺⇒t b 0 y⊂⟨t⟩⧺ b y 1+ b#⊂⧺⇒b 1⇒d]
[y b#1-<[N t b y 1+@⧺⇒t b 0 y⊂⟨t⟩⧺ b y 2+ b#⊂⧺⇒b 1⇒d ¯1⇒e][]?]?]≔D
[y v<[y⇒v][]? y v r 3-+>[y r 3--⇒v][]? x b y@#⊓⇒x
« MatrixPad — »n«»=[«(new)»][n]?⧺d[« ×»][«»]?⧺⇒t
«H»E«7m»E⧺t c K⧺«0m»E⧺
r 2-⍸[⇒q q 2+ 1 G⧺ v q+b#<[b v q+@][«»]?⧺«K»E⧺]∀
m«»=[«^S save  ^O open  ^Z undo  ^Y redo  ^F find  ^X exit»][m]?« · Ln »⧺y 1+⍕⧺« Col »⧺x 1+⍕⧺⇒t
r 1 G⧺«7m»E⧺t c K⧺«0m»E⧺
y v-2+ x 1+G⧺↥o «»⇒m]≔F                   ※ draw one frame → o
[L«↑»=[y 0>[y 1-⇒y][]?¯1⇒e][L«↓»=[y b#1-<[y 1+⇒y][]?¯1⇒e][L«←»=[x 0>[x 1-⇒x][y 0>[y 1-⇒y b y@#⇒x][]?]?¯1⇒e][L«→»=[x b y@#<[x 1+⇒x][y b#1-<[y 1+⇒y 0⇒x][]?]?¯1⇒e][L«⇱»=[0⇒x][L«⇲»=[b y@#⇒x][L«⇞»=[y r 3-- 0⊔⇒y¯1⇒e][L«⇟»=[y r 3-+ b#1-⊓⇒y¯1⇒e][L«⏎»=[R][L«⌫»=[B][L«⌦»=[D][L«^S»=[S][L«^O»=[Q][L«^Z»=[U][L«^Y»=[Y][L«^F»=[X][L«^X»=[d w¬∧[1⇒w«unsaved — ^X again discards, ^S saves»⇒m][0⇒g]?][L#1=[I][]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]≔C
⇊
[⌦∂∅≠][↥k]⟳⌫∅↥k
⍜∂0@⇒r 1@⇒c ⟨«»⟩⇒b «»⇒n 0⇒y 0⇒x 0⇒v 0⇒d ⟨⟩⇒h ⟨⟩⇒z ¯1⇒e 0⇒w «»⇒m 1⇒g «2J»E↥o ⌂#[⌂⊃⇒n O][]? [g][F ↧k⇒L L∅=[0⇒g][L«^X»≠[0⇒w][]?[C][⍕⇒m]⍥]?]⟳ «2J»E«H»E⧺↥o ∅↥o
[↧o∂∅≠][⊸]⟳⌫
