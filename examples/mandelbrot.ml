※ The Matrix computes the Mandelbrot set — in parallel.
※
※ Boot defines two globals:
※   P — pixel: cr ci → shade char. Escape-time iteration z ← z² + c
※       (z in locals z/w, c in locals r/i, count in n), max 30 rounds,
※       then indexes an 11-char palette by n÷3.
※   R — row: y → 70-char string, mapping x across [¯2.2, 1.0].
※
※ Four worker strands shard the 24 rows by strand id (row % 4 = ⍳),
※ each pouring finished rows into its own channel; the reducer prints
※ rows in order by cycling its receives. Fully deterministic.
[⇒i⇒r 0⇒z 0⇒w 0⇒n [z∂× w∂×+ 4≤ n 30<∧][z∂× w∂×- r+ 2 z× w× i+ ⇒w⇒z n 1+⇒n]⟳ « .:-=+*#%@█» n 3÷⌊ @]≔P
[⇒y 70⍸[3.2× 70÷ 2.2- y 2.4× 24÷ 1.2- P]∵«»⊇]≔R
⇊
24⍸[∂4%⍳=[R↥a][⌫]?]∀
24⍸[∂4%⍳=[R↥b][⌫]?]∀
24⍸[∂4%⍳=[R↥c][⌫]?]∀
24⍸[∂4%⍳=[R↥d][⌫]?]∀
6[↧a⍞↧b⍞↧c⍞↧d⍞]⍣
