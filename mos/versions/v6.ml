※ v6 — a change its author was wrong about
※
※ The axis preference is pinned to vertical: `p q>` becomes `0`, so the
※ machine always resolves the vertical distance first whatever the two
※ distances are. This was written as a *deliberately bad* candidate, to
※ check that the gate says no to things. The gate shipped it: 336/336
※ invariants clean and score 1100 → 1300, which is the shortest-path
※ score for a machine that does not model the slip tiles.
※
※ It is kept because being overruled by the rollout is the entire reason
※ to have one. Judgement proposes; selection decides.
⟨«N»«E»«S»«W»«G»«D»«X»⟩≔T                                 ※ the action alphabet
[« »⊆[⍎]∵]≔F                                              ※ tick → list of ints
[F⇒v v2@[v9@ v10@][v7@ v8@]?⇒z⇒u
 v0@u= v1@z= ∧[v2@[«D»][«G»]?][
  v0@u<[«E»][v0@u>[«W»][«»]?]?⇒h                          ※ the horizontal wish
  v1@z<[«S»][v1@z>[«N»][«»]?]?⇒w                          ※ the vertical wish
  v0@u-∂0<[±][]?⇒p v1@z-∂0<[±][]?⇒q                       ※ how far, each way
  0[⟨h w⟩][⟨w h⟩]?T 0 4⊂⧺                      ※ wishes first, then any escape
  [⇒d d#0> v«NESW»d⍷3+@¬ ∧]⌿⇒c                            ※ keep the ones that are not wall
  «NESW»l⍷⇒i i0≥[«NESW»i2+4%@][«»]?⇒r                     ※ r undoes the last step
  c[⇒d d r≠]⌿∂#0>[][⌫c]?                                  ※ rather not double back
  ∂#0>[0@][⌫«X»]?∂⇒l
 ]?]≔P                  ※ sensor → action
⇊
1⇒g[g][⎆∂∅=[⌫ ∅↥α 0⇒g][⇒r r3@↥α ↧β⇒a T a∈[a][«X»]?⇒a ⟨r0@ 200 «text/plain» a⟩⍅]?]⟳
«X»⇒l [P]⇉αβ
