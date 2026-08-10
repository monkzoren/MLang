※ The same three-strand pipeline as pipeline.ml, built by hand from
※ raw sends and receives — this is the protocol the stream
※ combinators (⇈ ⇉ ⇟) package up: send values, end with ∅,
※ receive until ∅, forward the ∅ so downstream stages stop too.
9⍸[1+∂×↥α]∀∅↥α
[↧α∂∅≠][2×↥β]⟳⌫∅↥β
[↧β∂∅≠][⍞]⟳⌫
