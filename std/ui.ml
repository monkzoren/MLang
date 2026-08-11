※ The Construct — the MLang UI library, in the lineage of Qt/PySide.
※
※ Written in MLang and woven into a program's boot strand automatically
※ when the program references any Construct sigil it does not define
※ itself (SPEC §6.1). `mlang ui` prints this source.
※
※ Widgets are immutable values: tagged lists built by constructors.
※ Signals are slots: a quotation carried by the widget, run by the event
※ loop in the app's own strand when its key arrives on stdin.
※ Internals use circled-lowercase sigils (ⓐ ⓡ …) and fullwidth-letter
※ strand-locals (ｑ ｖ ｗ ｓ …) — treat both as reserved, like std's.

※ ── plumbing ──────────────────────────────────────────────────────
[⇅⟨⇅⟩⇅⧺]≔ⓐ               ※ cons              x L ⓐ → ⟨x …L⟩
[⍸«»[⌫⊚⧺]⍀⇅⌫]≔ⓡ          ※ repeat            s n ⓡ → s…s (n copies)
[[#]∵0[⊔]⍀]≔ⓦ            ※ block width       lines ⓦ → longest length
[⇒ｚ⇒ｙ ｙ ｙ#0>[ｙ0 1⊂«─»=][0]?[«─»][« »]? ｚｙ#-0⊔ⓡ⧺]≔ⓟ  ※ pad line to ｚ (─ lines stretch with ─)

※ ── widget constructors (the Qt cast) ─────────────────────────────
[⟨⇅⟩«L»⇅ⓐ]≔Ⓛ             ※ QLabel        «text»Ⓛ (⏎ makes lines; «»Ⓛ is a spacer)
[⟨⇅⟩ⓐⓐ«B»⇅ⓐ]≔Ⓑ           ※ QPushButton   «caption»«key»[slot]Ⓑ
[⟨⇅⟩ⓐⓐⓐ«E»⇅ⓐ]≔Ⓔ          ※ QLineEdit     «key»«label»«value»[slot]Ⓔ — slot gets the typed text
[⟨⇅⟩ⓐⓐⓐ«C»⇅ⓐ]≔Ⓒ          ※ QCheckBox     «caption»«key»state[slot]Ⓒ
[⟨⇅⟩ⓐ«P»⇅ⓐ]≔Ⓟ            ※ QProgressBar  value maximum Ⓟ
[⟨⇅⟩«I»⇅ⓐ]≔Ⓘ             ※ QListWidget   ⟨items…⟩Ⓘ
[⟨«S»⟩]≔Ⓢ                ※ separator     Ⓢ — a ─ rule, stretches to the window
[⟨⇅⟩«V»⇅ⓐ]≔Ⓥ             ※ QVBoxLayout   ⟨widgets…⟩Ⓥ
[⟨⇅⟩«H»⇅ⓐ]≔Ⓗ             ※ QHBoxLayout   ⟨widgets…⟩Ⓗ
[⟨⇅⟩⇅⟨⇅⟩⧺«W»⇅ⓐ]≔Ⓦ        ※ QMainWindow   widget«title»Ⓦ

※ ── rendering: widget → list of lines ─────────────────────────────
[∂1@«[ »⇅⧺« ](»⧺⇅2@⧺«)»⧺⟨⇅⟩]≔ⓑ                            ※ button   [ caption ](key)
[∂1@⇒ｙ∂3@⇒ｚ2@«: »⧺ｚ⧺«▁»12ｚ#-0⊔ⓡ⧺« (»⧺ｙ⧺«)»⧺⟨⇅⟩]≔ⓔ      ※ edit     label: value▁▁▁ (key)
[∂1@⇒ｙ∂2@⇒ｚ3@[«[×] »][«[ ] »]?ｙ⧺« (»⧺ｚ⧺«)»⧺⟨⇅⟩]≔ⓒ       ※ checkbox [×] caption (key)
[∂1@⇒ｙ2@⇒ｚ ｙ20×ｚ÷⌊0⊔20⊓⇒ｔ«▓»ｔⓡ«░»20ｔ-ⓡ⧺« »⧺ｙ100×ｚ÷⌊⍕⧺«%»⧺⟨⇅⟩]≔ⓖ  ※ progress ▓▓░░ n%

※ hbox: blocks side by side. Pad every block to its own width and the
※ tallest height, then join the rows with a two-space gutter.
[⇒ｏ ｏ[#]∵0[⊔]⍀⇒ｎ ｏ[∂ⓦ⇒ｒ[ｒⓟ]∵∂#ｎ⇅-0⊔⍸[⌫«»ｒⓟ]∵⧺]∵⇒ｏ ｎ⍸[⇒ｒｏ[ｒ@]∵«  »⊇]∵]≔ⓗ

※ window: frame the content, title in the top border.
[⇒ｔ⇒ｏ ｏⓦｔ#2+⊔⇒ｎ ｔ#0=[«┌»«─»ｎ2+ⓡ⧺«┐»⧺][«┌─ »ｔ⧺« »⧺«─»ｎｔ#-1-ⓡ⧺«┐»⧺]?
⋮⟨⇅⟩ｏ[ｎⓟ«│ »⇅⧺« │»⧺]∵⧺«└»«─»ｎ2+ⓡ⧺«┘»⧺⟨⇅⟩⧺]≔ⓩ

※ render: one dispatcher, one row of glyphs per widget kind.
[⍙«list»=[∂#0>][0]?[∂⊃«L»=[1@⍖][∂⊃«B»=[ⓑ][∂⊃«E»=[ⓔ][∂⊃«C»=[ⓒ][∂⊃«P»=[ⓖ][∂⊃«I»=[1@[⍕«• »⇅⧺]∵][∂⊃«S»=[⌫⟨«─»⟩][∂⊃«V»=[1@[ⓛ]∵⟨⟩[⧺]⍀][∂⊃«H»=[1@[ⓛ]∵ⓗ][∂⊃«W»=[∂2@ⓛ⇅1@ⓩ][«⌺ not a widget: »⇅⍕⧺↯]?]?]?]?]?]?]?]?]?]?][«⌺ not a widget: »⇅⍕⧺↯]?]≔ⓛ

[ⓛ[⍞]∀]≔⌺                ※ draw: render a widget tree and print it

※ ── signals & slots ───────────────────────────────────────────────
※ The keymap walks a tree to ⟨key kind slot⟩ entries — the widgets
※ that listen. Input protocol: a line is «key» or «key argument».
[∂⊃«B»=[∂3@⟨⇅⟩«B»⇅ⓐ⇅2@⇅ⓐ⟨⇅⟩][∂⊃«E»=[∂4@⟨⇅⟩«E»⇅ⓐ⇅1@⇅ⓐ⟨⇅⟩][∂⊃«C»=[∂4@⟨⇅⟩«C»⇅ⓐ⇅2@⇅ⓐ⟨⇅⟩][∂⊃∂«V»=⇅«H»=∨[1@[ⓚ]∵⟨⟩[⧺]⍀][∂⊃«W»=[2@ⓚ][⌫⟨⟩]?]?]?]?]?]≔ⓚ

※ dispatch one input line against the current tree ｗ: run the slot of
※ the widget whose key matches (line edits get the argument text).
[ｌ⍭∂#0=[⌫][⊃⇒ｋ ｌｋ#1+⊥⇒ｕ ｗⓚ[⊃ｋ=]⌿⇒ｍ ｍ#0=[«? »ｌ⧺✎][ｍ⊃∂1@«E»=[2@ｕ⇅!][2@!]?]?]?]≔ⓓ

※ ── the application object ────────────────────────────────────────
[⇒ｓ]≔✎                   ※ status bar    «message»✎ — shown under the next frame
[0⇒ｑ]≔◼                  ※ quit          ◼ — ends the event loop
※ exec: [view]▶ — Qt's app.exec(). Each turn: rebuild the tree from
※ the view quotation, draw it, show the status line, read a line of
※ stdin, dispatch it. A glitch in a slot becomes a ✗ status message —
※ the loop survives. EOF (∅) or ◼ ends the loop.
[⇒ｖ 1⇒ｑ«»⇒ｓ[ｑ][ｖ⇒ｗ ｗ⌺ ｓ#0>[ｓ⍞«»⇒ｓ][]?⌨∂∅=[⌫0⇒ｑ][⇒ｌ ｌ#0>[[ⓓ][⍕«✗ »⇅⧺✎]⍥][]?]?]⟳]≔▶
