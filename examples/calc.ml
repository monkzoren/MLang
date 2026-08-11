※ RPN Calculator with UI, inspired by construct.ml
※
※ Like construct.ml, this is a PySide-style application. The view quotation V
※ rebuilds the widget tree each frame from strand-locals: s (stack values) and
※ e (current digit input). Each button carries its slot that event loop ▶
※ runs when that button's key arrives on stdin.
※
※ Keys: 0-9 (digits), +, -, ×, ÷, ^, % (operators), = (push), c (clear), q (quit)
※ Try:
※   printf '3⏎4⏎+⏎q⏎' | mlang run examples/calc.ml

[
  ⟨«Stack»Ⓛ
    sⒾ
    Ⓢ
    «Entry: « e »Ⓛ
    Ⓢ
    «Digits»Ⓛ
    ⟨«0»«0»[e«0»⇅⧺⇒e]Ⓑ
     «1»«1»[e«1»⇅⧺⇒e]Ⓑ
     «2»«2»[e«2»⇅⧺⇒e]Ⓑ
     «3»«3»[e«3»⇅⧺⇒e]Ⓑ
     «4»«4»[e«4»⇅⧺⇒e]Ⓑ
     «5»«5»[e«5»⇅⧺⇒e]Ⓑ
     «6»«6»[e«6»⇅⧺⇒e]Ⓑ
     «7»«7»[e«7»⇅⧺⇒e]Ⓑ
     «8»«8»[e«8»⇅⧺⇒e]Ⓑ
     «9»«9»[e«9»⇅⧺⇒e]Ⓑ
    ⟩Ⓗ
    Ⓢ
    «Operators»Ⓛ
    ⟨«+»«+»[e«»≠ s ∂2≥ ∧ [e⍎ s⧺⇒s ↓ ↓ + ⧺⇒s «»⇒e] [] ? ]Ⓑ
     «−»«-»[e«»≠ s ∂2≥ ∧ [e⍎ s⧺⇒s ↓ ↓ - ⧺⇒s «»⇒e] [] ? ]Ⓑ
     «×»«*»[e«»≠ s ∂2≥ ∧ [e⍎ s⧺⇒s ↓ ↓ × ⧺⇒s «»⇒e] [] ? ]Ⓑ
     «÷»«/»[e«»≠ s ∂2≥ ∧ [e⍎ s⧺⇒s ↓ ↓ ÷ ⧺⇒s «»⇒e] [] ? ]Ⓑ
     «^»«^»[e«»≠ s ∂2≥ ∧ [e⍎ s⧺⇒s ↓ ↓ ^ ⧺⇒s «»⇒e] [] ? ]Ⓑ
     «%»«%»[e«»≠ s ∂2≥ ∧ [e⍎ s⧺⇒s ↓ ↓ % ⧺⇒s «»⇒e] [] ? ]Ⓑ
    ⟩Ⓗ
    Ⓢ
    ⟨«=»«=»[e«»≠ [e⍎ s⧺⇒s «»⇒e] [] ?]Ⓑ
     «C»«c»[[] ⇒s «»⇒e]Ⓑ
     «Q»«q»[◼]Ⓑ
    ⟩Ⓗ
   ⟩Ⓥ«RPN Calculator»Ⓦ
]≔V

⇊

[] ⇒s «»⇒e [V]⏵ «Done.»⍞
