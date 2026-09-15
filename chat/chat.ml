※ chat.ml — a chatbot that is a grid, and nothing else.
※
※ Every reply is computed here. So is every lesson: ⟐ hands this program
※ its own source and ⟡ weaves the changed one back in (SPEC §4.7), so the
※ bot rewrites itself while it is answering. No second process takes part.
※
※   mlang serve chat.ml 8080 TOKEN /data/chat.ml
※      ⌂0@  the token POST /teach must carry
※      ⌂1@  where to save the program it becomes, so a restart remembers
※      ⌂2@  the key the learner asks with; without it the learner is inert
※      ⌂3@ ⌂4@ ⌂5@ ⌂6@  model, url, dialect, version — each falling back to G
※                        when blank, so a deployment overrides one without
※                        having to know the other three
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
«⟩≔»«Q»⧺≔M                                      ※ where the learned table ends — built, never written
※ The first skill. A row answers one question; this answers every sum,
※ including the ones nobody has asked yet, and it costs no API call. It
※ keeps only the characters arithmetic is made of, so «what is 120+50»
※ is «120+50», and leaves ∅ — not mine — for anything it cannot read, so
※ the question goes on as if no rule had matched.
※ Locals are the calling strand's. The body strand keeps its request in r
※ and its answer in a, so nothing called from it may store there — a
※ definition that does answers the wrong request. C uses u v l f i j.
[⇒s s«»⊆[⇒c «0123456789.+-×*÷/» c∈]⌿«»⊇⇒u ∅⇒v
 ⟨«+» «-» «×» «*» «÷» «/»⟩[⇒l v∅=[u l⊆⇒f f#2=[[f0@⍎⇒i f1@⍎⇒j l«+»=[i j+][l«-»=[i j-][l«÷»=l«/»=∨[i j÷][i j×]?]?]?⍕⇒v][⌫]⍥][]?][]?]∀
 v]≔C
⟨
 ⟨«hello» «Hello. I am a grid. Every reply you get is computed by an MLang program.»⟩
 ⟨«hi» «Hello there.»⟩
 ⟨«who are you» «A chatbot that is one MLang grid. No model answers you, and none rewrites me.»⟩
 ⟨«how do you learn» «Teach me: POST /teach with token|pattern|reply. I re-weave myself, in flight.»⟩
 ⟨«help» «Say hello, ask who I am, ask how I learn, or teach me something new.»⟩
 ⟨«bye» «Goodbye.»⟩
 ⟨«+» [C]⟩ ⟨«-» [C]⟩ ⟨«×» [C]⟩ ⟨«*» [C]⟩ ⟨«÷» [C]⟩ ⟨«/» [C]⟩
⟩≔R
※ What it has learned since. The image ships this empty and never writes
※ inside it, and every lesson lands here and never in R — so the seed and
※ the memory own different lines, and a new image merges with an old volume
※ without the two ever contending for the same spot. (They did, once: the
※ seed's calculator and a volume's lessons both appended after «bye», and
※ diff3 rightly called it a conflict.)
⟨
⟩≔Q
«I do not know that yet. I am finding out — ask me again shortly.»≔D
※ With no key in ⌂2@ the learner cannot ask anyone, so D would be a lie
※ told forever. A bot that says what it is is worth more than one that
※ promises what it cannot do.
«I do not know that yet, and nobody has taught me. Teach me with /teach.»≔U
※ A reply is a string, said as it is — or a quotation, run with the message,
※ and what it leaves is the answer. ∅ from a quotation means «not mine»,
※ and the next matching rule is tried; ∅ from all of them means the grid
※ does not know. Matches are tried in table order, so the seed comes
※ before anything learned (R before Q), and a skill before a fact about the
※ same thing.
※ A skill that glitches is «not mine» too, not the end of the conversation.
[⇩⇒s R Q⧺[⇒p s p0@∈]⌿ ∅⇒m [⇒p m∅=[p1@ ∂⍙⇒e⌫ e«quot»=[[s⇅!][⌫∅]⍥][]?⇒m][]?]∀ m]≔A
[L⊆«»⊇Y⊆«»⊇«⏎»⊆« »⊇]≔Z                          ※ scrub: a rule is one line, and holds no guillemets
[⇒w⇒p « ⟨»L⧺p⧺Y⧺« »⧺L⧺w⧺Y⧺«⟩⏎»⧺M⧺⇒c ⟐M⊆c⊇⟡]≔W   ※ pattern reply → ⟨status report⟩
※ Where to ask, and in whose dialect: ⟨url model version dialect⟩. Two wire
※ formats are spoken. «openai» covers DeepSeek and everything else that
※ copied that shape; «anthropic» covers Claude, which wants its own header,
※ its own body and its own place to keep the answer. E is a definition, so
※ a running bot can be re-pointed at another provider through the loom
※ without a restart — the version field is unused by «openai».
⟨«https://api.deepseek.com/chat/completions» «deepseek-flash» «» «openai»⟩≔G
※ for Claude: ⟨«https://api.anthropic.com/v1/messages» «claude-opus-5» «2023-06-01» «anthropic»⟩
[⇒i⇒v ⌂#i>[⌂i@⇒t t#0>[t][v]?][v]?]≔O            ※ default index → the argument, if given and not blank
⟨G0@ 4 O  G1@ 3 O  G2@ 6 O  G3@ 5 O⟩≔E          ※ so the deployment can say, without a rebuild
※ The primer the model writes MLang against — `mlang ops`, dropped into the
※ image at build time. Absent (a replay test, a bare `mlang serve`) it is
※ empty and the model is on its own, which is the same thing as inert.
[«/app/primer.txt»⍇][⌫«»]⍥≔N
※ What the learner asks. Not «answer this» but «is this a fact or a skill?»
※ — a fact is one row; a skill is a quotation that computes for a whole
※ class of messages, and is installed only if it passes its own tests.
«You extend a chatbot written in MLang. A user asked something the bot could not answer. Decide whether the right response is a FACT (one specific answer to this one question) or a SKILL (a general ability that computes answers for a whole class of messages: arithmetic, unit conversion, string manipulation, date sums, and the like). Reply with JSON only. No prose, no code fence.⏎For a fact: {"kind":"fact","answer":"one short plain sentence"}⏎For a skill: {"kind":"skill","pattern":"...","code":"[...]","tests":[["message","exact expected reply"],["message","exact expected reply"],["message","exact expected reply"]]}⏎pattern: a short lowercase substring present in every message this skill should handle.⏎code: exactly one MLang quotation. It is run with the whole lowercased message as a string on the stack and must leave exactly one value: the reply as a string, or the nil value if this message is not for it. Operands come before the glyph. Names are one glyph. Use only the single letters c e f i l u v as local names and no other letter. Do not define globals, spawn strands, use channels, write loops that may not end, or read files or the network. Only operations in the reference below exist.⏎tests: three pairs; the skill is installed only if it passes every one, compared exactly.⏎⏎MLang reference:⏎»N⧺≔I
※ A truncated answer is never learned. A reasoning model spends the budget
※ thinking before it says anything, so a cap that looks generous can still
※ cut the sentence in half — and a half sentence welded into the program is
※ there for good. Both dialects say when they stopped early; believe them.
[⇒q ⌂#3<[∅][E3@«openai»=⇒o
 o[⟨⟨«Authorization» «Bearer »⌂2@⧺⟩ ⟨«content-type» «application/json»⟩⟩][⟨⟨«x-api-key» ⌂2@⟩ ⟨«anthropic-version» E2@⟩ ⟨«content-type» «application/json»⟩⟩]?⇒x
 o[⟨⟨«model» E1@⟩ ⟨«max_tokens» 8192⟩ ⟨«messages» ⟨⟨⟨«role» «system»⟩ ⟨«content» I⟩⟩ ⟨⟨«role» «user»⟩ ⟨«content» q⟩⟩⟩⟩⟩][⟨⟨«model» E1@⟩ ⟨«max_tokens» 8192⟩ ⟨«output_config» ⟨⟨«effort» «low»⟩⟩⟩ ⟨«system» I⟩ ⟨«messages» ⟨⟨⟨«role» «user»⟩ ⟨«content» q⟩⟩⟩⟩⟩]?⒮⇒d
 ⟨E0@ x d⟩⇒b [b⍄⒥⇒j
  [j o[⟨«choices» 0 «finish_reason»⟩][⟨«stop_reason»⟩]?⒫][⌫«»]⍥⇒z   ※ absent is not truncated
  z«length»= z«max_tokens»= ∨[∅][j o[⟨«choices» 0 «message» «content»⟩][⟨«content» 0 «text»⟩]?⒫]?
 ][⌫∅]⍥]?]≔K   ※ question → the model's reply text, or ∅
[⇒x x«{»⍷⇒i x⌽«}»⍷⇒k x i x#k-⊂]≔F              ※ the JSON object inside whatever wrapped it
※ Install a skill: weave a row whose reply is the quotation, run the model's
※ own tests against the live grid, and if any fails weave the old program
※ straight back. The loom makes that a transaction — a rejected skill leaves
※ nothing behind but a version in the log. Greek locals: a skill under test
※ can store into any Latin letter, and so can everything it calls.
[⇒Ω⇒γ⇒π ⟐⇒Θ « ⟨»L⧺π⧺Y⧺« »⧺γ⧺«⟩⏎»⧺M⧺⇒ν
 Θ M⊆ν⊇⟡0@200=[Ω[⇒φ φ0@ A φ1@=]⌿# Ω#= Ω#0> ∧[1][Θ⟡⌫ 0]?][0]?]≔S
※ What the model said → a fact learned, a skill installed, or nothing. Every
※ way the reply can be malformed ends in the ⍥ and teaches nothing.
[⇒ω⇒κ [ω F⒥⇒ι ι⟨«kind»⟩⒫«skill»=
  [ι⟨«pattern»⟩⒫ Z⇩⇒π π#0>[π ι⟨«code»⟩⒫ ι⟨«tests»⟩⒫ S⌫][]?]
  [ι⟨«answer»⟩⒫ Z⇒ω κ Z⇩⇒π π#0>[π ω W⌫][]?]?][⌫]⍥]≔J
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
1⇒g[g][⎆∂∅=[⌫ ∅↥λ 0⇒g][⇒r r2@⇒k k«/say»=[r3@⇒q q A⇒n n∅=[q↥λ ⌂#3<[U][D]?][n]?][k«/teach»=[r3@T][H]?]?⇒a k«/say»=k«/teach»=∨[«text/plain»][«text/html»]?⇒t ⟨r0@ 200 t a⟩⍅ ⌂#2≥[⟐∂[y][⌫«»]⍥≠[∂⌂1@⍈⇒y][⌫]?][]?]?]⟳
1⇒h[h][↧λ∂∅=[⌫0⇒h][⇒q q K⇒w w∅≠[q w J][]?]?]⟳
