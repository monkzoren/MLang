※ a deliberately bad candidate — three new strands, none of them wired to anything
※ It exists to check that the gate says no.
※ v1 — rebind a definition
※
※ P's axis tie-break, ≥ becomes >. One glyph. Nothing structural: the
※ wiring diagram is unchanged and no strand is replaced — only what
※ flows through the pathway. Behaviour moves, which is the proof that a
※ hot rebind reaches a strand that is already running.
※ os0 — the starting operating system of the delivery machine.
※
※ Two strands and one pathway:
※
※   strand 0  the body     ⎆ → α … β → ⍅
※   strand 1  the policy   «X»⇒l [P]⇉αβ
※
※ The body accepts a sensor tick, puts it on α, waits on β for an
※ action, and answers with it. The policy is a pump: one value in, one
※ value out, no other state — the most neuron-shaped thing the language
※ has. Every later version grows from here.
※
※ A tick body is eleven integers, space separated:
※   x y carrying  north east south west  px py  dx dy
※ where the four compass fields are 1 when that neighbour is a wall,
※ and (px,py) / (dx,dy) are the pickup and dropoff of the open job.
※ An action is one of N S E W G D X.

[« »⊆[⍎]∵]≔F                                              ※ tick → list of ints
[F⇒v v2@[v9@ v10@][v7@ v8@]?⇒z⇒u
 v0@u= v1@z= ∧[v2@[«D»][«G»]?][
  v0@u<[«E»][v0@u>[«W»][«»]?]?⇒h                          ※ the horizontal wish
  v1@z<[«S»][v1@z>[«N»][«»]?]?⇒w                          ※ the vertical wish
  v0@u-∂0<[±][]?⇒p v1@z-∂0<[±][]?⇒q                       ※ how far, each way
  p q>[⟨h w⟩][⟨w h⟩]?⟨«N»«E»«S»«W»⟩⧺                      ※ wishes first, then any escape
  [⇒d d#0> v«NESW»d⍷3+@¬ ∧]⌿⇒c                            ※ keep the ones that are not wall
  «NESW»l⍷⇒i i0≥[«NESW»i2+4%@][«»]?⇒r                     ※ r undoes the last step
  c[⇒d d r≠]⌿∂#0>[][⌫c]?                                  ※ rather not double back
  ∂#0>[0@][⌫«X»]?∂⇒l
 ]?]≔P                  ※ sensor → action
⇊
1⇒g[g][⎆∂∅=[⌫ ∅↥α 0⇒g][⇒r r3@↥α ↧β⇒a ⟨r0@ 200 «text/plain» a⟩⍅]?]⟳
«X»⇒l [P]⇉αβ
[]⇉γδ
[]⇉εζ
[]⇉ηθ
