※ Dynamic scaling: the grid is one strand wide, but ⚡ adds machines
※ at runtime. Each spawned strand announces itself and sends its id
※ down channel ρ; the parent joins them all, then reduces.
※ The boot section defines W, the worker body.
[«machine »⍳⍕⧺« online»⧺⍞ ⍳↥ρ]≔W
⇊
⟨⟩ 4[⟨[W]⚡⟩⧺]⍣ ∂[⋈]∀ [⌫↧ρ]∵ 0[+]⍀ «ids sum: »⇅⍕⧺⍞
