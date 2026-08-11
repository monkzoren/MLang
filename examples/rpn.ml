※ A fault-tolerant concurrent RPN calculator.
※
※ Three-stage pipeline over channels l (lines) and o (output):
※   strand 0 reads stdin lines            → l
※   strand 1 evaluates each line          → o
※   strand 2 prints results
※
※ Strand 1 spawns a FRESH strand per line (⚡): the line is evaluated
※ on that strand's own clean stack — MLang evaluating MLang, since the
※ input is postfix too. T dispatches one token: an operator applies,
※ anything else must parse as a number. A bad line glitches inside ⍥,
※ so it reports «✗ …» and the calculator keeps running; ⋈ keeps
※ answers in input order. For the same arithmetic behind the Construct's
※ widgets instead of a pipe, see examples/calc.ml. Try:
※   printf '3 4 +⏎10 2 - 6 ×⏎1 0 ÷⏎2 63 ^⏎oops⏎' | mlang run examples/rpn.ml
[⇒t t«+»=[+][t«-»=[-][t«×»=[×][t«÷»=[÷][t«^»=[^][t«%»=[%][t⍎]?]?]?]?]?]?]≔T
⇊
[⌨∂∅≠][↥l]⟳⌫∅↥l
[↧l∂∅≠][⇒L[[L« »⊆[«»≠]⌿[T]∀⍕][«✗ »⇅⧺]⍥↥o]⚡⋈]⟳⌫∅↥o
[↧o∂∅≠][⍞]⟳⌫
