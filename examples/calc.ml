※ RPN Calculator with UI, inspired by construct.ml
※
※ Like construct.ml, this is a PySide-style application. The view quotation V
※ rebuilds the widget tree each frame from two strand-locals: stack (values)
※ and entry (current digit input). Each button carries its slot: the quotation
※ the event loop ▶ runs when that button's key arrives on stdin.
※
※ Keys: 0-9 (digits), +, -, ×, ÷, ^, % (operators), =, c (clear), q (quit)
※ Try:
※   printf '3⏎4⏎+⏎q⏎' | mlang run examples/calc.ml

[⇒t t«+»=[[↓ ↓ +]][t«-»=[[↓ ↓ -]][t«×»=[[↓ ↓ ×]][t«÷»=[[↓ ↓ ÷]][t«^»=[[↓ ↓ ^]][t«%»=[[↓ ↓ %]][t⍎]?]?]?]?]?]?]≔T

⇊

[
  ⟨«Stack»Ⓛ
    ⟨stack ⇅ « » ⊆ [«»≠] ⌿ [⍕] ∀⟩Ⓘ
    Ⓢ
    «Entry: « ⍕entry »Ⓛ
    Ⓢ
    «Digits»Ⓛ
    ⟨«0»«0»[entry«0»⇅⧺⇒entry]Ⓑ
     «1»«1»[entry«1»⇅⧺⇒entry]Ⓑ
     «2»«2»[entry«2»⇅⧺⇒entry]Ⓑ
     «3»«3»[entry«3»⇅⧺⇒entry]Ⓑ
     «4»«4»[entry«4»⇅⧺⇒entry]Ⓑ
    ⟩Ⓗ
    ⟨«5»«5»[entry«5»⇅⧺⇒entry]Ⓑ
     «6»«6»[entry«6»⇅⧺⇒entry]Ⓑ
     «7»«7»[entry«7»⇅⧺⇒entry]Ⓑ
     «8»«8»[entry«8»⇅⧺⇒entry]Ⓑ
     «9»«9»[entry«9»⇅⧺⇒entry]Ⓑ
    ⟩Ⓗ
    Ⓢ
    «Operators»Ⓛ
    ⟨«+»«+»[entry«»≠ [entry⇐ stack⧺⇒stack] [] ? stack «+» T⟨⇅⟩⧺⇒stack «»⇒entry]Ⓑ
     «−»«-»[entry«»≠ [entry⇐ stack⧺⇒stack] [] ? stack «-» T⟨⇅⟩⧺⇒stack «»⇒entry]Ⓑ
     «×»«*»[entry«»≠ [entry⇐ stack⧺⇒stack] [] ? stack «×» T⟨⇅⟩⧺⇒stack «»⇒entry]Ⓑ
     «÷»«/»[entry«»≠ [entry⇐ stack⧺⇒stack] [] ? stack «÷» T⟨⇅⟩⧺⇒stack «»⇒entry]Ⓑ
     «^»«^»[entry«»≠ [entry⇐ stack⧺⇒stack] [] ? stack «^» T⟨⇅⟩⧺⇒stack «»⇒entry]Ⓑ
     «%»«%»[entry«»≠ [entry⇐ stack⧺⇒stack] [] ? stack «%» T⟨⇅⟩⧺⇒stack «»⇒entry]Ⓑ
    ⟩Ⓗ
    Ⓢ
    ⟨«=»«=»[entry«»≠ [entry⇐ stack⧺⇒stack] [] ? «»⇒entry]Ⓑ
     «C»«c»[∅⇒stack «»⇒entry]Ⓑ
     «Q»«q»[◼]Ⓑ
    ⟩Ⓗ
   ⟩Ⓥ«RPN Calculator»Ⓦ
]≔V

⇊

∅⇒stack «»⇒entry [V]▶ «Done.»⍞
