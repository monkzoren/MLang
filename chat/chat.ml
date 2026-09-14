※ chat.ml — a chatbot that is a grid.
※
※ Every reply is computed by this program. There is no model in the hot
※ path: a turn is a substring match over R, the rule table, and R is a
※ definition — so the loom can rebind it while the grid is serving, and
※ the bot gains a reflex without dropping a connection or a conversation.
※
※   GET  /       the chat page
※   POST /say    a message in, a reply out
※
※ The rule table. Each entry is ⟨pattern reply⟩; the first pattern that
※ occurs anywhere in the lowercased message wins. Everything the bot
※ learns is a new entry here, woven in by the loom (see serve.py).
⟨
 ⟨«hello» «Hello. I am a grid — every reply you get is computed by an MLang program.»⟩
 ⟨«hi» «Hello there.»⟩
 ⟨«who are you» «A chatbot whose whole runtime is one MLang grid. No model answers you.»⟩
 ⟨«how do you learn» «Teach me: POST /teach with a pattern and a reply. I am re-woven while I run.»⟩
 ⟨«help» «Say hello, ask who I am, ask how I learn, or teach me something new.»⟩
 ⟨«bye» «Goodbye.»⟩
⟩≔R
«I do not know that one yet. You can teach me it.»≔D          ※ the fallback
[⇩⇒s R[⇒p s p0@∈]⌿⇒m m#0>[m0@1@][D]?]≔A                      ※ message → reply
«<!doctype html><meta charset=utf-8><title>grid</title><meta name=viewport content='width=device-width,initial-scale=1'><style>body{font:16px/1.6 system-ui,sans-serif;max-width:42rem;margin:0 auto;padding:1.5rem 1rem;background:#0b0c0e;color:#e6e6e6}h1{font-size:1.1rem;letter-spacing:.3em;text-transform:uppercase;color:#7fd1b9;margin:0 0 1rem}#log{min-height:40vh}p{margin:.4rem 0}.u{color:#7fd1b9}form{display:flex;gap:.5rem;flex-wrap:wrap}input{flex:1 1 8rem;min-width:0;padding:.6rem;background:#16181c;border:1px solid #2a2d33;color:inherit;border-radius:6px;font:inherit}button{padding:.6rem 1rem;background:#2a2d33;border:0;color:inherit;border-radius:6px;font:inherit;cursor:pointer}details{margin-top:2rem;border-top:1px solid #2a2d33;padding-top:1rem}summary{cursor:pointer;color:#8a8f98}#o{color:#8a8f98;white-space:pre-wrap;font-size:.85rem}</style><h1>grid</h1><div id=log></div><form id=f><input id=m autocomplete=off placeholder='say something' autofocus><button>send</button></form><details><summary>teach me something</summary><form id=t><input id=p autocomplete=off placeholder='when I hear...'><input id=r autocomplete=off placeholder='...say this'><input id=k type=password autocomplete=off placeholder=token><button>teach</button></form><pre id=o></pre></details><script>const $=i=>document.getElementById(i),K='grid-token';$('k').value=localStorage.getItem(K)||'';function add(c,t){const p=document.createElement('p');p.className=c;p.textContent=(c=='u'?'you: ':'grid: ')+t;$('log').appendChild(p);scrollTo(0,document.body.scrollHeight)}$('f').addEventListener('submit',async e=>{e.preventDefault();const t=$('m').value.trim();if(!t)return;$('m').value='';add('u',t);const r=await fetch('/say',{method:'POST',body:t});add('b',await r.text())});$('t').addEventListener('submit',async e=>{e.preventDefault();localStorage.setItem(K,$('k').value);const r=await fetch('/teach',{method:'POST',headers:{'X-Teach-Token':$('k').value},body:JSON.stringify({pattern:$('p').value,reply:$('r').value})});$('o').textContent=await r.text();if(r.ok){$('p').value='';$('r').value=''}});</script>»≔H
⇊
1⇒g[g][⎆∂∅=[⌫0⇒g][⇒r r2@«/say»=⇒k k[r3@A][H]?⇒a k[«text/plain»][«text/html»]?⇒t ⟨r0@ 200 t a⟩⍅]?]⟳
