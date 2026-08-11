※ The Oracle — a concurrent MapReduce analytics engine you can question.
※
※ Phase 1, ingest: strand 0 reads document lines until «.», dealing them
※ onto channel δ, then deals one ∅ per mapper. Strands 1–3 are mappers:
※ three receivers on one channel form a work-stealing pool — each steals
※ a line, lowercases and splits it, and streams the words to the counter
※ channel ω, signing off with its own ∅ when the deck runs dry.
※
※ Phase 2, serve: strand 4 folds ω into an association list until all
※ three mappers have signed off, then becomes the state-owning actor:
※ requests arrive on κ, answers leave on β. Strand 5 opens the floor
※ (after the go-signal γ — it must not race strand 0 for stdin), reads
※ one query per line, and parses each on a freshly spawned strand inside
※ ⍥: a malformed command reports «✗ …» and dies alone, the calc.ml
※ let-it-crash pattern. Strand 6 prints every answer, in command order
※ because strand 5 joins each parser before reading on.
※
※   count WORD · top K · stats · q     (document first, closed by «.»)
※   printf 'There is no spoon⏎.⏎count spoon⏎top 3⏎stats⏎q⏎' \
※     | mlang run examples/oracle.ml
※
※ B: counts word → counts′ — bump one word's tally.
[⇒z ∂[⊃z=]⌿⇒m [⊃z≠]⌿ ⟨⟨z m⟨⟩=[1][m⊃1@1+]?⟩⟩⧺]≔B
※ G: counts word → n — look a tally up, 0 when the Oracle never saw it.
[⇒z [⊃z=]⌿⇒m m⟨⟩=[0][m⊃1@]?]≔G
※ T: counts k → report — top k by count, ties alphabetical: each pair
※ becomes the sortable key «(11000−n) word» (counts stay below 1000),
※ so one string sort ⍋ orders by count descending, then word ascending.
[⇒k [∂1@11000⇅-⍕« »⧺⇅⊃⧺]∵⍋ k⊤ [⍭∂1@⇅0@⍎11000⇅-⍕« × »⇅⧺⧺]∵«⏎»⊇]≔T
⇊
0⇒n [⌨∂∅≠⊚«.»≠∧][↥δ n1+⇒n]⟳⌫ n≔Y 3[∅↥δ]⍣
[↧δ∂∅≠][⇩⍭[↥ω]∀]⟳⌫ ∅↥ω
[↧δ∂∅≠][⇩⍭[↥ω]∀]⟳⌫ ∅↥ω
[↧δ∂∅≠][⇩⍭[↥ω]∀]⟳⌫ ∅↥ω
0⇒d ⟨⟩ [d3<][↧ω∂∅=[⌫d1+⇒d][B]?]⟳ 1↥γ 1⇒g
⋮ [g][↧κ⇒q q⊃«count»=[∂q1@G q1@«: »⧺⇅⍕⧺↥β][q⊃«top»=[∂q1@T↥β][q⊃«stats»=[∂[1@]∵∑⇒w «lines »Y⍕⧺« · words »⧺w⍕⧺« · distinct »⧺⊚#⍕⧺↥β][0⇒g «goodbye, operator»↥α ∅↥α]?]?]?]⟳⌫
↧γ⌫ 1⇒r [r][⌨⇒v v∅=v«q»=∨[0⇒r ⟨«halt»⟩↥κ][[[v⍭⇒t t⊃«count»=[⟨«count»t1@⟩][t⊃«top»=[⟨«top»t1@⍎⟩][t⊃«stats»=[⟨«stats»⟩][«unknown command»↯]?]?]?↥κ ↧β↥α][⌫«✗ »v⧺↥α]⍥]⚡⋈]?]⟳
[↧α∂∅≠][⍞]⟳⌫
