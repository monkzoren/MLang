※ MatrixPad — Notepad in the Matrix: a full text editor, one welded binary.
※
※ Build it and it behaves like a real editor:
※   mlang build examples/editor.ml -o matrixpad.exe
※ makes a standalone executable, and dropping a .txt file onto it opens
※ that file — Windows passes the path as an argument, ⌂ reads the
※ arguments, ⍇ loads the file, and `w` writes it back with ⍈.
※
※ Three strands, wired like a real editor's event loop:
※   strand 0  keyboard — reads stdin lines           → k
※   strand 1  the editor core: document + commands   → o
※   strand 2  screen — prints whatever arrives on o
※
※ The document is an immutable list of lines in strand-local b; every
※ edit builds a new buffer from slices (⊂) and concat (⧺). That makes
※ undo/redo trivial: h and z are lists of whole old buffers — undo is
※ literally «keep the previous value». Command dispatch runs inside ⍥,
※ so a bad command («d oops», «z») answers «? …» and the session goes
※ on: the command crashes, never the editor, and the document cannot be
※ half-edited, because every edit is one immutable-value swap.
※
※ Commands (line numbers 1-based; a and i enter input mode, «.» ends it):
※   o file       open file (a missing file starts a new one)
※   w [file]     save — write the document with ⍈
※   n            new empty document
※   a / i N      append at end / insert before line N (input mode)
※   d N [M]      delete line N, or lines N…M
※   r N txt      retype line N as txt
※   m N M        move line N to line M
※   s/old/new    substitute everywhere, report the count
※   f txt        list the lines containing txt
※   p            view the document: numbered frame + status bar
※   u / U        undo / redo      h  help      q  quit
※ Try:
※   printf 'a⏎WAKE UP, NEO...⏎.⏎p⏎w note.txt⏎q⏎' | mlang run examples/editor.ml
※   mlang run examples/editor.ml note.txt    ※ open a file — drag & drop
«MatrixPad — o file:open  w file:save  n:new  a:append  i N:insert  d N M:delete  r N txt:retype  m N M:move  s/old/new:substitute  f txt:find  p:view  u:undo  U:redo  h:help  q:quit  (. ends input)»≔G
[⇒j⇒i«»j[i⧺]⍣]≔J                                         ※ repeat:  c n J → ccc…
[⇒i∂⍕« »⧺i⧺⇅1=[][«s»⧺]?]≔Z                               ※ 3«line»Z → «3 lines»
[⇒w∂# w⇅- 0⊔« »⇅J⧺]≔K                                    ※ pad:  s w K → s␣␣…
[∂#[∂⌷«⏎»=[∂#1- 0⇅⊂][]?][]?]≔T                           ※ trim one trailing ⏎
[h⟨b⟩⧺⇒h ⟨⟩⇒z]≔H                                         ※ snapshot b for undo
[h#[z⟨b⟩⧺⇒z h⌷⇒b h 0 h#1-⊂⇒h«↶ undo»↥o][«? nothing to undo»↥o]?]≔U
[z#[h⟨b⟩⧺⇒h z⌷⇒b z 0 z#1-⊂⇒z«↷ redo»↥o][«? nothing to redo»↥o]?]≔Y
[b 0 p⊂⟨L⟩⧺ b p b#⊂⧺⇒b p1+⇒p]≔I                          ※ insert line L at p
[a⍭∂⊃⍎⇒x #1>[a⍭1@⍎][x]?⇒e b 0 x1-⊂ b e b#⊂⧺⇒b]≔D        ※ delete lines x…e
[a⍭⊃⍎⇒x a« »⍷1+∂[a⇅⊥][⌫«»]?⇒y b 0 x1-⊂⟨y⟩⧺ b x b#⊂⧺⇒b]≔R ※ retype line x as y
[a⍭∂⊃⍎⇒x 1@⍎⇒e b x1-@⇒c b 0 x1-⊂ b x b#⊂⧺⇒b e1-0⊔b#⊓⇒e b 0 e⊂⟨c⟩⧺ b e b#⊂⧺⇒b]≔M
[a«/»⊆⇒x x⊃«»=[x⍫⇒x][]?x⊃⇒c x#1>[x 1@][«»]?⇒e
c«»=[«? usage: s/old/new»↥o][b 0[c⊆#1-+]⍀⇒x b[c⊆e⊇]∵⇒b x⍕« replaced»⧺↥o]?]≔S
[b#⍸[⇒x bx@a∈[x1+⍕« │ »⧺bx@⧺↥o][]?]∀]≔F                  ※ find lines holding a
[n«»=[«? no file name — try: o name.txt»↥o]
[H[n⍇∂«»=[⌫⟨⟩][T⍖]?⇒b«opened »n⧺« · »⧺b#«line»Z⧺↥o][⌫⟨⟩⇒b«new file »n⧺↥o]⍥]?]≔O
[n«»=[«? no file name — try: w name.txt»↥o]
[b#[b«⏎»⊇«⏎»⧺][«»]?n⍈«saved »n⧺« · »⧺b#«line»Z⧺↥o]?]≔W
[n«»=[«MatrixPad»][n]?⇒x b#⍕#⇒u b 0[#⊔]⍀ 12⊔ x# u-1- 0⊔ ⊔⇒v
b#«line»Z« · »⧺b 0[⍭#+]⍀«word»Z⧺« · »⧺b 0[#+]⍀«char»Z⧺⇒c c# u-3- 0⊔ v⊔⇒v u v+7+⇒e
«┌─ »x⧺« »⧺«─»e x#-5-J⧺«┐»⧺↥o
b#⍸[⇒x «│ »x1+⍕u K⧺« │ »⧺bx@v K⧺« │»⧺↥o]∀
«├»«─»e 2-J⧺«┤»⧺↥o «│ »c u v+3+K⧺« │»⧺↥o «└»«─»e 2-J⧺«┘»⧺↥o]≔P
[L 0 1⊂⇒t L 2⊥⇒a t«a»=[H b#⇒p 1⇒m][t«i»=[H a⍭⊃⍎1-0⊔b#⊓⇒p 1⇒m][t«o»=[a«»≠[a⇒n][]?O][t«w»=[a«»≠[a⇒n][]?W]
[t«n»=[H⟨⟩⇒b«»⇒n«new buffer»↥o][t«d»=[H D][t«r»=[H R][t«m»=[H M][t«s»=[H S][t«f»=[F][t«p»=[P][t«u»=[U]
[t«U»=[Y][t«h»=[G↥o][t«q»=[0⇒g][«? »L⧺↥o]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]≔C
⇊
[⌨∂∅≠][↥k]⟳⌫∅↥k
⟨⟩⇒b ⟨⟩⇒h ⟨⟩⇒z «»⇒n 0⇒m 0⇒p 1⇒g G↥o ⌂#[⌂⊃⇒n O][]?[g[↧k∂∅≠][∅ 0]?][⇒L m[L«.»=[0⇒m][I]?][[C][«? »⇅⍕⧺↥o]⍥]?]⟳⌫∅↥o
[↧o∂∅≠][⍞]⟳⌫
