※ THE ARCHITECT — a live multiplayer spreadsheet, served by MLang.
※
※ The killer application was always the spreadsheet; MLang's motto was
※ always «programs are grids». This program is both: a full web
※ application in one MLang file. Serve it and open the page — in as
※ many browser windows as you like, because the sheet is shared:
※
※   mlang serve examples/architect.ml 4321        # → http://127.0.0.1:4321
※   mlang serve examples/architect.ml 4321 my.tsv # open/save a real TSV
※   mlang build examples/architect.ml -o architect && MLANG_PORT=4321 ./architect
※
※ Six strands run the site (flat form: one line per machine):
※   0 acceptor    ⎆ pulls each HTTP request, deals it onto κ
※   1 the engine  owns the sheet; parses, recalculates, routes, answers on β
※   2 responder   ⍅ writes every response back to the browser
※   3-5 fetchers  a work-stealing pool: ↧φ url → ⍆ fetch → ⒥ parse → ↥ρ
※
※ Multiplayer is the ⎆/⍅ design paying off: responses are addressed by
※ request id, so the engine simply *doesn't answer* a poll until the
※ sheet changes — a held GET /api/poll?v=N is released by whichever
※ client edits next, and every open window converges. Undo, redo,
※ fills, styles, refreshes: all shared, all in version order, and the
※ whole conversation replays byte-for-byte from a recorded session.
※
※ The engine wraps each request in ⍥, so a broken formula or a bad
※ body becomes a 500 answer and a ✗ status — the server cannot die.
※ Shutdown is a cascade of ∅ poison pills (held polls answered 204
※ first); nothing hangs, nothing is left blocked.
※
※ Formulas (typed into any cell after =):
※   arithmetic  + - * / ^ %  ·  ( )  ·  unary minus  ·  & concatenates
※   compare     = <> < <= > >=      ·  "quoted" or bare strings
※   refs        B3 · ranges B3:D5   ·  SUM AVG MIN MAX CNT over ranges
※   logic       IF(cond,then,else)  ·  ABS ROUND SQRT LEN
※   text        UPPER LOWER LEFT RIGHT MID
※   time        NOW() TODAY() TIME()      — the ⌚ clock, pinnable
※   charts      SPARK(B3:B10)             — ▁▂▃▅▇ block sparklines
※   live data   FX(EUR,USD) exchange rates · BTC(USD) bitcoin price
※               WX(lat,lon) temperature °C · GET("url","path.to.field")
※ Live cells show … until ↻ refresh fans their urls out to the fetcher
※ pool (run with --parallel and the fetches overlap for real); every
※ response is JSON read by the Operator (std/json.ml). A formula error
※ shows ✗ in that cell alone; circular references are detected by the
※ recalculation fixpoint and marked ⚠ — the sheet never hangs. Fill
※ and paste shift cell references by their offset (Ξ), the classic
※ relative-reference rewrite; a reference pushed off the grid becomes
※ #REF and reports «shifted off the grid».
※
※ The sheet arrives and leaves as honest TSV (⍇/⍈), so it pastes
※ straight into and out of any other spreadsheet — and a block copied
※ from one lands here through POST /api/batch.
※
※ Engine-strand register file (strand-locals; Greek carries the new
※ machinery so the Latin letters keep their old jobs):
※   persistent   b raw grid · v values · k live cache · n file name
※                w wanted urls · s status · g run flag · η styles
※                θ undo · λ redo · μ version · ξ held poll ids
※   tokenizer T  t i a c j      parser P  o q z y      eval E  z m
※   range G      f h u x        live H  l              set-cell S  z m y
※   recalc C     p d e h        edits A/Π  i j         router U  r ε δ
※   shift Ξ      t i a c j ε δ ζ ν       fill ∇  σ ψ ω ς ϰ ε ζ
※   dates Θ      ε ζ ι ο υ ν            style ¤  ι ο
30≔N
16≔M
«ABCDEFGHIJKLMNOP»≔Y
[Y⇅@⇅1+⍕⧺]≔Q
[⍕∂#1=[«0»⇅⧺][]?]≔②
※ Θ: Unix milliseconds → ⟨year month day⟩ (Hinnant's civil-from-days)
[86400000÷⌊719468+⇒ζ ζ146097÷⌊⇒ε ζ ε146097×-⇒ι
ι ι1460÷⌊-ι36524÷⌊+ι146096÷⌊-365÷⌊⇒ο ι365ο×ο4÷⌊+ο100÷⌊--⇒υ
5υ×2+153÷⌊⇒ν υ153ν×2+5÷⌊-1+ν10<[ν3+][ν9-]?ο ε400×+⊚2≤[1+][]?
⟨⇅⟩⇅⟨⇅⟩⧺⇅⟨⇅⟩⧺]≔Θ
[⇒z⇒m⇒y∂y@∂0 m⊂⟨z⟩⧺⇅∂#m 1+⇅⊂⧺⇅∂0 y⊂⥀⟨⇅⟩⧺⇅∂#y 1+⇅⊂⧺]≔S
[∂«»=[][∂0 1⊂«=»=[⌫⟨«⌛»⟩][∂[⍎⇅⌫][⌫]⍥]?]?]≔K
[10⍢[∂⌷«0»=][∂#1-0⇅⊂]⟳∂⌷«.»=[∂#1-0⇅⊂][]?]≔Φ
[∂0<[±⍕«-»⇅⧺][⍕]?]≔Γ
[∂«»=[][⍙«int»=[Γ][⍙«float»=[∂0<[±Φ«-»⇅⧺][Φ]?][⍙«str»=[][∂⊃«e»=[1@«✗ »⇅⧺][⌫«…»]?]?]?]?]?]≔D
[∂«»=[⌫«»][⍙«int»=[⌫«n»][⍙«float»=[⌫«n»][⍙«str»=[⌫«t»][∂⊃«e»=[⌫«e»][⌫«p»]?]?]?]?]?]≔Λ
[⍙«list»=[[⍙∂«int»=⇅«float»=∨⇅⌫]⌿][⍙«str»=[⌫⟨⟩][⟨⇅⟩]?]?]≔Z
[∂«+»=[⌫+][∂«-»=[⌫-][∂«*»=[⌫×][∂«/»=[⌫÷][∂«^»=[⌫^][∂«%»=[⌫%][∂«&»=[⌫⇅⍕⇅⍕⧺][∂«=»=[⌫=][∂«<»=[⌫<][∂«>»=[⌫>][∂«<=»=[⌫≤][∂«>=»=[⌫≥][∂«<>»=[⌫≠][«bad op »⇅⍕⧺↯]?]?]?]?]?]?]?]?]?]?]?]?]?]≔O
[∂«^»=[⌫5][∂«*»=[⌫4][∂«/»=[⌫4][∂«%»=[⌫4][∂«+»=[⌫3][∂«-»=[⌫3][∂«&»=[⌫2][⌫1]?]?]?]?]?]?]?]≔V
[⊚⊚v⥀@⇅@∂⟨«⌛»⟩=[«⌛»↯][⍙«list»=[⌫Q«✗ in »⇅⧺↯][⥀⌫⇅⌫]?]?]≔R
[R∂«»=[⌫0][]?]≔W
[∂0@⇒f∂1@⇒h∂2@⇒u 3@⇒x u f-1+⍸[f+x h-1+⍸[h+⊚⇅R]∵⇅⌫]∵⟨⟩[⧺]⍀]≔G
※ T: tokenize formula text (past the =):  «text»T → tokens
※ tokens: ⟨«n» num⟩ ⟨«s» str⟩ ⟨«r» ⟨row col⟩⟩ ⟨«g» ⟨r1 c1 r2 c2⟩⟩
※         ⟨«f» NAME⟩ ⟨«o» op⟩ ⟨«(»⟩ ⟨«)»⟩ ⟨«,»⟩
[⇒t 0⇒i ⟨⟩⇒a
[i t#<][t i@⇒c
c« »=[i 1+⇒i][
c«"»=[i 1+⇒j[j t#<[t j@«"»≠][0]?][j 1+⇒j]⟳ j t#≥[«unterminated "»↯][]?
  a⟨«s»t i 1+j⊂⟩⟨⇅⟩⧺⇒a j 1+⇒i][
«0123456789.»c∈[i⇒j[j t#<[«0123456789.»t j@∈][0]?][j 1+⇒j]⟳
  t i j⊂∂[⍎⇅⌫][⌫«bad number »⇅⧺↯]⍥⟨«n»⥀⟩a⇅⟨⇅⟩⧺⇒a j⇒i][
«ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz»c∈[i⇒j
  [j t#<[«ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789»t j@∈][0]?][j 1+⇒j]⟳
  t i j⊂⇑⇒c
  c#2≥[Y c 0 1⊂∈[c 1 c#⊂∂[⍎⇅⌫1][⌫⌫0]⍥][0]?][0]?
  [∂1≥⊚N≤∧[1-Y c 0 1⊂⍷⟨⥀⥀⟩
    j t#<[t j@«:»=][0]?[j 1+⇒j j
      [j t#<[«ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789»t j@∈][0]?][j 1+⇒j]⟳
      t⇅j⊂⇑⇒c
      c#2≥[Y c 0 1⊂∈[c 1 c#⊂∂[⍎⇅⌫1][⌫⌫0]⍥][0]?][0]?
      [∂1≥⊚N≤∧[1-Y c 0 1⊂⍷⟨⥀⥀⟩⧺⟨«g»⥀⟩a⇅⟨⇅⟩⧺⇒a j⇒i][«bad range»↯]?][«bad range»↯]?
    ][⟨«r»⥀⟩a⇅⟨⇅⟩⧺⇒a j⇒i]?
  ][«ref out of grid »c⧺↯]?]
  [j[∂t#<[t⊚@« »=][0]?][1+]⟳∂t#<[t⊚@«(»=][0]?[⌫⟨«f»c⟩][⌫⟨«s»c⟩]?a⇅⟨⇅⟩⧺⇒a j⇒i]?][
c«<»=[i 1+t#<[t i 1+@∂«=»=[⌫«<=»2][∂«>»=[⌫«<>»2][⌫«<»1]?]?][«<»1]?
  ⇅⟨«o»⥀⟩a⇅⟨⇅⟩⧺⇒a i⇅+⇒i][
c«>»=[i 1+t#<[t i 1+@«=»=[«>=»2][«>»1]?][«>»1]?
  ⇅⟨«o»⥀⟩a⇅⟨⇅⟩⧺⇒a i⇅+⇒i][
«+-*/^%&=(),»c∈[
  c«-»=[a#0=[1][a⌷⊃∂«o»=⊚«(»=∨⇅«,»=∨]?[a⟨«n»0⟩⟨⇅⟩⧺⇒a][]?][]?
  c«(»=[⟨«(»⟩][c«)»=[⟨«)»⟩][c«,»=[⟨«,»⟩][⟨«o»c⟩]?]?]?a⇅⟨⇅⟩⧺⇒a i 1+⇒i][
c«#»=[«shifted off the grid»↯][«bad char »c⧺↯]?]?]?]?]?]?]?]?]⟳ a]≔T
※ P: shunting-yard — tokens → RPN.  E: evaluate RPN on the data stack.
[⟨⟩⇒o⟨⟩⇒q[⇒z
z⊃«n»=z⊃«s»=∨z⊃«r»=∨z⊃«g»=∨[q z⟨⇅⟩⧺⇒q][
z⊃«f»=z⊃«(»=∨[o z⟨⇅⟩⧺⇒o][
z⊃«,»=[[o#0>[o⌷⊃«(»≠][0]?][q o⌷⟨⇅⟩⧺⇒q o 0 o#1-⊂⇒o]⟳ o#0=[«misplaced ,»↯][]?][
z⊃«o»=[z 1@V z 1@«^»=[1+][]?⇒y
  [o#0>[o⌷∂⊃«o»=[1@V y≥][⌫0]?][0]?][q o⌷⟨⇅⟩⧺⇒q o 0 o#1-⊂⇒o]⟳ o z⟨⇅⟩⧺⇒o][
z⊃«)»=[[o#0>[o⌷⊃«(»≠][0]?][q o⌷⟨⇅⟩⧺⇒q o 0 o#1-⊂⇒o]⟳ o#0=[«unbalanced )»↯][]?o 0 o#1-⊂⇒o
  o#0>[o⌷⊃«f»=][0]?[q o⌷⟨⇅⟩⧺⇒q o 0 o#1-⊂⇒o][]?][]?]?]?]?]?]∀
[o#0>][o⌷⊃«(»=[«unbalanced (»↯][]?q o⌷⟨⇅⟩⧺⇒q o 0 o#1-⊂⇒o]⟳ q]≔P
[⇒z≢⇒m z[∂⊃«n»=[1@][∂⊃«s»=[1@][∂⊃«r»=[1@∂0@⇅1@W][∂⊃«g»=[1@G][∂⊃«o»=[1@O][1@F]?]?]?]?]?]∀≢m 1+≠[«bad formula»↯][]?]≔E
[⇅⇒l k l⒢∂∅=[⌫⌫ w l∈¬[w l⟨⇅⟩⧺⇒w][]?«⌛»↯][⍙«list»=[∂#2=[∂⊃«✗fetch»=[1@↯][]?][]?][]?⇅⒫∂∅=[⌫«no data at that path»↯][⍙«list»=[⌫«not a single value»↯][]?]?]?]≔H
[∂«SUM»=[⌫Z 0[+]⍀][
∂«AVG»=[⌫Z µ][
∂«MIN»=[⌫Z∂#0=[«MIN of nothing»↯][∂⊃[⊓]⍀]?][
∂«MAX»=[⌫Z∂#0=[«MAX of nothing»↯][∂⊃[⊔]⍀]?][
∂«CNT»=[⌫Z#][
∂«IF»=[⌫⥀[⌫][⇅⌫]?][
∂«ABS»=[⌫∣][
∂«ROUND»=[⌫⍢⍎][
∂«LEN»=[⌫⍕#][
∂«FX»=[⌫⇑⇅⇑«https://open.er-api.com/v6/latest/»⇅⧺⇅⟨⇅«rates»⇅⟩H][
∂«BTC»=[⌫⇩∂«https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=»⇅⧺⇅⟨⇅«bitcoin»⇅⟩H][
∂«WX»=[⌫⇅Γ«https://api.open-meteo.com/v1/forecast?current=temperature_2m&latitude=»⇅⧺«&longitude=»⧺⇅Γ⧺⟨«current» «temperature_2m»⟩H][
∂«GET»=[⌫«.»⊆[[∂⍎⇅⌫][⌫]⍥]∵H][
∂«NOW»=[⌫⌚1000÷⌊][
∂«TODAY»=[⌫⌚Θ∂0@⍕«-»⧺⇅∂1@②⥀⇅⧺«-»⧺⇅2@②⧺][
∂«TIME»=[⌫⌚1000÷⌊86400%∂3600÷⌊②«:»⧺⇅∂3600%60÷⌊②⥀⇅⧺«:»⧺⇅60%②⧺][
∂«SPARK»=[⌫Z∂#0=[⌫«»][∂∂⊃[⊓]⍀⇒ε∂∂⊃[⊔]⍀ε-⇒δ[δ0=[⌫4][ε-7×δ÷⌊0⊔7⊓]?«▁▂▃▄▅▆▇█»⇅∂1+⊂]∵«»⊇]?][
∂«UPPER»=[⌫⍕⇑][
∂«LOWER»=[⌫⍕⇩][
∂«LEFT»=[⌫⇅⍕0⥀⊂][
∂«RIGHT»=[⌫⇅⍕∂#⥀-0⊔⇅∂#⥀⇅⊂][
∂«MID»=[⌫⥀⍕⥀1-⥀⊚+⊂][
∂«SQRT»=[⌫√][
«unknown function »⇅⧺↯]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]≔F
※ C: recalculate v from b. Formula cells parse once, then evaluate in
※ passes until a fixpoint: a pass that resolves nothing leaves only
※ cells waiting on live data (w) — or, when w is empty, proven cycles.
[b[[K]∵]∵⇒v ⟨⟩⇒w ⟨⟩⇒p
N⍸[⇒e M⍸[⇒h v e@h@⟨«⌛»⟩=[
  [b e@h@∂#1⇅⊂T P⟨⇅⟩⟨e h⟩⇅⧺p⇅⟨⇅⟩⧺⇒p]
  [⟨«e»⥀⟩v⇅e h⥀S⇒v]⍥][]?]∀]∀
1⇒d[d p#0>∧][0⇒d⟨⟩⇒a p[⇒e
  [e 2@E v⇅e 0@e 1@⥀S⇒v 1⇒d]
  [∂«⌛»=[⌫a e⟨⇅⟩⧺⇒a][⟨«e»⥀⟩v⇅e 0@e 1@⥀S⇒v 1⇒d]?]⍥]∀
a⇒p]⟳
w#0=[p[⇒e v e 0@e 1@⟨«e» «⚠ circular»⟩S⇒v]∀][]?]≔C
※ Ξ: shift the cell references in a formula body by ⟨δrow δcol⟩ —
※ the relative-reference rewrite behind fill and paste. Quoted text
※ is copied verbatim; a reference pushed off the grid becomes #REF,
※ which the tokenizer reports as «shifted off the grid».
[⇒δ⇒ε⇒t 0⇒i«»⇒a[i t#<][t i@⇒c
c«"»=[i 1+⇒j[j t#<[t j@«"»≠][0]?][j 1+⇒j]⟳ a t i j t#⊓1+⊂⧺⇒a j 1+⇒i][
«ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz»c∈[i⇒j
[j t#<[«ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789»t j@∈][0]?][j 1+⇒j]⟳
t i j⊂∂⇑⇒c
c#2≥[Y c 0 1⊂∈[c 1 c#⊂∂[⍎⇅⌫1][⌫⌫0]⍥][0]?][0]?
[1-ε+⇒ζ⌫Y c 0 1⊂⍷δ+⇒ν ζ0≥ζN<∧ν0≥∧νM<∧[a Yν@⧺ζ1+⍕⧺⇒a][a«#REF»⧺⇒a]?]
[a⇅⧺⇒a]?j⇒i][
a c⧺⇒a i 1+⇒i]?]?]⟳ a]≔Ξ
※ ⌸: normalize a list of cell rows to exactly N rows of M cells.
[[[∂#M<][«»⟨⇅⟩⧺]⟳M⊤]∵[∂#N<][M⍸[⌫«»]∵⟨⇅⟩⧺]⟳N⊤]≔⌸
※ Σ: snapshot ⟨grid styles⟩ for undo (last 50); a new edit clears redo.
[θ⟨⟨b η⟩⟩⧺∂#50>[∂#50-⊥][]?⇒θ⟨⟩⇒λ]≔Σ
※ Ρ: a change happened — bump the version and answer every held poll.
[μ1+⇒μ ξ[⟨⇅200«application/json»J⟩↥β]∀⟨⟩⇒ξ]≔Ρ
※ J: the whole sheet as one JSON document.
[⟨⟨«name» n⟩ ⟨«rows» N⟩ ⟨«cols» M⟩ ⟨«v» μ⟩ ⟨«status» s⟩ ⟨«live» w#⟩ ⟨«styles» η⟩ ⟨«cells» b⟩ ⟨«disp» v[[D]∵]∵⟩ ⟨«kind» v[[Λ]∵]∵⟩⟩⒮]≔J
※ A: apply one cell edit {"r","c","t"} and recalculate.
[Σ∂«r»⒢⇒i∂«c»⒢⇒j«t»⒢⍕b⇅i j⥀S⇒b C Ρ]≔A
※ Π: apply a batch of edits {"edits":[{"r","c","t"}…]} in one recalc —
※ a block pasted from another spreadsheet arrives here.
[Σ«edits»⒢[∂«r»⒢⇒i∂«c»⒢⇒j«t»⒢⍕b⇅i j⥀S⇒b]∀C Ρ]≔Π
※ ∇: fill {"sr","sc","r0","c0","r1","c1"} — stamp the source cell over
※ the rectangle, shifting each formula's references by its offset.
[Σ∂«sr»⒢⇒σ∂«sc»⒢⇒ψ∂«r0»⒢⇒ε∂«r1»⒢⇅∂«c0»⒢⇒ζ«c1»⒢ζ-1+⍸[ζ+]∵⇒ϰ
ε-1+⍸[ε+]∵[⇒ω ϰ[⇒ς b σ@ψ@∂0 1⊂«=»=[∂#1⇅⊂ω σ-ς ψ-Ξ«=»⇅⧺][]?b⇅ω ς⥀S⇒b]∀]∀C Ρ]≔∇
※ ¤: set a cell's style flags {"r","c","s"} — bold, alignment, format.
[Σ∂«r»⒢⍕«,»⧺⇅∂«c»⒢⍕⥀⇅⧺⇒ι«s»⒢⍕⇒ο η[⊃ι≠]⌿ο«»≠[⟨⟨ι ο⟩⟩⧺][]?⇒η Ρ]≔¤
※ ↶ ↷: undo and redo — whole ⟨grid styles⟩ snapshots, like MatrixPad.
[θ#0>[λ⟨⟨b η⟩⟩⧺⇒λ θ⌷∂0@⇒b 1@⇒η θ 0 θ#1-⊂⇒θ C«undid»⇒s Ρ][«nothing to undo»⇒s]?]≔↶
[λ#0>[θ⟨⟨b η⟩⟩⧺⇒θ λ⌷∂0@⇒b 1@⇒η λ 0 λ#1-⊂⇒λ C«redid»⇒s Ρ][«nothing to redo»⇒s]?]≔↷
※ Δ: refresh — forget the cache, find every live url, fan the fetches
※ out to the worker pool on φ, fold the answers back in from ρ, recalc.
[⟨⟩⇒k C w#0>[w[↥φ]∀w#[↧ρ⟨⇅⟩k⇅⧺⇒k]⍣⟨⟩⇒w C«live data refreshed»⇒s Ρ][«no live cells in this sheet»⇒s]?]≔Δ
※ X: save the raw grid as honest TSV.
[n«»=[«zion-ledger.tsv»⇒n][]?b[9⍘⊇]∵«⏎»⊇«⏎»⧺[n⍈«saved as »n⧺⇒s][⌫«cannot write »n⧺⇒s]⍥]≔X
※ U: route one request r → the response ⟨id status type body⟩, or ∅
※ when the answer is deferred (a held poll waiting for the next change).
[r 2@«?»⊆∂⊃⇒ε∂#1>[1@][⌫«»]?⇒δ
r 1@«GET»=[ε«/»=ε«/index.html»=∨[⟨r⊃ 200«text/html; charset=utf-8»Ω⟩][
  ε«/api/sheet»=[⟨r⊃ 200«application/json»J⟩][
  ε«/api/poll»=[δ«=»⊆∂#1>[1@[⍎][⌫0]⍥][⌫0]?μ<[⟨r⊃ 200«application/json»J⟩][ξ r⊃⟨⇅⟩⧺⇒ξ∅]?][
  ⟨r⊃ 404«text/plain»«lost in the Matrix»⟩]?]?]?][
r 1@«POST»=[
  ε«/api/cell»=[r 3@⒥A⟨r⊃ 200«application/json»J⟩][
  ε«/api/batch»=[r 3@⒥Π⟨r⊃ 200«application/json»J⟩][
  ε«/api/fill»=[r 3@⒥∇⟨r⊃ 200«application/json»J⟩][
  ε«/api/style»=[r 3@⒥¤⟨r⊃ 200«application/json»J⟩][
  ε«/api/refresh»=[Δ⟨r⊃ 200«application/json»J⟩][
  ε«/api/undo»=[↶⟨r⊃ 200«application/json»J⟩][
  ε«/api/redo»=[↷⟨r⊃ 200«application/json»J⟩][
  ε«/api/save»=[X⟨r⊃ 200«application/json»J⟩][
  ⟨r⊃ 404«text/plain»«lost in the Matrix»⟩]?]?]?]?]?]?]?]?][
⟨r⊃ 404«text/plain»«lost in the Matrix»⟩]?]?]≔U
※ Ψ: the demo ledger.
⟨⟨«THE ZION LEDGER» «» «» «» «» «» «» «»⟩
⟨«item» «EUR» «rate» «USD» «» «» «» «»⟩
⟨«hovercraft fuel» «1200» «=FX(EUR,USD)» «=B3*C3» «» «» «» «»⟩
⟨«EMP charges» «350» «=C3» «=B4*C4» «» «» «» «»⟩
⟨«real steak dinner» «42.5» «=C3» «=B5*C5» «» «» «» «»⟩
⟨«total» «=SUM(B3:B5)» «» «=SUM(D3:D5)» «» «» «» «»⟩
⟨«» «» «» «» «» «» «» «»⟩
⟨«BTC reserve» «0.25» «=BTC(USD)» «=B8*C8» «» «» «» «»⟩
⟨«status» «=IF(D8>10000,"we are rich","keep mining")» «» «» «» «» «» «»⟩
⟨«» «» «» «» «» «» «» «»⟩
⟨«Zion weather °C» «=WX(-33.86,151.21)» «» «» «» «» «» «»⟩
⟨«the answer» «=ROUND(6*7.0,0)» «» «» «» «» «» «»⟩
⟨«today» «=TODAY()» «utc» «=TIME()» «» «» «» «»⟩
⟨«usd trend» «=SPARK(D3:D6)» «» «» «» «» «» «»⟩⟩≔Ψ
※ B: open the sheet named on the command line, or wake into the demo.
[⟨⟩⇒k⟨⟩⇒w«»⇒s⟨⟩⇒θ⟨⟩⇒λ1⇒μ⟨⟩⇒ξ⟨⟩⇒η⌂#0>[⌂⊃⇒n
[n⍇«⏎»⊆[9⍘⊆]∵⌸⇒b«opened »n⧺⇒s]
[⌫Ψ⌸⇒b«new sheet »n⧺⇒s]⍥]
[Ψ⌸⇒b«»⇒n«the demo ledger — click a cell and type»⇒s
⟨⟨«0,0» «b»⟩ ⟨«1,0» «b»⟩ ⟨«1,1» «b»⟩ ⟨«1,2» «b»⟩ ⟨«1,3» «b»⟩ ⟨«2,3» «$»⟩ ⟨«3,3» «$»⟩ ⟨«4,3» «$»⟩ ⟨«5,3» «$»⟩ ⟨«7,3» «$»⟩⟩⇒η]?]≔B
※ Ω: the page itself — the whole front end, served at /.
«<!doctype html>⏎
<html lang="en">⏎
<head>⏎
<meta charset="utf-8">⏎
<meta name="viewport" content="width=device-width,initial-scale=1">⏎
<title>THE ARCHITECT</title>⏎
<link rel="icon" href="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'%3E%3Crect width='16' height='16' fill='%23071008'/%3E%3Cpath d='M3 13 8 3l5 10H3z' fill='none' stroke='%2335ff9d' stroke-width='1.5'/%3E%3C/svg%3E">⏎
<style>⏎
  :root { --bg:#060b07; --panel:#0a130c; --line:#12351f; --dim:#2e7d4f;⏎
          --ink:#b8e6c9; --hot:#35ff9d; --num:#e4fff0; --txt:#8fcfa8;⏎
          --err:#ff6161; --sel:#123f26; --fill:#0d2c1b; }⏎
  * { box-sizing:border-box; margin:0; }⏎
  html,body { height:100%; }⏎
  body { background:var(--bg); color:var(--ink); overflow:hidden;⏎
         font:13px/1.45 ui-monospace,'Cascadia Code',Menlo,Consolas,monospace; }⏎
  #rain { position:fixed; inset:0; opacity:.33; pointer-events:none; }⏎
  main { position:relative; height:100%; display:flex; flex-direction:column;⏎
         gap:8px; padding:14px 18px; max-width:1280px; margin:0 auto; }⏎
  header { display:flex; align-items:center; gap:12px; flex-wrap:wrap; }⏎
  h1 { font-size:17px; letter-spacing:.35em; color:var(--hot);⏎
       text-shadow:0 0 18px rgba(53,255,157,.45); font-weight:600; }⏎
  .sub { color:var(--dim); font-size:11px; flex:1; }⏎
  button { background:var(--panel); color:var(--ink); border:1px solid var(--line);⏎
           border-radius:5px; padding:5px 10px; font:inherit; cursor:pointer; }⏎
  button:hover { border-color:var(--hot); color:var(--hot);⏎
                 box-shadow:0 0 12px rgba(53,255,157,.25); }⏎
  button:disabled { opacity:.45; cursor:wait; }⏎
  button.on { color:var(--hot); border-color:var(--hot); }⏎
  .actions { display:flex; gap:6px; }⏎
  #livecount { color:var(--hot); margin-left:5px; }⏎
  #fbar { display:flex; border:1px solid var(--line); border-radius:5px;⏎
          background:var(--panel); overflow:hidden; }⏎
  #cellname { padding:6px 10px; color:var(--hot); border-right:1px solid var(--line);⏎
              min-width:52px; text-align:center; }⏎
  #formula { flex:1; background:transparent; border:0; outline:0; color:var(--num);⏎
             font:inherit; padding:6px 10px; }⏎
  #formula::placeholder { color:#1d5433; }⏎
  #gridwrap { overflow:auto; border:1px solid var(--line); border-radius:5px;⏎
              background:rgba(7,14,9,.9); flex:1; position:relative; }⏎
  table { border-collapse:separate; border-spacing:0; table-layout:fixed; }⏎
  th,td { border-right:1px solid #0e2716; border-bottom:1px solid #0e2716;⏎
          padding:3px 7px; height:26px; white-space:nowrap; overflow:hidden;⏎
          text-overflow:ellipsis; }⏎
  th { color:var(--dim); background:#081209; font-weight:400; font-size:10px;⏎
       position:sticky; top:0; z-index:2; position:sticky; }⏎
  th.rz { cursor:col-resize; }⏎
  td.rowh, th.corner { color:var(--dim); background:#081209; text-align:center;⏎
            font-size:10px; position:sticky; left:0; z-index:1; }⏎
  th.corner { z-index:3; }⏎
  td.n { text-align:right; color:var(--num); }⏎
  td.t { color:var(--txt); }⏎
  td.e { color:var(--err); }⏎
  td.p { color:var(--dim); font-style:italic; animation:breathe 1.6s infinite; }⏎
  td.f { text-shadow:0 0 10px rgba(53,255,157,.35); }⏎
  td.sb { font-weight:700; color:var(--num); }⏎
  td.sl { text-align:left; } td.sc { text-align:center; } td.sr { text-align:right; }⏎
  @keyframes breathe { 50% { opacity:.35; } }⏎
  td.sel { outline:2px solid var(--hot); outline-offset:-2px; background:var(--sel); }⏎
  td.fill { background:var(--fill); }⏎
  td input { width:100%; background:#04120a; color:var(--hot); border:0; outline:0;⏎
             font:inherit; padding:0; }⏎
  #handle { position:absolute; width:8px; height:8px; background:var(--hot);⏎
            border:1px solid #063; cursor:crosshair; z-index:4; display:none; }⏎
  footer { display:flex; gap:16px; font-size:11px; color:var(--dim); }⏎
  #status { color:var(--ink); flex:1; }⏎
  #status.err { color:var(--err); }⏎
  #hint { text-align:right; }⏎
  @media (max-width:980px){ #hint { display:none; } }⏎
</style>⏎
</head>⏎
<body>⏎
<canvas id="rain"></canvas>⏎
<main>⏎
  <header>⏎
    <h1>THE ARCHITECT</h1>⏎
    <p class="sub">a live multiplayer spreadsheet, served entirely by MLang</p>⏎
    <div class="actions">⏎
      <button id="bold" title="bold (Ctrl+B)"><b>B</b></button>⏎
      <button id="align" title="cycle alignment">&#8676;</button>⏎
      <button id="fmt" title="cycle number format">1.2</button>⏎
      <button id="undo" title="undo (Ctrl+Z)">&#8630;</button>⏎
      <button id="redo" title="redo (Ctrl+Y)">&#8631;</button>⏎
      <button id="refresh" title="fetch live data (Ctrl+R)">&#8635; live<span id="livecount"></span></button>⏎
      <button id="save" title="save as TSV (Ctrl+S)">&#8681; save</button>⏎
    </div>⏎
  </header>⏎
  <div id="fbar">⏎
    <span id="cellname">A1</span>⏎
    <input id="formula" spellcheck="false" autocomplete="off"⏎
           placeholder="cell contents — start with = for a formula">⏎
  </div>⏎
  <div id="gridwrap"><table id="grid"></table><div id="handle"></div></div>⏎
  <footer>⏎
    <span id="status">jacking in&#8230;</span>⏎
    <span id="hint">=SUM(B3:B5) &#183; =IF(D8&gt;100,"y","n") &#183; =FX(EUR,USD) &#183; =BTC(USD) &#183; =WX(-33.9,151.2) &#183; =GET("url","path") &#183; =SPARK(D3:D9) &#183; =TODAY() &#183; drag the corner handle to fill</span>⏎
  </footer>⏎
</main>⏎
<script>⏎
'use strict';⏎
let S = null, sel = {r:2, c:1}, editing = false, pendingS = null;⏎
let copySrc = null, fillTo = null, colW = {};⏎
try { colW = JSON.parse(localStorage.archCols || '{}'); } catch (e) {}⏎
const $ = id => document.getElementById(id);⏎
const grid = $('grid'), fbar = $('formula'), handle = $('handle');⏎
const colName = c => String.fromCharCode(65 + c);⏎
const status = (m, bad) => { $('status').textContent = m; $('status').className = bad ? 'err' : ''; };⏎
const styleOf = (r, c) => (S && S.styleMap[r + ',' + c]) || '';⏎
const fmts = ['', '2', '$', '€', '%', '0'];⏎
const fmtNum = (txt, st) => {⏎
  const f = fmts.find(x => x && st.includes(x));⏎
  if (!f) return txt;⏎
  const x = parseFloat(txt);⏎
  if (isNaN(x)) return txt;⏎
  const o = { minimumFractionDigits: 2, maximumFractionDigits: 2 };⏎
  if (f === '2') return x.toLocaleString('en-US', o);⏎
  if (f === '$') return x.toLocaleString('en-US', { style:'currency', currency:'USD' });⏎
  if (f === '€') return x.toLocaleString('de-DE', { style:'currency', currency:'EUR' });⏎
  if (f === '%') return (100*x).toLocaleString('en-US', { maximumFractionDigits: 1 }) + '%';⏎
  if (f === '0') return Math.round(x).toLocaleString('en-US');⏎
  return txt;⏎
};⏎
⏎
async function api(path, body) {⏎
  const opts = body === undefined ? {} : {method:'POST', body:JSON.stringify(body)};⏎
  try {⏎
    const resp = await fetch(path, opts);⏎
    const text = await resp.text();⏎
    if (!resp.ok) { status('the grid answered ' + resp.status + ': ' + text, true); return; }⏎
    take(JSON.parse(text));⏎
  } catch (e) { status('cannot reach the grid — is the server still up?', true); }⏎
}⏎
⏎
function take(sheet) {⏎
  sheet.styleMap = {};⏎
  (Array.isArray(sheet.styles) ? [] : Object.entries(sheet.styles || {})).forEach(([k,v]) => sheet.styleMap[k] = v);⏎
  S = sheet;⏎
  if (editing) { pendingS = sheet; return; }⏎
  render();⏎
}⏎
⏎
async function poll() {⏎
  while (true) {⏎
    if (!S) { await new Promise(r => setTimeout(r, 800)); continue; }⏎
    try {⏎
      const resp = await fetch('/api/poll?v=' + S.v);⏎
      if (resp.status === 204) return;⏎
      if (!resp.ok) { await new Promise(r => setTimeout(r, 1500)); continue; }⏎
      take(JSON.parse(await resp.text()));⏎
    } catch (e) { await new Promise(r => setTimeout(r, 1500)); }⏎
  }⏎
}⏎
⏎
function render() {⏎
  if (!S) return;⏎
  let h = '<colgroup><col style="width:36px">';⏎
  for (let c = 0; c < S.cols; c++)⏎
    h += '<col style="width:' + (colW[c] || (c === 0 ? 130 : 92)) + 'px">';⏎
  h += '</colgroup><tr><th class="corner"></th>';⏎
  for (let c = 0; c < S.cols; c++) h += '<th class="rz" data-c="' + c + '">' + colName(c) + '</th>';⏎
  h += '</tr>';⏎
  for (let r = 0; r < S.rows; r++) {⏎
    h += '<tr><td class="rowh">' + (r+1) + '</td>';⏎
    for (let c = 0; c < S.cols; c++) {⏎
      const kind = S.kind[r][c], raw = S.cells[r][c], st = styleOf(r, c);⏎
      const cls = [kind, raw[0] === '=' && kind === 'n' ? 'f' : '',⏎
                   st.includes('b') ? 'sb' : '',⏎
                   st.includes('l') ? 'sl' : st.includes('c') ? 'sc' : st.includes('r') ? 'sr' : '',⏎
                   r === sel.r && c === sel.c ? 'sel' : '',⏎
                   fillTo && inRect(r, c, fillTo) ? 'fill' : ''].join(' ').trim();⏎
      let disp = kind === 'n' ? fmtNum(S.disp[r][c], st) : S.disp[r][c];⏎
      disp = disp.replace(/&/g,'&amp;').replace(/</g,'&lt;');⏎
      h += '<td data-r="' + r + '" data-c="' + c + '" class="' + cls + '">' + disp + '</td>';⏎
    }⏎
    h += '</tr>';⏎
  }⏎
  grid.innerHTML = h;⏎
  $('livecount').textContent = S.live > 0 ? ' (' + S.live + ')' : '';⏎
  document.title = 'THE ARCHITECT — ' + (S.name || 'zion ledger');⏎
  if (!editing) { fbar.value = S.cells[sel.r][sel.c]; $('cellname').textContent = colName(sel.c) + (sel.r+1); }⏎
  const st = styleOf(sel.r, sel.c);⏎
  $('bold').className = st.includes('b') ? 'on' : '';⏎
  $('align').textContent = st.includes('l') ? '⇤' : st.includes('c') ? '↔' : st.includes('r') ? '⇥' : '⇤';⏎
  $('fmt').textContent = fmts.find(x => x && st.includes(x)) || '1.2';⏎
  status(S.status || 'resident');⏎
  placeHandle();⏎
}⏎
⏎
const inRect = (r, c, t) =>⏎
  r >= Math.min(sel.r, t.r) && r <= Math.max(sel.r, t.r) &&⏎
  c >= Math.min(sel.c, t.c) && c <= Math.max(sel.c, t.c);⏎
⏎
function cellAt(r, c) { return grid.querySelector('td[data-r="' + r + '"][data-c="' + c + '"]'); }⏎
⏎
function placeHandle() {⏎
  const td = cellAt(sel.r, sel.c);⏎
  if (!td || editing) { handle.style.display = 'none'; return; }⏎
  handle.style.display = 'block';⏎
  handle.style.left = (td.offsetLeft + td.offsetWidth - 5) + 'px';⏎
  handle.style.top = (td.offsetTop + td.offsetHeight - 5) + 'px';⏎
}⏎
⏎
function select(r, c) {⏎
  if (!S) return;⏎
  sel = { r: Math.max(0, Math.min(S.rows-1, r)), c: Math.max(0, Math.min(S.cols-1, c)) };⏎
  render();⏎
  const td = cellAt(sel.r, sel.c);⏎
  if (td) td.scrollIntoView({ block:'nearest', inline:'nearest' });⏎
}⏎
⏎
function edit(initial) {⏎
  const td = cellAt(sel.r, sel.c);⏎
  if (!td || editing) return;⏎
  editing = true;⏎
  const input = document.createElement('input');⏎
  input.value = initial !== undefined ? initial : S.cells[sel.r][sel.c];⏎
  td.textContent = '';⏎
  td.appendChild(input);⏎
  input.focus();⏎
  input.setSelectionRange(input.value.length, input.value.length);⏎
  input.addEventListener('keydown', e => {⏎
    if (e.key === 'Enter')      { commit(input.value, 1, 0); e.preventDefault(); }⏎
    else if (e.key === 'Tab')   { commit(input.value, 0, 1); e.preventDefault(); }⏎
    else if (e.key === 'Escape'){ finishEdit(); render(); }⏎
    e.stopPropagation();⏎
  });⏎
  input.addEventListener('blur', () => { if (editing) commit(input.value, 0, 0); });⏎
  handle.style.display = 'none';⏎
}⏎
⏎
function finishEdit() {⏎
  editing = false;⏎
  if (pendingS) { const p = pendingS; pendingS = null; S = p; }⏎
}⏎
⏎
function commit(text, dr, dc) {⏎
  if (!editing) return;⏎
  finishEdit();⏎
  const r = sel.r, c = sel.c;⏎
  sel = { r: Math.min(S.rows-1, r+dr), c: Math.min(S.cols-1, c+dc) };⏎
  api('/api/cell', { r:r, c:c, t:text });⏎
}⏎
⏎
grid.addEventListener('mousedown', e => {⏎
  const th = e.target.closest('th.rz');⏎
  if (th && e.offsetX > th.offsetWidth - 6) { startResize(e, +th.dataset.c); e.preventDefault(); return; }⏎
  const td = e.target.closest('td[data-r]');⏎
  if (!td) return;⏎
  const r = +td.dataset.r, c = +td.dataset.c;⏎
  if (r === sel.r && c === sel.c && !editing) { edit(); e.preventDefault(); return; }⏎
  if (editing) return;⏎
  select(r, c);⏎
});⏎
grid.addEventListener('dblclick', () => { if (!editing) edit(); });⏎
⏎
function startResize(e, c) {⏎
  const x0 = e.clientX, w0 = colW[c] || (c === 0 ? 130 : 92);⏎
  const move = ev => { colW[c] = Math.max(40, w0 + ev.clientX - x0); render(); };⏎
  const up = () => {⏎
    removeEventListener('mousemove', move); removeEventListener('mouseup', up);⏎
    localStorage.archCols = JSON.stringify(colW);⏎
  };⏎
  addEventListener('mousemove', move); addEventListener('mouseup', up);⏎
}⏎
⏎
handle.addEventListener('mousedown', e => {⏎
  e.preventDefault();⏎
  fillTo = { r: sel.r, c: sel.c };⏎
  const move = ev => {⏎
    const td = document.elementFromPoint(ev.clientX, ev.clientY);⏎
    const cell = td && td.closest('td[data-r]');⏎
    if (cell) { fillTo = { r: +cell.dataset.r, c: +cell.dataset.c }; render(); }⏎
  };⏎
  const up = () => {⏎
    removeEventListener('mousemove', move); removeEventListener('mouseup', up);⏎
    const t = fillTo; fillTo = null;⏎
    if (t && (t.r !== sel.r || t.c !== sel.c))⏎
      api('/api/fill', { sr:sel.r, sc:sel.c,⏎
        r0:Math.min(sel.r,t.r), c0:Math.min(sel.c,t.c),⏎
        r1:Math.max(sel.r,t.r), c1:Math.max(sel.c,t.c) });⏎
    else render();⏎
  };⏎
  addEventListener('mousemove', move); addEventListener('mouseup', up);⏎
});⏎
⏎
function cycleStyle(kind) {⏎
  let st = styleOf(sel.r, sel.c);⏎
  if (kind === 'b') st = st.includes('b') ? st.replace('b','') : st + 'b';⏎
  if (kind === 'a') {⏎
    const cur = st.includes('l') ? 'l' : st.includes('c') ? 'c' : st.includes('r') ? 'r' : '';⏎
    st = st.replace(/[lcr]/g, '') + ({'':'l', l:'c', c:'r', r:''}[cur]);⏎
  }⏎
  if (kind === 'f') {⏎
    const cur = fmts.find(x => x && st.includes(x)) || '';⏎
    const next = fmts[(fmts.indexOf(cur) + 1) % fmts.length];⏎
    st = st.replace(/[2$€%0]/g, '') + next;⏎
  }⏎
  api('/api/style', { r:sel.r, c:sel.c, s:st });⏎
}⏎
$('bold').addEventListener('click', () => cycleStyle('b'));⏎
$('align').addEventListener('click', () => cycleStyle('a'));⏎
$('fmt').addEventListener('click', () => cycleStyle('f'));⏎
$('undo').addEventListener('click', () => api('/api/undo', {}));⏎
$('redo').addEventListener('click', () => api('/api/redo', {}));⏎
$('save').addEventListener('click', () => api('/api/save', {}));⏎
$('refresh').addEventListener('click', async () => {⏎
  $('refresh').disabled = true;⏎
  status('reaching the outside world…');⏎
  await api('/api/refresh', {});⏎
  $('refresh').disabled = false;⏎
});⏎
⏎
document.addEventListener('copy', e => {⏎
  if (editing || !S) return;⏎
  copySrc = { r: sel.r, c: sel.c, raw: S.cells[sel.r][sel.c] };⏎
  e.clipboardData.setData('text/plain', copySrc.raw);⏎
  e.preventDefault();⏎
  status('copied ' + colName(sel.c) + (sel.r+1));⏎
});⏎
document.addEventListener('paste', e => {⏎
  if (editing || !S) return;⏎
  e.preventDefault();⏎
  const text = e.clipboardData.getData('text/plain');⏎
  if (copySrc && text === copySrc.raw) {⏎
    api('/api/fill', { sr:copySrc.r, sc:copySrc.c, r0:sel.r, c0:sel.c, r1:sel.r, c1:sel.c });⏎
    return;⏎
  }⏎
  if (text.includes('\t') || text.includes('\n')) {⏎
    const edits = [];⏎
    text.replace(/\r/g,'').split('\n').forEach((line, dr) => {⏎
      if (line === '' && dr > 0) return;⏎
      line.split('\t').forEach((t, dc) => {⏎
        const r = sel.r + dr, c = sel.c + dc;⏎
        if (r < S.rows && c < S.cols) edits.push({ r:r, c:c, t:t });⏎
      });⏎
    });⏎
    api('/api/batch', { edits: edits });⏎
    return;⏎
  }⏎
  api('/api/cell', { r:sel.r, c:sel.c, t:text });⏎
});⏎
⏎
document.addEventListener('keydown', e => {⏎
  if (editing || !S) return;⏎
  if (e.ctrlKey || e.metaKey) {⏎
    if (e.key === 's') { $('save').click(); e.preventDefault(); }⏎
    if (e.key === 'r') { $('refresh').click(); e.preventDefault(); }⏎
    if (e.key === 'z') { $('undo').click(); e.preventDefault(); }⏎
    if (e.key === 'y') { $('redo').click(); e.preventDefault(); }⏎
    if (e.key === 'b') { $('bold').click(); e.preventDefault(); }⏎
    return; // c and v fall through to the copy/paste events⏎
  }⏎
  switch (e.key) {⏎
    case 'ArrowUp':    select(sel.r-1, sel.c); e.preventDefault(); break;⏎
    case 'ArrowDown':  select(sel.r+1, sel.c); e.preventDefault(); break;⏎
    case 'ArrowLeft':  select(sel.r, sel.c-1); e.preventDefault(); break;⏎
    case 'ArrowRight': select(sel.r, sel.c+1); e.preventDefault(); break;⏎
    case 'Tab':        select(sel.r, sel.c+1); e.preventDefault(); break;⏎
    case 'Enter': case 'F2': edit(); e.preventDefault(); break;⏎
    case 'Delete': case 'Backspace':⏎
      api('/api/cell', { r:sel.r, c:sel.c, t:'' }); e.preventDefault(); break;⏎
    default:⏎
      if (e.key.length === 1) { edit(e.key); e.preventDefault(); }⏎
  }⏎
});⏎
⏎
fbar.addEventListener('focus', () => { editing = true; });⏎
fbar.addEventListener('keydown', e => {⏎
  if (e.key === 'Enter')  { finishEdit(); api('/api/cell', { r:sel.r, c:sel.c, t:fbar.value }); fbar.blur(); }⏎
  if (e.key === 'Escape') { finishEdit(); render(); fbar.blur(); }⏎
  e.stopPropagation();⏎
});⏎
fbar.addEventListener('blur', () => { if (editing) { finishEdit(); render(); } });⏎
⏎
const cv = $('rain'), cx = cv.getContext('2d');⏎
const glyphs = 'アイウエオカキクケコサシスセソ0123456789∂⇅⌫⧺⍞↥↧⇈⇉⇟⚡';⏎
let drops = [];⏎
function sizeRain() {⏎
  cv.width = innerWidth; cv.height = innerHeight;⏎
  drops = Array.from({length: Math.floor(innerWidth/18)}, () => Math.random()*innerHeight);⏎
}⏎
addEventListener('resize', sizeRain); sizeRain();⏎
setInterval(() => {⏎
  cx.fillStyle = 'rgba(4,8,5,.13)'; cx.fillRect(0, 0, cv.width, cv.height);⏎
  cx.font = '15px monospace';⏎
  drops.forEach((y, i) => {⏎
    cx.fillStyle = 'rgba(0,190,95,' + (Math.random()*.32 + .08) + ')';⏎
    cx.fillText(glyphs[Math.random()*glyphs.length|0], i*18, y);⏎
    drops[i] = y > innerHeight + 300 ? 0 : y + 15;⏎
  });⏎
}, 95);⏎
⏎
api('/api/sheet').then(poll);⏎
</script>⏎
</body>⏎
</html>⏎»≔Ω
⇊
1⇒g[g][⎆∂∅=[⌫0⇒g∅↥κ][↥κ]?]⟳
1⇒g B C[g][↧κ∂∅=[⌫0⇒g ξ[⟨⇅204«text/plain»«»⟩↥β]∀⟨⟩⇒ξ 3[∅↥φ]⍣∅↥β][⇒r[r U∂∅=[⌫][↥β]?][⍕«✗ »⇅⧺∂⇒s⟨r⊃ 500«text/plain»⟩⇅⟨⇅⟩⧺↥β]⍥]?]⟳
[↧β∂∅≠][⍅]⟳⌫
[↧φ∂∅≠][∂[⍆⒥][⟨⇅«✗fetch»⇅⟩]⍥⟨⥀⥀⟩↥ρ]⟳⌫
[↧φ∂∅≠][∂[⍆⒥][⟨⇅«✗fetch»⇅⟩]⍥⟨⥀⥀⟩↥ρ]⟳⌫
[↧φ∂∅≠][∂[⍆⒥][⟨⇅«✗fetch»⇅⟩]⍥⟨⥀⥀⟩↥ρ]⟳⌫
