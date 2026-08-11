※ THE DIVE — the Matrix falls into the Mandelbrot set, on autopilot.
※
※ No keys, no navigation: the program flies itself. Each frame the
※ navigator scores a 4×4 grid of cells by boundary richness —
※ min(interior pixels, everything else), highest where the set's edge
※ writhes — and zooms 2× toward the best cell, deepening the
※ escape-time limit as it goes. Blank sky and solid interior score 0,
※ so the dive hugs the filaments. Every dive resets and flies a
※ different line (dive k prefers the k-th best cell), and ⌂ argv sets
※ the dive count: mlang run examples/mandelbrot-dive.ml 6
※
※ The picture is painted the Matrix way. Rows are not reassembled in
※ order: four worker strands drop finished rows — ⟨y colored plain⟩ —
※ into one shared channel, and the navigator paints each row at its
※ absolute screen position (ESC[y;1H) the moment it lands, one ⍞ per
※ row so every landing is flushed. Run it with --parallel and the
※ interleave is real: four OS threads racing rows onto the screen,
※ the same final image every time.
※
※ Boot globals: E escape, A the shade palette, C its green-lit form,
※ P pixel → palette index (viewport in worker locals x/j/u/v, depth
※ d), R row → index list, H the cell hunt (excludes cells in X).
27⍘≔E
« .:-=+*#%@█»≔A
⟨« » E«[2;32m.»⧺ E«[2;32m:»⧺ E«[2;32m-»⧺ E«[0;32m=»⧺ E«[0;32m+»⧺ E«[0;32m*»⧺ E«[1;32m#»⧺ E«[1;92m%»⧺ E«[1;92m@»⧺ E«[1;97m█»⧺⟩≔C
[⇒i⇒r 0⇒z 0⇒w 0⇒n [z∂× w∂×+ 4≤ n d<∧][z∂× w∂×- r+ 2 z× w× i+ ⇒w⇒z n 1+⇒n]⟳ n 10× d÷⌊]≔P
[⇒y 70⍸[j× x+ y v× u+ P]∵]≔R
[¯1⇒b ¯1⇒v 16⍸[⇒c X c∈¬[0⇒a 0⇒l 6⍸[c 4÷⌊ 6× + G⇅@ c 4% 18× ∂ 18+ ⊂ ⇒w a w«█»⊆# 1- +⇒a l w# +⇒l]∀ a l a- ⊓⇒s s v>[s⇒v c⇒b][]?][]?]∀ X⟨b⟩⧺⇒X]≔H
⇊
[↧e∂∅≠][⇒t t0@⇒x t1@⇒j t2@⇒u t3@⇒v t4@⇒d 6⍸[4×⍳+⇒y ⟨y y R∂[C⇅@]∵«»⊇ ⇅[A⇅@]∵«»⊇⟩↥o]∀]⟳⌫
[↧f∂∅≠][⇒t t0@⇒x t1@⇒j t2@⇒u t3@⇒v t4@⇒d 6⍸[4×⍳+⇒y ⟨y y R∂[C⇅@]∵«»⊇ ⇅[A⇅@]∵«»⊇⟩↥o]∀]⟳⌫
[↧g∂∅≠][⇒t t0@⇒x t1@⇒j t2@⇒u t3@⇒v t4@⇒d 6⍸[4×⍳+⇒y ⟨y y R∂[C⇅@]∵«»⊇ ⇅[A⇅@]∵«»⊇⟩↥o]∀]⟳⌫
[↧h∂∅≠][⇒t t0@⇒x t1@⇒j t2@⇒u t3@⇒v t4@⇒d 6⍸[4×⍳+⇒y ⟨y y R∂[C⇅@]∵«»⊇ ⇅[A⇅@]∵«»⊇⟩↥o]∀]⟳⌫
⌂#0>[[⌂⊃⍎][⌫1]⍥][1]?⇒D 0⇒k
⋮D[¯0.5⇒p 0⇒q 3⇒m 30⇒d E«[2J»⧺⊸
⋮12[⟨p m 2÷- m 70÷ q m 0.375×- m 32÷ d⟩∂∂∂↥e↥f↥g↥h 24⍸[⌫«»]∵⇒G
⋮24[↧o⇒t E«[»⧺ t0@1+⍕⧺ «;1H»⧺ t1@⧺ E«[0m»⧺⧺ E«[1;1H»⧺⧺⍞ G 0 t0@⊂ ⟨t2@⟩⧺ G t0@1+ 24⊂⧺⇒G]⍣
⋮E«[1;1H»⧺ E«[1;92mMLANG ⇊ THE DIVE »⧺⧺ E«[0;32m∙ dive »⧺⧺ k1+⍕⧺ « ∙ width »⧺ m⍕⧺ « ∙ depth »⧺ d⍕⧺ E«[0m»⧺⧺ E«[K»⧺⧺⍞
⋮⟨⟩⇒X k 4% 1+[H]⍣ v 1<[⟨⟩⇒X H][]?
⋮p m 2÷- b 4% 18× 9+ 69⊓ m 70÷ × +⇒p q m 0.375×- b 4÷⌊ 6× 3+ m 32÷ × +⇒q m 2÷⇒m d 20+⇒d]⍣
⋮k 1+⇒k]⍣
⋮∅∂∂∂↥e↥f↥g↥h E«[24;1H»⧺⊸ «»⍞ «wake up, Neo...»⍞
