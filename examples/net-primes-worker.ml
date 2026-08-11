※ The worker half of a distributed prime finder — run on each machine:
※   mlang worker --connect HUB:7777 examples/net-primes-worker.ml
※ One pump, exactly as it would read in a single-process pipeline:
※ a ⟨lo hi⟩ range arrives on α, the primes in [lo,hi) are counted,
※ and ⟨count largest⟩ leaves on β. When the hub forwards its ∅ the
※ pump stops and the worker exits. Glitch mid-chunk and the hub
※ requeues the unanswered ranges on the surviving workers.
[∂2<[⌫0][∂√⌊1-⍸[2+⊚⇅%0=]⌿⇅⌫#0=]?]≔p    ※ p: trial-division primality
⇊
[∂0@⇒a 1@⇒b b a-⍸[a+]∵[p]⌿⇒r⟨r# r#0=[0][r⌷]?⟩]⇉αβ
