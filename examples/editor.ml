※ MatrixPad — Notepad in the Matrix: an ed-style line editor.
※
※ Three strands, wired like a real editor's event loop:
※   strand 0  keyboard — reads stdin lines           → k
※   strand 1  the editor core: document + commands   → o
※   strand 2  screen — prints whatever arrives on o
※
※ The document is an immutable list of lines in strand-local b. Every
※ edit builds a new buffer from slices (⊂) and concat (⧺) — nothing is
※ ever mutated, so a stray command cannot corrupt the document, and
※ undo would just mean keeping an old value of b.
※
※ Commands (line numbers are 1-based; a and i enter input mode, where
※ every line goes into the document until «.» on its own line):
※   a          append lines at the end
※   i N        insert lines before line N
※   d N        delete line N
※   r N txt    replace line N with txt
※   f txt      list the lines containing txt
※   p          list the document, numbered
※   w          write the document to the screen (the «file»)
※   q          quit
※ Dispatch runs inside ⍥, so a bad command («d oops», «z») answers
※ «? …» and the session continues — the command crashes, never the
※ editor. Try:
※   printf 'a⏎WAKE UP, NEO...⏎.⏎p⏎q⏎' | mlang run examples/editor.ml
[b#⍸[⇒x x1+⍕« │ »⧺bx@⧺↥o]∀]≔V                            ※ view: numbered listing
[b[↥o]∀]≔W                                               ※ write: the raw document
[b 0 p⊂⟨L⟩⧺ b p b#⊂⧺⇒b p1+⇒p]≔I                          ※ insert line L at point p
[⇒x b 0 x1-⊂ b x b#⊂⧺⇒b]≔D                               ※ delete line x
[a⍭⊃⍎⇒x a« »⍷1+∂[a⇅⊥][⌫«»]?⇒y b 0 x1-⊂⟨y⟩⧺ b x b#⊂⧺⇒b]≔R ※ replace line x with y
[b#⍸[⇒x bx@a∈[x1+⍕« │ »⧺bx@⧺↥o][]?]∀]≔F                  ※ find: lines containing a
[L 0 1⊂⇒t L 2⊥⇒a t«a»=[b#⇒p 1⇒m][t«i»=[a⍭⊃⍎1-0⊔b#⊓⇒p 1⇒m][t«d»=[a⍭⊃⍎D][t«r»=[R][t«f»=[F][t«p»=[V][t«w»=[W][t«q»=[0⇒g][«? »L⧺↥o]?]?]?]?]?]?]?]?]≔C
⇊
[⌨∂∅≠][↥k]⟳⌫∅↥k
⟨⟩⇒b 0⇒m 0⇒p 1⇒g «MatrixPad — a:append i N:insert d N:delete r N txt:replace f txt:find p:list w:write q:quit (. ends input)»↥o [g[↧k∂∅≠][∅ 0]?][⇒L m[L«.»=[0⇒m][I]?][[C][«? »⇅⍕⧺↥o]⍥]?]⟳⌫∅↥o
[↧o∂∅≠][⍞]⟳⌫
