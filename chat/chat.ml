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
※ After every answer it compares what it is to what it last wrote down, and
※ writes itself to ⌂1@ if they differ — so a rule that arrived on /.loom from
※ another agent is remembered too, not only one it was told through /teach.
※ The local is read through ⍥ because a strand woven into a running grid
※ resumes inside its loop: its prelude never runs again, so it cannot assume
※ a local exists.
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
⟩≔R
«I do not know that yet. I am finding out — ask me again shortly.»≔D
[⇩⇒s R[⇒p s p0@∈]⌿⇒m m#0>[m0@1@][∅]?]≔A          ※ message → reply, ∅ when it does not know
[L⊆«»⊇Y⊆«»⊇«⏎»⊆« »⊇]≔Z                          ※ scrub: a rule is one line, and holds no guillemets
[⇒w⇒p « ⟨»L⧺p⧺Y⧺« »⧺L⧺w⧺Y⧺«⟩⏎»⧺M⧺⇒c ⟐M⊆c⊇⟡]≔W   ※ pattern reply → ⟨status report⟩
⟨«https://api.anthropic.com/v1/messages» «claude-opus-5» «2023-06-01»⟩≔E   ※ where to ask
«Answer in one short plain sentence. No preamble, no markdown, no newlines.»≔I
[⇒q ⌂#3<[∅][⟨E0@ ⟨⟨«x-api-key» ⌂2@⟩ ⟨«anthropic-version» E2@⟩ ⟨«content-type» «application/json»⟩⟩ ⟨⟨«model» E1@⟩ ⟨«max_tokens» 512⟩ ⟨«output_config» ⟨⟨«effort» «low»⟩⟩⟩ ⟨«system» I⟩ ⟨«messages» ⟨⟨⟨«role» «user»⟩ ⟨«content» q⟩⟩⟩⟩⟩⒮⟩⇒b [b⍄⒥⟨«content» 0 «text»⟩⒫][⌫∅]⍥]?]≔K   ※ question → an answer, or ∅
[«|»⊆⇒f ⌂#2<[«teaching is not configured here.⏎»][
  ⌂0@#0=[«teaching is not configured here.⏎»][   ※ an unset TEACH_TOKEN is not a blank one
   f#3≠[«expected token|pattern|reply⏎»][
    f0@⌂0@≠[«refused: bad token⏎»][
     f1@⇩⇒p f2@⇒w
     p#0= w#0= ∨ p L∈ ∨ p Y∈ ∨ w L∈ ∨ w Y∈ ∨
     [«refused: a pattern and a reply are needed, without guillemets.⏎»][p w W 1@]?
    ]?]?]?]?]≔T
«<!doctype html><meta charset=utf-8><title>grid</title><meta name=viewport content='width=device-width,initial-scale=1'><style>body{font:16px/1.6 system-ui,sans-serif;max-width:42rem;margin:0 auto;padding:1.5rem 1rem;background:#0b0c0e;color:#e6e6e6}h1{font-size:1.1rem;letter-spacing:.3em;text-transform:uppercase;color:#7fd1b9;margin:0 0 1rem}#log{min-height:40vh}p{margin:.4rem 0}.u{color:#7fd1b9}form{display:flex;gap:.5rem;flex-wrap:wrap}input{flex:1 1 8rem;min-width:0;padding:.6rem;background:#16181c;border:1px solid #2a2d33;color:inherit;border-radius:6px;font:inherit}button{padding:.6rem 1rem;background:#2a2d33;border:0;color:inherit;border-radius:6px;font:inherit;cursor:pointer}details{margin-top:2rem;border-top:1px solid #2a2d33;padding-top:1rem}summary{cursor:pointer;color:#8a8f98}#o{color:#8a8f98;white-space:pre-wrap;font-size:.85rem}</style><h1>grid</h1><div id=log></div><form id=f><input id=m autocomplete=off placeholder='say something' autofocus><button>send</button></form><details><summary>teach me something</summary><form id=t><input id=p autocomplete=off placeholder='when I hear...'><input id=r autocomplete=off placeholder='...say this'><input id=k type=password autocomplete=off placeholder=token><button>teach</button></form><pre id=o></pre></details><script>const $=i=>document.getElementById(i),K='grid-token';$('k').value=localStorage.getItem(K)||'';function add(c,t){const p=document.createElement('p');p.className=c;p.textContent=(c=='u'?'you: ':'grid: ')+t;$('log').appendChild(p);scrollTo(0,document.body.scrollHeight)}$('f').addEventListener('submit',async e=>{e.preventDefault();const t=$('m').value.trim();if(!t)return;$('m').value='';add('u',t);const r=await fetch('/say',{method:'POST',body:t});add('b',await r.text())});$('t').addEventListener('submit',async e=>{e.preventDefault();localStorage.setItem(K,$('k').value);const b=[$('k').value,$('p').value,$('r').value].join('|');const r=await fetch('/teach',{method:'POST',body:b});const x=await r.text();$('o').textContent=x;if(x.indexOf('refused')<0&&x.indexOf('expected')<0){$('p').value='';$('r').value=''}});</script>»≔H
⇊
1⇒g[g][⎆∂∅=[⌫ ∅↥λ 0⇒g][⇒r r2@⇒k k«/say»=[r3@⇒q q A⇒n n∅=[q↥λ D][n]?][k«/teach»=[r3@T][H]?]?⇒a k«/say»=k«/teach»=∨[«text/plain»][«text/html»]?⇒t ⟨r0@ 200 t a⟩⍅ ⌂#2≥[⟐∂[y][⌫«»]⍥≠[∂⌂1@⍈⇒y][⌫]?][]?]?]⟳
1⇒h[h][↧λ∂∅=[⌫0⇒h][⇒q q K⇒w w∅≠[q Z⇩⇒p p#0>[p w Z W⌫][]?][]?]?]⟳
