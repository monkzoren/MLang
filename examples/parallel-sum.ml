※ Sum 1..1000 across four worker strands plus a reducer.
※ Each worker uses its own strand id ⍳ to pick its chunk of 250,
※ sends its partial sum down channel σ; strand 4 reduces.
0 250⍸[⍳250×+1++]∀↥σ
0 250⍸[⍳250×+1++]∀↥σ
0 250⍸[⍳250×+1++]∀↥σ
0 250⍸[⍳250×+1++]∀↥σ
↧σ↧σ↧σ↧σ+++⍞
