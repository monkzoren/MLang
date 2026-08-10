※ A three-strand pipeline over channels α and β.
※ strand 0: emits squares of 1..9        → α
※ strand 1: doubles everything from α    → β
※ strand 2: prints everything from β
※ ∅ is the end-of-stream sentinel each stage forwards.
9⍸[1+∂×↥α]∀∅↥α
[↧α∂∅≠][2×↥β]⟳⌫∅↥β
[↧β∂∅≠][⍞]⟳⌫
