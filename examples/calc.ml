※ RPN Calculator — a Construct application (see construct.ml for the tour).
※
※ A PySide-style program: the view quotation V rebuilds the widget tree
※ each frame from two strand-locals — s (the stack, a list) and e (the
※ digits typed so far). Every button carries its slot; the ▶ event loop
※ runs the slot whose key arrives as a stdin line. A glitch in a slot
※ (÷ by zero below!) becomes a ✗ status message and the app carries on —
※ the calc.ml let-it-crash pattern.
※
※ Keys: 0-9 digits · + - * / ^ % operators · = push · c clear · q quit
※ An operator first pushes the pending entry, then folds the top two.
※ Try:  printf '3⏎=⏎4⏎+⏎0⏎/⏎c⏎9⏎=⏎3⏎/⏎q⏎' | mlang run examples/calc.ml
※
※ E: commit the typed entry onto the stack     (e → s)
[e«»≠[s e⍎⟨⇅⟩⧺⇒s«»⇒e][]?]≔E
※ A: pop a b, run the operator quotation, push the result
[⇒o E s#2≥[s∂#2-@s∂#1-@o⟨⇅⟩s s#2-⊤⇅⧺⇒s][«need two values»✎]?]≔A

[
  ⟨«Stack»Ⓛ
    sⒾ
    Ⓢ
    «Entry: »e⧺Ⓛ
    Ⓢ
    ⟨«0»«0»[e«0»⧺⇒e]Ⓑ
     «1»«1»[e«1»⧺⇒e]Ⓑ
     «2»«2»[e«2»⧺⇒e]Ⓑ
     «3»«3»[e«3»⧺⇒e]Ⓑ
     «4»«4»[e«4»⧺⇒e]Ⓑ
    ⟩Ⓗ
    ⟨«5»«5»[e«5»⧺⇒e]Ⓑ
     «6»«6»[e«6»⧺⇒e]Ⓑ
     «7»«7»[e«7»⧺⇒e]Ⓑ
     «8»«8»[e«8»⧺⇒e]Ⓑ
     «9»«9»[e«9»⧺⇒e]Ⓑ
    ⟩Ⓗ
    Ⓢ
    ⟨«+»«+»[[+]A]Ⓑ
     «−»«-»[[-]A]Ⓑ
     «×»«*»[[×]A]Ⓑ
     «÷»«/»[[÷]A]Ⓑ
     «^»«^»[[^]A]Ⓑ
     «%»«%»[[%]A]Ⓑ
    ⟩Ⓗ
    Ⓢ
    ⟨«=»«=»[E]Ⓑ
     «C»«c»[⟨⟩⇒s«»⇒e]Ⓑ
     «Q»«q»[◼]Ⓑ
    ⟩Ⓗ
   ⟩Ⓥ«RPN Calculator»Ⓦ
]≔V

⇊

⟨⟩⇒s «»⇒e [V]▶ «Done.»⍞
