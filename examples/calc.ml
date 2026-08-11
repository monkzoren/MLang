※ RPN Calculator with UI
※
※ Interactive calculator demonstrating Construct UI library
※ Uses ⏵ live event loop for real keyboard/mouse input

[
  ⟨«RPN Calculator»Ⓛ Ⓢ
    «Digits»Ⓛ
    ⟨«0»«0»[e«0»⇅⧺⇒e]Ⓑ
     «1»«1»[e«1»⇅⧺⇒e]Ⓑ
     «2»«2»[e«2»⇅⧺⇒e]Ⓑ
     «3»«3»[e«3»⇅⧺⇒e]Ⓑ
     «4»«4»[e«4»⇅⧺⇒e]Ⓑ
    ⟩Ⓗ
    ⟨«5»«5»[e«5»⇅⧺⇒e]Ⓑ
     «6»«6»[e«6»⇅⧺⇒e]Ⓑ
     «7»«7»[e«7»⇅⧺⇒e]Ⓑ
     «8»«8»[e«8»⇅⧺⇒e]Ⓑ
     «9»«9»[e«9»⇅⧺⇒e]Ⓑ
    ⟩Ⓗ Ⓢ
    «Operations»Ⓛ
    ⟨«+»«+»[e«»≠ [e⍎ s⧺⇒s «»⇒e] [] ?]Ⓑ
     «−»«-»[e«»≠ [e⍎ s⧺⇒s «»⇒e] [] ?]Ⓑ
     «×»«*»[e«»≠ [e⍎ s⧺⇒s «»⇒e] [] ?]Ⓑ
     «÷»«/»[e«»≠ [e⍎ s⧺⇒s «»⇒e] [] ?]Ⓑ
    ⟩Ⓗ
    ⟨«^»«^»[e«»≠ [e⍎ s⧺⇒s «»⇒e] [] ?]Ⓑ
     «%»«%»[e«»≠ [e⍎ s⧺⇒s «»⇒e] [] ?]Ⓑ
     «=»«=»[e«»≠ [e⍎ s⧺⇒s «»⇒e] [] ?]Ⓑ
     «Clr»«c»[[] ⇒s «»⇒e]Ⓑ
    ⟩Ⓗ Ⓢ
    «Controls»Ⓛ
    ⟨«Quit»«q»[◼]Ⓑ⟩Ⓗ
   ⟩Ⓥ«Calculator»Ⓦ
]≔V

⇊

[] ⇒s «»⇒e [V]⏵ «Closed.»⍞
