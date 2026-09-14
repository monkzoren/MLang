※ chat.ml — a chatbot that is a grid, and nothing else.
※
※ Every reply is computed here. So is every lesson: ⟐ hands this program
※ its own source and ⟡ weaves the changed one back in (SPEC §4.7), so the
※ bot rewrites itself while it is answering. No second process takes part.
※
※   mlang serve chat.ml 8080 TOKEN /data/chat.ml
※      ⌂0@  the token POST /teach must carry
※      ⌂1@  where to save the program it becomes, so a restart remembers
※
※   GET  /        the chat page
※   POST /say     a message in, a reply out
※   POST /teach   token|pattern|reply
※
※ Bind the public interface with MLANG_HOST=0.0.0.0. The loom's HTTP
※ routes stay shut there unless MLANG_LOOM=1 — which is wanted: the bot
※ re-weaves itself from the inside, and nobody outside can.
171⍘≔L 187⍘≔Y                                   ※ « and », which no literal can hold
«⟩≔»«R»⧺≔M                                      ※ where the table ends — built, never written
⟨
 ⟨«hello» «Hello. I am a grid. Every reply you get is computed by an MLang program.»⟩
 ⟨«hi» «Hello there.»⟩
 ⟨«who are you» «A chatbot that is one MLang grid. No model answers you, and none rewrites me.»⟩
 ⟨«how do you learn» «Teach me: POST /teach with token|pattern|reply. I re-weave myself, in flight.»⟩
 ⟨«help» «Say hello, ask who I am, ask how I learn, or teach me something new.»⟩
 ⟨«bye» «Goodbye.»⟩
 ⟨«w00r00000» «agent 0 rule 0»⟩
 ⟨«w00r00001» «agent 0 rule 1»⟩
 ⟨«w02r00000» «agent 2 rule 0»⟩
 ⟨«w00r00002» «agent 0 rule 2»⟩
 ⟨«w00r00003» «agent 0 rule 3»⟩
 ⟨«w03r00000» «agent 3 rule 0»⟩
 ⟨«w02r00001» «agent 2 rule 1»⟩
 ⟨«w01r00000» «agent 1 rule 0»⟩
 ⟨«w03r00001» «agent 3 rule 1»⟩
 ⟨«w03r00002» «agent 3 rule 2»⟩
 ⟨«w01r00001» «agent 1 rule 1»⟩
 ⟨«w03r00003» «agent 3 rule 3»⟩
 ⟨«w02r00002» «agent 2 rule 2»⟩
 ⟨«w01r00002» «agent 1 rule 2»⟩
 ⟨«w01r00003» «agent 1 rule 3»⟩
 ⟨«w03r00004» «agent 3 rule 4»⟩
 ⟨«w03r00005» «agent 3 rule 5»⟩
 ⟨«w00r00004» «agent 0 rule 4»⟩
 ⟨«w01r00004» «agent 1 rule 4»⟩
 ⟨«w01r00005» «agent 1 rule 5»⟩
 ⟨«w01r00006» «agent 1 rule 6»⟩
 ⟨«w03r00006» «agent 3 rule 6»⟩
 ⟨«w02r00003» «agent 2 rule 3»⟩
 ⟨«w02r00004» «agent 2 rule 4»⟩
 ⟨«w01r00007» «agent 1 rule 7»⟩
 ⟨«w01r00008» «agent 1 rule 8»⟩
 ⟨«w00r00005» «agent 0 rule 5»⟩
 ⟨«w02r00005» «agent 2 rule 5»⟩
 ⟨«w01r00009» «agent 1 rule 9»⟩
 ⟨«w01r00010» «agent 1 rule 10»⟩
 ⟨«w00r00006» «agent 0 rule 6»⟩
 ⟨«w03r00007» «agent 3 rule 7»⟩
 ⟨«w00r00007» «agent 0 rule 7»⟩
 ⟨«w03r00008» «agent 3 rule 8»⟩
 ⟨«w02r00006» «agent 2 rule 6»⟩
 ⟨«w01r00011» «agent 1 rule 11»⟩
 ⟨«w00r00008» «agent 0 rule 8»⟩
 ⟨«w01r00012» «agent 1 rule 12»⟩
 ⟨«w00r00009» «agent 0 rule 9»⟩
 ⟨«w03r00009» «agent 3 rule 9»⟩
 ⟨«w00r00010» «agent 0 rule 10»⟩
 ⟨«w00r00011» «agent 0 rule 11»⟩
 ⟨«w03r00010» «agent 3 rule 10»⟩
 ⟨«w02r00007» «agent 2 rule 7»⟩
 ⟨«w01r00013» «agent 1 rule 13»⟩
 ⟨«w02r00008» «agent 2 rule 8»⟩
 ⟨«w03r00011» «agent 3 rule 11»⟩
 ⟨«w01r00014» «agent 1 rule 14»⟩
 ⟨«w01r00015» «agent 1 rule 15»⟩
 ⟨«w03r00012» «agent 3 rule 12»⟩
 ⟨«w03r00013» «agent 3 rule 13»⟩
 ⟨«w02r00009» «agent 2 rule 9»⟩
 ⟨«w02r00010» «agent 2 rule 10»⟩
 ⟨«w00r00012» «agent 0 rule 12»⟩
 ⟨«w03r00014» «agent 3 rule 14»⟩
 ⟨«w01r00016» «agent 1 rule 16»⟩
 ⟨«w02r00011» «agent 2 rule 11»⟩
 ⟨«w00r00013» «agent 0 rule 13»⟩
 ⟨«w00r00014» «agent 0 rule 14»⟩
 ⟨«w02r00012» «agent 2 rule 12»⟩
 ⟨«w00r00015» «agent 0 rule 15»⟩
 ⟨«w00r00016» «agent 0 rule 16»⟩
 ⟨«w02r00013» «agent 2 rule 13»⟩
 ⟨«w02r00014» «agent 2 rule 14»⟩
 ⟨«w02r00015» «agent 2 rule 15»⟩
 ⟨«w00r00017» «agent 0 rule 17»⟩
 ⟨«w02r00016» «agent 2 rule 16»⟩
 ⟨«w02r00017» «agent 2 rule 17»⟩
 ⟨«w02r00018» «agent 2 rule 18»⟩
 ⟨«w02r00019» «agent 2 rule 19»⟩
 ⟨«w02r00020» «agent 2 rule 20»⟩
 ⟨«w02r00021» «agent 2 rule 21»⟩
 ⟨«w01r00017» «agent 1 rule 17»⟩
 ⟨«w03r00015» «agent 3 rule 15»⟩
 ⟨«w03r00016» «agent 3 rule 16»⟩
 ⟨«w02r00022» «agent 2 rule 22»⟩
 ⟨«w02r00023» «agent 2 rule 23»⟩
 ⟨«w02r00024» «agent 2 rule 24»⟩
 ⟨«w02r00025» «agent 2 rule 25»⟩
 ⟨«w02r00026» «agent 2 rule 26»⟩
 ⟨«w02r00027» «agent 2 rule 27»⟩
 ⟨«w02r00028» «agent 2 rule 28»⟩
 ⟨«w03r00017» «agent 3 rule 17»⟩
 ⟨«w03r00018» «agent 3 rule 18»⟩
 ⟨«w03r00019» «agent 3 rule 19»⟩
 ⟨«w03r00020» «agent 3 rule 20»⟩
 ⟨«w03r00021» «agent 3 rule 21»⟩
 ⟨«w03r00022» «agent 3 rule 22»⟩
 ⟨«w03r00023» «agent 3 rule 23»⟩
 ⟨«w03r00024» «agent 3 rule 24»⟩
 ⟨«w00r00018» «agent 0 rule 18»⟩
 ⟨«w03r00025» «agent 3 rule 25»⟩
 ⟨«w00r00019» «agent 0 rule 19»⟩
 ⟨«w02r00029» «agent 2 rule 29»⟩
 ⟨«w00r00020» «agent 0 rule 20»⟩
 ⟨«w03r00026» «agent 3 rule 26»⟩
 ⟨«w02r00030» «agent 2 rule 30»⟩
 ⟨«w00r00021» «agent 0 rule 21»⟩
 ⟨«w00r00022» «agent 0 rule 22»⟩
 ⟨«w00r00023» «agent 0 rule 23»⟩
 ⟨«w01r00018» «agent 1 rule 18»⟩
 ⟨«w03r00027» «agent 3 rule 27»⟩
 ⟨«w00r00024» «agent 0 rule 24»⟩
 ⟨«w00r00025» «agent 0 rule 25»⟩
 ⟨«w02r00031» «agent 2 rule 31»⟩
 ⟨«w02r00032» «agent 2 rule 32»⟩
 ⟨«w02r00033» «agent 2 rule 33»⟩
 ⟨«w01r00019» «agent 1 rule 19»⟩
 ⟨«w02r00034» «agent 2 rule 34»⟩
 ⟨«w00r00026» «agent 0 rule 26»⟩
 ⟨«w02r00035» «agent 2 rule 35»⟩
 ⟨«w02r00036» «agent 2 rule 36»⟩
 ⟨«w03r00028» «agent 3 rule 28»⟩
 ⟨«w00r00027» «agent 0 rule 27»⟩
 ⟨«w00r00028» «agent 0 rule 28»⟩
 ⟨«w00r00029» «agent 0 rule 29»⟩
 ⟨«w01r00020» «agent 1 rule 20»⟩
 ⟨«w00r00030» «agent 0 rule 30»⟩
 ⟨«w02r00037» «agent 2 rule 37»⟩
 ⟨«w00r00031» «agent 0 rule 31»⟩
 ⟨«w00r00032» «agent 0 rule 32»⟩
 ⟨«w00r00033» «agent 0 rule 33»⟩
 ⟨«w00r00034» «agent 0 rule 34»⟩
 ⟨«w00r00035» «agent 0 rule 35»⟩
 ⟨«w02r00038» «agent 2 rule 38»⟩
 ⟨«w00r00036» «agent 0 rule 36»⟩
 ⟨«w03r00029» «agent 3 rule 29»⟩
 ⟨«w03r00030» «agent 3 rule 30»⟩
 ⟨«w03r00031» «agent 3 rule 31»⟩
 ⟨«w03r00032» «agent 3 rule 32»⟩
 ⟨«w00r00037» «agent 0 rule 37»⟩
 ⟨«w01r00021» «agent 1 rule 21»⟩
 ⟨«w01r00022» «agent 1 rule 22»⟩
 ⟨«w02r00039» «agent 2 rule 39»⟩
 ⟨«w01r00023» «agent 1 rule 23»⟩
 ⟨«w02r00040» «agent 2 rule 40»⟩
 ⟨«w03r00033» «agent 3 rule 33»⟩
 ⟨«w03r00034» «agent 3 rule 34»⟩
 ⟨«w03r00035» «agent 3 rule 35»⟩
 ⟨«w02r00041» «agent 2 rule 41»⟩
 ⟨«w00r00038» «agent 0 rule 38»⟩
 ⟨«w00r00039» «agent 0 rule 39»⟩
 ⟨«w00r00040» «agent 0 rule 40»⟩
 ⟨«w01r00024» «agent 1 rule 24»⟩
 ⟨«w03r00036» «agent 3 rule 36»⟩
 ⟨«w02r00042» «agent 2 rule 42»⟩
 ⟨«w02r00043» «agent 2 rule 43»⟩
 ⟨«w02r00044» «agent 2 rule 44»⟩
 ⟨«w01r00025» «agent 1 rule 25»⟩
 ⟨«w01r00026» «agent 1 rule 26»⟩
 ⟨«w02r00045» «agent 2 rule 45»⟩
 ⟨«w00r00041» «agent 0 rule 41»⟩
 ⟨«w00r00042» «agent 0 rule 42»⟩
 ⟨«w00r00043» «agent 0 rule 43»⟩
 ⟨«w00r00044» «agent 0 rule 44»⟩
 ⟨«w01r00027» «agent 1 rule 27»⟩
 ⟨«w01r00028» «agent 1 rule 28»⟩
 ⟨«w01r00029» «agent 1 rule 29»⟩
 ⟨«w01r00030» «agent 1 rule 30»⟩
 ⟨«w02r00046» «agent 2 rule 46»⟩
 ⟨«w02r00047» «agent 2 rule 47»⟩
 ⟨«w02r00048» «agent 2 rule 48»⟩
 ⟨«w02r00049» «agent 2 rule 49»⟩
 ⟨«w01r00031» «agent 1 rule 31»⟩
 ⟨«w01r00032» «agent 1 rule 32»⟩
 ⟨«w00r00045» «agent 0 rule 45»⟩
 ⟨«w00r00046» «agent 0 rule 46»⟩
 ⟨«w00r00047» «agent 0 rule 47»⟩
 ⟨«w00r00048» «agent 0 rule 48»⟩
 ⟨«w01r00033» «agent 1 rule 33»⟩
 ⟨«w02r00050» «agent 2 rule 50»⟩
 ⟨«w02r00051» «agent 2 rule 51»⟩
 ⟨«w03r00037» «agent 3 rule 37»⟩
 ⟨«w01r00034» «agent 1 rule 34»⟩
 ⟨«w01r00035» «agent 1 rule 35»⟩
 ⟨«w03r00038» «agent 3 rule 38»⟩
 ⟨«w03r00039» «agent 3 rule 39»⟩
 ⟨«w00r00049» «agent 0 rule 49»⟩
 ⟨«w02r00052» «agent 2 rule 52»⟩
 ⟨«w03r00040» «agent 3 rule 40»⟩
 ⟨«w03r00041» «agent 3 rule 41»⟩
 ⟨«w03r00042» «agent 3 rule 42»⟩
 ⟨«w03r00043» «agent 3 rule 43»⟩
 ⟨«w01r00036» «agent 1 rule 36»⟩
 ⟨«w01r00037» «agent 1 rule 37»⟩
 ⟨«w01r00038» «agent 1 rule 38»⟩
 ⟨«w01r00039» «agent 1 rule 39»⟩
 ⟨«w00r00050» «agent 0 rule 50»⟩
 ⟨«w03r00044» «agent 3 rule 44»⟩
 ⟨«w03r00045» «agent 3 rule 45»⟩
 ⟨«w03r00046» «agent 3 rule 46»⟩
 ⟨«w03r00047» «agent 3 rule 47»⟩
 ⟨«w03r00048» «agent 3 rule 48»⟩
 ⟨«w03r00049» «agent 3 rule 49»⟩
 ⟨«w03r00050» «agent 3 rule 50»⟩
 ⟨«w01r00040» «agent 1 rule 40»⟩
 ⟨«w01r00041» «agent 1 rule 41»⟩
 ⟨«w02r00053» «agent 2 rule 53»⟩
 ⟨«w02r00054» «agent 2 rule 54»⟩
 ⟨«w02r00055» «agent 2 rule 55»⟩
 ⟨«w03r00051» «agent 3 rule 51»⟩
 ⟨«w01r00042» «agent 1 rule 42»⟩
 ⟨«w00r00051» «agent 0 rule 51»⟩
