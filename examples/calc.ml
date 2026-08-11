※ An RPN calculator on the live platform.
※
※ Two strand-locals hold the whole machine: s, the value stack, and e,
※ the digits being typed. Every button carries the slot that edits them,
※ and ⏵ redraws the tree after each one. P commits a pending entry to
※ the stack; B takes an operator quotation, commits, then folds the top
※ two values through it. A bad slot — ÷0, one operand, «..»⍎ — glitches
※ into a ✗ status line and the calculator keeps running.
※
※ Keys:  0-9 .   type a number      + - * / ^ %   operate
※        p       push it            d             drop the top
※        w       swap the top two   c             clear
※        q       quit               ⇥/↑↓←→        move focus
※ ↵ or space fires the focused button, and a mouse click fires whatever
※ drew the (key) under the pointer.
※
※   mlang run examples/calc.ml
[e#0>[s e⍎⟨⇅⟩⧺⇒s«»⇒e][]?]≔P                                     ※ commit the entry
[⇒O P s#2<[«✗ needs two values»✎][s∂#2-@⇒x s∂#1-@⇒y s 0 s#2-⊂ x y O⟨⇅⟩⧺⇒s]?]≔B
[⟨«Stack»Ⓛ
  s#0=[⟨«— empty —»⟩][s]?Ⓘ
  Ⓢ
  «Entry: »e#0=[«—»][e]?⧺Ⓛ
  Ⓢ
  ⟨«7»«7»[e«7»⧺⇒e]Ⓑ «8»«8»[e«8»⧺⇒e]Ⓑ «9»«9»[e«9»⧺⇒e]Ⓑ «÷»«/»[[÷]B]Ⓑ⟩Ⓗ
  ⟨«4»«4»[e«4»⧺⇒e]Ⓑ «5»«5»[e«5»⧺⇒e]Ⓑ «6»«6»[e«6»⧺⇒e]Ⓑ «×»«*»[[×]B]Ⓑ⟩Ⓗ
  ⟨«1»«1»[e«1»⧺⇒e]Ⓑ «2»«2»[e«2»⧺⇒e]Ⓑ «3»«3»[e«3»⧺⇒e]Ⓑ «−»«-»[[-]B]Ⓑ⟩Ⓗ
  ⟨«0»«0»[e«0»⧺⇒e]Ⓑ «.»«.»[e«.»⧺⇒e]Ⓑ «^»«^»[[^]B]Ⓑ «+»«+»[[+]B]Ⓑ⟩Ⓗ
  Ⓢ
  ⟨«Push»«p»[P]Ⓑ
   «Drop»«d»[s#0>[s 0 s#1-⊂⇒s][]?]Ⓑ
   «Swap»«w»[s#2<[][s∂#2-@⇒x s∂#1-@⇒y s 0 s#2-⊂ y⟨⇅⟩⧺ x⟨⇅⟩⧺⇒s]?]Ⓑ
   «Mod»«%»[[%]B]Ⓑ⟩Ⓗ
  ⟨«Clear»«c»[⟨⟩⇒s«»⇒e]Ⓑ «Quit»«q»[◼]Ⓑ⟩Ⓗ
 ⟩Ⓥ«Calculator»Ⓦ]≔V
⇊
⟨⟩⇒s «»⇒e [V]⏵ «Calculator closed.»⍞
