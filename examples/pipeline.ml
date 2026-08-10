※ A three-strand pipeline in 25 glyphs, using the stream combinators:
※   ⇈α  pour a list into channel α (then the ∅ end-marker)
※   ⇉αβ pump: transform each value from α, send to β, forward ∅
※   ⇟β  drain channel β into a list
※ strand 0 emits the squares of 1..9, strand 1 doubles them,
※ strand 2 prints them. All three run concurrently.
9⍸[1+∂×]∵⇈α
[2×]⇉αβ
⇟β[⍞]∀
