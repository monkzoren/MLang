※ The Matrix computes the Mandelbrot set — in parallel, interactively.
※
※ Boot defines two globals; both read the viewport from strand-locals
※ set by the executing worker:
※   P — pixel: cr ci → shade char. Escape-time iteration z ← z² + c
※       (z in locals z/w, c in locals r/i, count in n), up to d rounds,
※       then indexes an 11-char palette by n×10÷d.
※   R — row: y → 70-char string, stepping x across the viewport
※       (origin in locals x/u, step in locals j/v).
※
※ Four worker strands shard the 24 rows by strand id (⍳): each blocks
※ on its job channel (e f g h), unpacks the job ⟨x0 dx y0 dy depth⟩,
※ renders rows ⍳ ⍳+4 … ⍳+20 into its own row channel (a b c d), and
※ loops — until a ∅ job ends the show.
※
※ Strand 4 is the navigator. It owns the viewport (center p q, width
※ m, depth d), broadcasts one job per frame, prints the 24 rows in
※ order by cycling its receives, then reads a command from stdin:
※   a d w s  pan left / right / up / down     z x  zoom in / out
※   r        reset the view                   q    quit (EOF quits too)
※ Anything else re-renders. Zooming in raises the iteration depth so
※ the boundary keeps its texture. Try:
※   printf 'z⏎w⏎q⏎' | mlang run examples/mandelbrot.ml
[⇒i⇒r 0⇒z 0⇒w 0⇒n [z∂× w∂×+ 4≤ n d<∧][z∂× w∂×- r+ 2 z× w× i+ ⇒w⇒z n 1+⇒n]⟳ « .:-=+*#%@█» n 10× d÷⌊ @]≔P
[⇒y 70⍸[j× x+ y v× u+ P]∵«»⊇]≔R
⇊
[↧e∂∅≠][⇒t t0@⇒x t1@⇒j t2@⇒u t3@⇒v t4@⇒d 6⍸[4×⍳+R↥a]∀]⟳⌫
[↧f∂∅≠][⇒t t0@⇒x t1@⇒j t2@⇒u t3@⇒v t4@⇒d 6⍸[4×⍳+R↥b]∀]⟳⌫
[↧g∂∅≠][⇒t t0@⇒x t1@⇒j t2@⇒u t3@⇒v t4@⇒d 6⍸[4×⍳+R↥c]∀]⟳⌫
[↧h∂∅≠][⇒t t0@⇒x t1@⇒j t2@⇒u t3@⇒v t4@⇒d 6⍸[4×⍳+R↥d]∀]⟳⌫
¯0.5⇒p 0⇒q 3⇒m 30⇒d 1⇒k
⋮[k][⟨p m 2÷- m 70÷ q m 0.375×- m 32÷ d⟩∂∂∂↥e↥f↥g↥h 6[↧a⍞↧b⍞↧c⍞↧d⍞]⍣
⋮«center »p⍕⧺« »⧺q⍕⧺«  width »⧺m⍕⧺«  depth »⧺d⍕⧺«  [a/d/w/s pan  z/x zoom  r reset  q quit] > »⧺⊸
⋮⌨∂∅=[⌫0⇒k][⇒t t«q»=[0⇒k][t«a»=[p m 4÷-⇒p][t«d»=[p m 4÷+⇒p][t«w»=[q m 0.1875×-⇒q][t«s»=[q m 0.1875×+⇒q][t«z»=[m 2÷⇒m d 20+⇒d][t«x»=[m 2×⇒m d 20- 30⊔⇒d][t«r»=[¯0.5⇒p 0⇒q 3⇒m 30⇒d][]?]?]?]?]?]?]?]?]?]⟳
⋮∅∂∂∂↥e↥f↥g↥h
