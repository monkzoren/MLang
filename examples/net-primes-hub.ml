※ The hub half of a distributed prime finder — run with:
※   mlang hub --workers 2 examples/net-primes-hub.ml [limit] [chunk]
※ The program is an ordinary two-stage pipeline; `mlang hub` bridges
※ its channels so the pump between them runs on other machines:
※ every ⟨lo hi⟩ range poured into α goes over the wire to a joined
※ worker (examples/net-primes-worker.ml), and each worker's answer
※ comes back on β. Both reductions below are order-independent, so
※ the output is byte-identical however many workers share the run —
※ and whatever the network timing was.
⌂#0=[50000][⌂0@⍎]?≔l
⌂#2<[2000][⌂1@⍎]?≔c
⇊
l c÷⌈⍸[⇒i⟨i c× i1+c× l⊓⟩]∵⇈α    ※ carve 0..l into ⟨lo hi⟩ ranges, pour
⇟β⇒r«π(<»⊸l⊸«) = »⊸r[0@]∵∑⍞«largest: »⊸r[1@]∵0[⊔]⍀⍞
