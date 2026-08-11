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
    ⟨«0»«d0»[e«0»⇅⧺⇒e]Ⓑ
     «1»«d1»[e«1»⇅⧺⇒e]Ⓑ
     «2»«d2»[e«2»⇅⧺⇒e]Ⓑ
     «3»«d3»[e«3»⇅⧺⇒e]Ⓑ
    ⟩Ⓗ
    ⟨«4»«d4»[e«4»⇅⧺⇒e]Ⓑ
     «5»«d5»[e«5»⇅⧺⇒e]Ⓑ
     «6»«d6»[e«6»⇅⧺⇒e]Ⓑ
     «7»«d7»[e«7»⇅⧺⇒e]Ⓑ
    ⟩Ⓗ
    ⟨«8»«d8»[e«8»⇅⧺⇒e]Ⓑ
     «9»«d9»[e«9»⇅⧺⇒e]Ⓑ
     «+»«op_add»[e«»≠ [e⍎ s⧺⇒s «»⇒e] [] ?]Ⓑ
     «−»«op_sub»[e«»≠ [e⍎ s⧺⇒s «»⇒e] [] ?]Ⓑ
    ⟩Ⓗ
    ⟨«×»«op_mul»[e«»≠ [e⍎ s⧺⇒s «»⇒e] [] ?]Ⓑ
     «÷»«op_div»[e«»≠ [e⍎ s⧺⇒s «»⇒e] [] ?]Ⓑ
     «^»«op_pow»[e«»≠ [e⍎ s⧺⇒s «»⇒e] [] ?]Ⓑ
     «%»«op_mod»[e«»≠ [e⍎ s⧺⇒s «»⇒e] [] ?]Ⓑ
    ⟩Ⓗ
    ⟨«=»«op_eq»[e«»≠ [e⍎ s⧺⇒s «»⇒e] [] ?]Ⓑ
     «Clr»«clr»[[] ⇒s «»⇒e]Ⓑ
     «Quit»«q»[◼]Ⓑ
    ⟩Ⓗ
   ⟩Ⓥ«Calculator»Ⓦ
]≔V

⇊

[] ⇒s «»⇒e [V]⏵ «Done.»⍞
