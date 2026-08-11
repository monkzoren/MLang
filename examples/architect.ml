※ THE ARCHITECT — a live spreadsheet in your browser, served by MLang.
※
※ The killer application was always the spreadsheet; MLang's motto was
※ always «programs are grids». This program is both: a full web
※ application in one MLang file. Serve it and open the page:
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
※ The engine wraps each request in ⍥, so a broken formula or a bad body
※ becomes a 500 answer and a ✗ status — the server itself cannot die.
※ Shutdown is a clean cascade of ∅ poison pills: stdin closing (replay
※ mode) ends the acceptor, which ends the engine, which ends the pool
※ and the responder. Nothing hangs; nothing is left blocked.
※
※ Formulas (typed into any cell after =):
※   arithmetic  + - * / ^ %  ·  ( )  ·  unary minus  ·  & concatenates
※   compare     = <> < <= > >=      ·  "quoted" or bare strings
※   refs        B3 · ranges B3:D5   ·  SUM AVG MIN MAX CNT over ranges
※   logic       IF(cond,then,else)  ·  ABS ROUND LEN
※   live data   FX(EUR,USD) exchange rates · BTC(USD) bitcoin price
※               WX(lat,lon) temperature °C · GET("url","path.to.field")
※ Live cells show … until ↻ refresh fans their urls out to the fetcher
※ pool (run with --parallel and the fetches overlap for real); every
※ response is JSON read by the Operator (std/json.ml). A formula error
※ shows ✗ in that cell alone; circular references are detected by the
※ recalculation fixpoint and marked ⚠ — the sheet never hangs.
※
※ The sheet arrives and leaves as honest TSV (⍇/⍈), so it pastes
※ straight into and out of any other spreadsheet.
※
※ Engine-strand register file (strand-locals, single letters):
※   persistent   b raw grid · v values · k live cache · n file name
※                w wanted urls · s status line · g run flag
※   tokenizer T  t i a c j      parser P  o q z y      eval E  z m
※   range G      f h u x        live H  l              set-cell S  z m y
※   recalc C     p d e h        edits A  i j           router U  r
12≔N
8≔M
«ABCDEFGH»≔Y
[Y⇅@⇅1+⍕⧺]≔Q
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
«bad char »c⧺↯]?]?]?]?]?]?]?]⟳ a]≔T
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
«unknown function »⇅⧺↯]?]?]?]?]?]?]?]?]?]?]?]?]?]≔F
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
※ J: the whole sheet as one JSON document — raw text, display strings,
※ and a kind per cell («» empty · n number · t text · e error · p pending).
[⟨⟨«name» n⟩ ⟨«rows» N⟩ ⟨«cols» M⟩ ⟨«status» s⟩ ⟨«live» w#⟩ ⟨«cells» b⟩ ⟨«disp» v[[D]∵]∵⟩ ⟨«kind» v[[Λ]∵]∵⟩⟩⒮]≔J
※ A: apply one cell edit {"r":…,"c":…,"t":…} and recalculate.
[∂«r»⒢⇒i∂«c»⒢⇒j«t»⒢⍕b⇅i j⥀S⇒b C]≔A
※ Δ: refresh — forget the cache, find every live url, fan the fetches
※ out to the worker pool on φ, fold the answers back in from ρ, recalc.
[⟨⟩⇒k C w#0>[w[↥φ]∀w#[↧ρ⟨⇅⟩k⇅⧺⇒k]⍣⟨⟩⇒w C«live data refreshed»⇒s][«no live cells in this sheet»⇒s]?]≔Δ
※ X: save the raw grid as honest TSV.
[n«»=[«zion-ledger.tsv»⇒n][]?b[9⍘⊇]∵«⏎»⊇«⏎»⧺[n⍈«saved as »n⧺⇒s][⌫«cannot write »n⧺⇒s]⍥]≔X
※ U: route one request r → the response ⟨id status type body⟩.
[r 1@«GET»=[r 2@«/»=r 2@«/index.html»=∨[⟨r⊃ 200«text/html; charset=utf-8»Ω⟩][r 2@«/api/sheet»=[⟨r⊃ 200«application/json»J⟩][⟨r⊃ 404«text/plain»«lost in the Matrix»⟩]?]?][r 1@«POST»=[r 2@«/api/cell»=[r 3@⒥A⟨r⊃ 200«application/json»J⟩][r 2@«/api/refresh»=[Δ⟨r⊃ 200«application/json»J⟩][r 2@«/api/save»=[X⟨r⊃ 200«application/json»J⟩][⟨r⊃ 404«text/plain»«lost in the Matrix»⟩]?]?]?][⟨r⊃ 404«text/plain»«lost in the Matrix»⟩]?]?]≔U
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
⟨«the answer» «=ROUND(6*7.0,0)» «» «» «» «» «» «»⟩⟩≔Ψ
※ B: open the sheet named on the command line, or wake into the demo.
[⟨⟩⇒k⟨⟩⇒w«»⇒s⌂#0>[⌂⊃⇒n
[n⍇«⏎»⊆[9⍘⊆[∂#M<][«»⟨⇅⟩⧺]⟳M⊤]∵[∂#N<][M⍸[⌫«»]∵⟨⇅⟩⧺]⟳N⊤⇒b«opened »n⧺⇒s]
[⌫Ψ⇒b«new sheet »n⧺⇒s]⍥]
[Ψ⇒b«»⇒n«the demo ledger — click a cell and type»⇒s]?]≔B
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
          --err:#ff6161; --sel:#123f26; }⏎
  * { box-sizing:border-box; margin:0; }⏎
  html,body { height:100%; }⏎
  body { background:var(--bg); color:var(--ink); overflow:hidden;⏎
         font:14px/1.45 ui-monospace,'Cascadia Code',Menlo,Consolas,monospace; }⏎
  #rain { position:fixed; inset:0; opacity:.33; pointer-events:none; }⏎
  main { position:relative; height:100%; display:flex; flex-direction:column;⏎
         gap:10px; padding:18px 22px; max-width:1120px; margin:0 auto; }⏎
  header { display:flex; align-items:baseline; gap:14px; flex-wrap:wrap; }⏎
  h1 { font-size:19px; letter-spacing:.35em; color:var(--hot);⏎
       text-shadow:0 0 18px rgba(53,255,157,.45); font-weight:600; }⏎
  .sub { color:var(--dim); font-size:12px; flex:1; }⏎
  .actions { display:flex; gap:8px; }⏎
  button { background:var(--panel); color:var(--ink); border:1px solid var(--line);⏎
           border-radius:6px; padding:6px 13px; font:inherit; cursor:pointer; }⏎
  button:hover { border-color:var(--hot); color:var(--hot);⏎
                 box-shadow:0 0 14px rgba(53,255,157,.25); }⏎
  button:disabled { opacity:.45; cursor:wait; }⏎
  #livecount { color:var(--hot); margin-left:6px; }⏎
  #fbar { display:flex; gap:0; border:1px solid var(--line); border-radius:6px;⏎
          background:var(--panel); overflow:hidden; }⏎
  #cellname { padding:7px 12px; color:var(--hot); border-right:1px solid var(--line);⏎
              min-width:52px; text-align:center; }⏎
  #formula { flex:1; background:transparent; border:0; outline:0; color:var(--num);⏎
             font:inherit; padding:7px 12px; }⏎
  #formula::placeholder { color:#1d5433; }⏎
  #gridwrap { overflow:auto; border:1px solid var(--line); border-radius:6px;⏎
              background:rgba(7,14,9,.88); backdrop-filter:blur(2px); flex:1; }⏎
  table { border-collapse:collapse; width:100%; }⏎
  th,td { border:1px solid #0e2716; padding:5px 9px; min-width:92px;⏎
          height:30px; white-space:nowrap; overflow:hidden; text-overflow:ellipsis;⏎
          max-width:220px; }⏎
  th { color:var(--dim); background:#081209; font-weight:400; font-size:11px;⏎
       position:sticky; top:0; }⏎
  td.rowh { color:var(--dim); background:#081209; text-align:center; min-width:34px;⏎
            font-size:11px; }⏎
  td.n { text-align:right; color:var(--num); }⏎
  td.t { color:var(--txt); }⏎
  td.e { color:var(--err); }⏎
  td.p { color:var(--dim); font-style:italic; animation:breathe 1.6s infinite; }⏎
  td.f { text-shadow:0 0 10px rgba(53,255,157,.35); }⏎
  @keyframes breathe { 50% { opacity:.35; } }⏎
  td.sel { outline:2px solid var(--hot); outline-offset:-2px; background:var(--sel); }⏎
  td input { width:100%; background:#04120a; color:var(--hot); border:0; outline:0;⏎
             font:inherit; padding:0; }⏎
  footer { display:flex; gap:18px; font-size:12px; color:var(--dim); }⏎
  #status { color:var(--ink); flex:1; }⏎
  #status.err { color:var(--err); }⏎
  #hint { text-align:right; }⏎
  @media (max-width:900px){ #hint { display:none; } }⏎
</style>⏎
</head>⏎
<body>⏎
<canvas id="rain"></canvas>⏎
<main>⏎
  <header>⏎
    <h1>THE ARCHITECT</h1>⏎
    <p class="sub">a live spreadsheet, served entirely by MLang — the grid computes itself</p>⏎
    <div class="actions">⏎
      <button id="refresh" title="fetch live data (Ctrl+R)">&#8635; live data<span id="livecount"></span></button>⏎
      <button id="save" title="save as TSV (Ctrl+S)">&#8681; save</button>⏎
    </div>⏎
  </header>⏎
  <div id="fbar">⏎
    <span id="cellname">A1</span>⏎
    <input id="formula" spellcheck="false" autocomplete="off"⏎
           placeholder="cell contents — start with = for a formula">⏎
  </div>⏎
  <div id="gridwrap"><table id="grid"></table></div>⏎
  <footer>⏎
    <span id="status">jacking in&#8230;</span>⏎
    <span id="hint">=B3*C3 &#183; =SUM(B3:B5) &#183; =IF(D8&gt;100,"yes","no") &#183; =FX(EUR,USD) &#183; =BTC(USD) &#183; =WX(-33.9,151.2) &#183; =GET("url","path.to.field")</span>⏎
  </footer>⏎
</main>⏎
<script>⏎
'use strict';⏎
let S = null, sel = {r:2, c:1}, editing = false;⏎
const $ = id => document.getElementById(id);⏎
const grid = $('grid'), fbar = $('formula');⏎
const colName = c => 'ABCDEFGH'[c];⏎
const status = (m, bad) => { $('status').textContent = m; $('status').className = bad ? 'err' : ''; };⏎
⏎
async function api(path, body) {⏎
  const opts = body === undefined ? {} : {method:'POST', body:JSON.stringify(body)};⏎
  try {⏎
    const resp = await fetch(path, opts);⏎
    const text = await resp.text();⏎
    if (!resp.ok) { status('the grid answered ' + resp.status + ': ' + text, true); return; }⏎
    S = JSON.parse(text);⏎
    render();⏎
  } catch (e) { status('cannot reach the grid — is the server still up?', true); }⏎
}⏎
⏎
function render() {⏎
  if (!S) return;⏎
  let h = '<tr><th></th>';⏎
  for (let c = 0; c < S.cols; c++) h += '<th>' + colName(c) + '</th>';⏎
  h += '</tr>';⏎
  for (let r = 0; r < S.rows; r++) {⏎
    h += '<tr><td class="rowh">' + (r+1) + '</td>';⏎
    for (let c = 0; c < S.cols; c++) {⏎
      const kind = S.kind[r][c], raw = S.cells[r][c];⏎
      const cls = [kind, raw[0] === '=' && kind === 'n' ? 'f' : '',⏎
                   r === sel.r && c === sel.c ? 'sel' : ''].join(' ').trim();⏎
      const disp = S.disp[r][c].replace(/&/g,'&amp;').replace(/</g,'&lt;');⏎
      h += '<td data-r="' + r + '" data-c="' + c + '" class="' + cls + '">' + disp + '</td>';⏎
    }⏎
    h += '</tr>';⏎
  }⏎
  grid.innerHTML = h;⏎
  const live = S.live > 0 ? ' (' + S.live + ')' : '';⏎
  $('livecount').textContent = live;⏎
  document.title = 'THE ARCHITECT — ' + (S.name || 'zion ledger');⏎
  if (!editing) { fbar.value = S.cells[sel.r][sel.c]; $('cellname').textContent = colName(sel.c) + (sel.r+1); }⏎
  status(S.status || 'resident');⏎
}⏎
⏎
function cellAt(r, c) { return grid.querySelector('td[data-r="' + r + '"][data-c="' + c + '"]'); }⏎
⏎
function select(r, c) {⏎
  if (!S) return;⏎
  sel = { r: Math.max(0, Math.min(S.rows-1, r)), c: Math.max(0, Math.min(S.cols-1, c)) };⏎
  render();⏎
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
    else if (e.key === 'Escape'){ editing = false; render(); }⏎
    e.stopPropagation();⏎
  });⏎
  input.addEventListener('blur', () => { if (editing) commit(input.value, 0, 0); });⏎
}⏎
⏎
function commit(text, dr, dc) {⏎
  if (!editing) return;⏎
  editing = false;⏎
  const r = sel.r, c = sel.c;⏎
  sel = { r: Math.min(S.rows-1, r+dr), c: Math.min(S.cols-1, c+dc) };⏎
  api('/api/cell', { r:r, c:c, t:text });⏎
}⏎
⏎
grid.addEventListener('mousedown', e => {⏎
  const td = e.target.closest('td[data-r]');⏎
  if (!td) return;⏎
  const r = +td.dataset.r, c = +td.dataset.c;⏎
  if (r === sel.r && c === sel.c && !editing) { edit(); e.preventDefault(); return; }⏎
  if (editing) return;⏎
  select(r, c);⏎
});⏎
grid.addEventListener('dblclick', e => { if (!editing) edit(); });⏎
⏎
document.addEventListener('keydown', e => {⏎
  if (editing || !S) return;⏎
  if (e.ctrlKey || e.metaKey) {⏎
    if (e.key === 's') { $('save').click(); e.preventDefault(); }⏎
    if (e.key === 'r') { $('refresh').click(); e.preventDefault(); }⏎
    return;⏎
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
  if (e.key === 'Enter')  { editing = false; api('/api/cell', { r:sel.r, c:sel.c, t:fbar.value }); fbar.blur(); }⏎
  if (e.key === 'Escape') { editing = false; render(); fbar.blur(); }⏎
  e.stopPropagation();⏎
});⏎
fbar.addEventListener('blur', () => { editing = false; });⏎
⏎
$('refresh').addEventListener('click', async () => {⏎
  $('refresh').disabled = true;⏎
  status('reaching the outside world…');⏎
  await api('/api/refresh', {});⏎
  $('refresh').disabled = false;⏎
});⏎
$('save').addEventListener('click', () => api('/api/save', {}));⏎
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
api('/api/sheet');⏎
</script>⏎
</body>⏎
</html>⏎»≔Ω
⇊
1⇒g[g][⎆∂∅=[⌫0⇒g∅↥κ][↥κ]?]⟳
1⇒g B C w#0>[s« · press ↻ for live data»⧺⇒s][]?[g][↧κ∂∅=[⌫0⇒g 3[∅↥φ]⍣∅↥β][⇒r[r U↥β][⍕«✗ »⇅⧺∂⇒s⟨r⊃ 500«text/plain»⟩⇅⟨⇅⟩⧺↥β]⍥]?]⟳
[↧β∂∅≠][⍅]⟳⌫
[↧φ∂∅≠][∂[⍆⒥][⟨⇅«✗fetch»⇅⟩]⍥⟨⥀⥀⟩↥ρ]⟳⌫
[↧φ∂∅≠][∂[⍆⒥][⟨⇅«✗fetch»⇅⟩]⍥⟨⥀⥀⟩↥ρ]⟳⌫
[↧φ∂∅≠][∂[⍆⒥][⟨⇅«✗fetch»⇅⟩]⍥⟨⥀⥀⟩↥ρ]⟳⌫
