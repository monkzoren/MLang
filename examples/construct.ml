※ The Construct in action: the Nebuchadnezzar's operator console.
※
※ A PySide-style application. The view quotation V rebuilds the widget
※ tree each frame from four strand-locals — s signal, n operator,
※ r red pill, c crew — and every interactive widget carries its slot:
※ the quotation the event loop ▶ runs when that widget's key arrives
※ on stdin. State changes are just ⇒ rebinds; the next frame shows
※ them. Nothing is ever mutated in place, so no slot can corrupt the
※ interface — the worst a bad line can do is a ✗ status message.
※
※ Keys:  + / -        boost / cut the signal
※        n NAME       rename the operator      j  jack the operator in
※        r            toggle the red pill      q  quit
※ Try:
※   printf '+⏎n Trinity⏎j⏎r⏎q⏎' | mlang run examples/construct.ml
[⟨«Wake up, Neo…»Ⓛ
  Ⓢ
  «n»«Operator»n[⇒n]Ⓔ
  ⟨«Jack in»«j»[c n⟨⇅⟩⧺⇒c n« is aboard»⧺✎]Ⓑ «Red pill»«r»r[r¬⇒r]Ⓒ⟩Ⓗ
  Ⓢ
  «Signal strength»Ⓛ
  ⟨s 10Ⓟ «+»«+»[s1+10⊓⇒s]Ⓑ «−»«-»[s1-0⊔⇒s]Ⓑ⟩Ⓗ
  Ⓢ
  «Crew aboard»Ⓛ
  cⒾ
  «»Ⓛ
  «Exit»«q»[◼]Ⓑ
 ⟩Ⓥ«Nebuchadnezzar — operator console»Ⓦ]≔V
⇊
3⇒s «Neo»⇒n 0⇒r ⟨«Morpheus» «Trinity»⟩⇒c [V]▶ «Connection terminated.»⍞
