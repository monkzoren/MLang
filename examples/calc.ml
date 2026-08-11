※ RPN Calculator with UI, inspired by construct.ml
※
※ Interactive calculator using the Construct widget library.
※ Real-time keyboard and mouse input with focus management.
※
※ Keys: 0-9 (digits), +, -, ×, ÷, ^, % (operators), = (push), c (clear), q (quit)

[
  ⟨«Stack»Ⓛ
    sⒾ
    Ⓢ
    «Digits»Ⓛ
    ⟨«0»«0»[e«0»⇅⧺⇒e]Ⓑ
     «1»«1»[e«1»⇅⧺⇒e]Ⓑ
     «2»«2»[e«2»⇅⧺⇒e]Ⓑ
     «3»«3»[e«3»⇅⧺⇒e]Ⓑ
    ⟩Ⓗ
    ⟨«4»«4»[e«4»⇅⧺⇒e]Ⓑ
     «5»«5»[e«5»⇅⧺⇒e]Ⓑ
     «6»«6»[e«6»⇅⧺⇒e]Ⓑ
     «7»«7»[e«7»⇅⧺⇒e]Ⓑ
    ⟩Ⓗ
    ⟨«8»«8»[e«8»⇅⧺⇒e]Ⓑ
     «9»«9»[e«9»⇅⧺⇒e]Ⓑ
     «+»«+»[e«»≠ [e⍎ s⧺⇒s «»⇒e] [] ?]Ⓑ
     «−»«-»[e«»≠ [e⍎ s⧺⇒s «»⇒e] [] ?]Ⓑ
    ⟩Ⓗ
    ⟨«×»«*»[e«»≠ [e⍎ s⧺⇒s «»⇒e] [] ?]Ⓑ
     «÷»«/»[e«»≠ [e⍎ s⧺⇒s «»⇒e] [] ?]Ⓑ
     «^»«^»[e«»≠ [e⍎ s⧺⇒s «»⇒e] [] ?]Ⓑ
     «%»«%»[e«»≠ [e⍎ s⧺⇒s «»⇒e] [] ?]Ⓑ
    ⟩Ⓗ
    ⟨«=»«=»[e«»≠ [e⍎ s⧺⇒s «»⇒e] [] ?]Ⓑ
     «Clr»«c»[[] ⇒s «»⇒e]Ⓑ
     «Quit»«q»[◼]Ⓑ
    ⟩Ⓗ
   ⟩Ⓥ«Calculator»Ⓦ
]≔V

⇊

[] ⇒s «»⇒e [V]⏵ «Done.»⍞