⟩≔R
«I do not know that one yet. You can teach me it.»≔D
[⇩⇒s R[⇒p s p0@∈]⌿⇒m m#0>[m0@1@][D]?]≔A          ※ message → reply
[«|»⊆⇒f ⌂#2<[«teaching is not configured here.⏎»][
  ⌂0@#0=[«teaching is not configured here.⏎»][   ※ an unset TEACH_TOKEN is not a blank one

  f#3≠[«expected token|pattern|reply⏎»][
   f0@⌂0@≠[«refused: bad token⏎»][
    f1@⇩⇒p f2@⇒w
    p#0= w#0= ∨ p L∈ ∨ p Y∈ ∨ w L∈ ∨ w Y∈ ∨
    [«refused: a pattern and a reply are needed, without guillemets.⏎»][
      « ⟨»L⧺p⧺Y⧺« »⧺L⧺w⧺Y⧺«⟩⏎»⧺M⧺⇒c              ※ the new rule, as a line of me
      ⟐M⊆c⊇⇒n                                     ※ my source with it spliced in
      n⟡⇒v v0@200=[[n⌂1@⍈][⌫]⍥][]? v1@            ※ weave it, then remember it
    ]?]?]?]?]?]≔T
«<!doctype html><meta charset=utf-8><title>grid</title><meta name=viewport content='width=device-width,initial-scale=1'><style>body{font:16px/1.6 system-ui,sans-serif;max-width:42rem;margin:0 auto;padding:1.5rem 1rem;background:#0b0c0e;color:#e6e6e6}h1{font-size:1.1rem;letter-spacing:.3em;text-transform:uppercase;color:#7fd1b9;margin:0 0 1rem}#log{min-height:40vh}p{margin:.4rem 0}.u{color:#7fd1b9}form{display:flex;gap:.5rem;flex-wrap:wrap}input{flex:1 1 8rem;min-width:0;padding:.6rem;background:#16181c;border:1px solid #2a2d33;color:inherit;border-radius:6px;font:inherit}button{padding:.6rem 1rem;background:#2a2d33;border:0;color:inherit;border-radius:6px;font:inherit;cursor:pointer}details{margin-top:2rem;border-top:1px solid #2a2d33;padding-top:1rem}summary{cursor:pointer;color:#8a8f98}#o{color:#8a8f98;white-space:pre-wrap;font-size:.85rem}</style><h1>grid</h1><div id=log></div><form id=f><input id=m autocomplete=off placeholder='say something' autofocus><button>send</button></form><details><summary>teach me something</summary><form id=t><input id=p autocomplete=off placeholder='when I hear...'><input id=r autocomplete=off placeholder='...say this'><input id=k type=password autocomplete=off placeholder=token><button>teach</button></form><pre id=o></pre></details><script>const $=i=>document.getElementById(i),K='grid-token';$('k').value=localStorage.getItem(K)||'';function add(c,t){const p=document.createElement('p');p.className=c;p.textContent=(c=='u'?'you: ':'grid: ')+t;$('log').appendChild(p);scrollTo(0,document.body.scrollHeight)}$('f').addEventListener('submit',async e=>{e.preventDefault();const t=$('m').value.trim();if(!t)return;$('m').value='';add('u',t);const r=await fetch('/say',{method:'POST',body:t});add('b',await r.text())});$('t').addEventListener('submit',async e=>{e.preventDefault();localStorage.setItem(K,$('k').value);const b=[$('k').value,$('p').value,$('r').value].join('|');const r=await fetch('/teach',{method:'POST',body:b});const x=await r.text();$('o').textContent=x;if(x.indexOf('refused')<0&&x.indexOf('expected')<0){$('p').value='';$('r').value=''}});</script>»≔H
⇊
1⇒g[g][⎆∂∅=[⌫0⇒g][⇒r r2@⇒k k«/say»=[r3@A][k«/teach»=[r3@T][H]?]?⇒a k«/say»=k«/teach»=∨[«text/plain»][«text/html»]?⇒t ⟨r0@ 200 t a⟩⍅]?]⟳
