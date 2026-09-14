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
«<!doctype html><meta charset=utf-8><title>grid</title><style>body{font:16px/1.5 system-ui;max-width:42rem;margin:0 auto;padding:2rem 1rem;background:#0b0c0e;color:#e6e6e6}#log{min-height:50vh}p{margin:.5rem 0}.u{color:#7fd1b9}.b{color:#e6e6e6}form{display:flex;gap:.5rem}input{flex:1;padding:.6rem;background:#16181c;border:1px solid #2a2d33;color:inherit;border-radius:6px}button{padding:.6rem 1rem;background:#2a2d33;border:0;color:inherit;border-radius:6px}</style><h1>grid</h1><div id=log></div><form onsubmit=\"event.preventDefault();send()\"><input id=m autofocus autocomplete=off placeholder=\"say something\"><button>send</button></form><script>async function send(){const i=document.getElementById(\"m\"),t=i.value.trim();if(!t)return;i.value=\"\";add(\"u\",t);const r=await fetch(\"/say\",{method:\"POST\",body:t});add(\"b\",await r.text())}function add(c,t){const p=document.createElement(\"p\");p.className=c;p.textContent=(c==\"u\"?\"you: \":\"grid: \")+t;log.appendChild(p);scrollTo(0,document.body.scrollHeight)}</script>»≔H
⇊
1⇒g[g][⎆∂∅=[⌫0⇒g][⇒r r2@«/say»=⇒k k[r3@A][H]?⇒a k[«text/plain»][«text/html»]?⇒t ⟨r0@ 200 t a⟩⍅]?]⟳
