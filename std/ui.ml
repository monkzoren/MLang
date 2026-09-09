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
※ The strand-local ｉ holds the focused widget's key («» when nothing
※ has focus): the focused widget swaps its [ ] for ⟦ ⟧ — same width,
※ so the layout never shifts — and a focused edit shows the ▏caret.
[∂1@⇒ｔ2@⇒ｚ ｚｉ=[«⟦ »ｔ⧺« ⟧(»⧺][«[ »ｔ⧺« ](»⧺]?ｚ⧺«)»⧺⟨⇅⟩]≔ⓑ            ※ button   [ caption ](key)
[∂1@⇒ｙ∂2@⇒ｔ3@ｙｉ=[«▏»⧺][]?⇒ｚ ｔ«: »⧺ｚ⧺«▁»12ｚ#-0⊔ⓡ⧺« (»⧺ｙ⧺«)»⧺⟨⇅⟩]≔ⓔ ※ edit     label: value▁▁▁ (key)
[∂1@⇒ｔ∂2@⇒ｚ3@[«×»][« »]?⇒ｙ ｚｉ=[«⟦»ｙ⧺«⟧ »⧺][«[»ｙ⧺«] »⧺]?ｔ⧺« (»⧺ｚ⧺«)»⧺⟨⇅⟩]≔ⓒ ※ checkbox [×] caption (key)
※ progress ▓▓░░ n% — the percentage is clamped to 0…100 and a maximum
※ of zero (or less) simply shows an empty bar, so drawing never glitches
[∂1@⇒ｙ2@⇒ｚ ｚ0>[ｙ100×ｚ÷⌊0⊔100⊓][0]?⇒ｔ«▓»ｔ5÷⌊ⓡ«░»20ｔ5÷⌊-ⓡ⧺« »⧺ｔ⍕⧺«%»⧺⟨⇅⟩]≔ⓖ

※ hbox: blocks side by side. Pad every block to its own width and the
※ tallest height, then join the rows with a two-space gutter.
[⇒ｏ ｏ[#]∵0[⊔]⍀⇒ｎ ｏ[∂ⓦ⇒ｒ[ｒⓟ]∵∂#ｎ⇅-0⊔⍸[⌫«»ｒⓟ]∵⧺]∵⇒ｏ ｎ⍸[⇒ｒｏ[ｒ@]∵«  »⊇]∵]≔ⓗ

※ window: frame the content, title in the top border.
[⇒ｔ⇒ｏ ｏⓦｔ#2+⊔⇒ｎ ｔ#0=[«┌»«─»ｎ2+ⓡ⧺«┐»⧺][«┌─ »ｔ⧺« »⧺«─»ｎｔ#-1-ⓡ⧺«┐»⧺]?
⋮⟨⇅⟩ｏ[ｎⓟ«│ »⇅⧺« │»⧺]∵⧺«└»«─»ｎ2+ⓡ⧺«┘»⧺⟨⇅⟩⧺]≔ⓩ

※ render: one dispatcher, one row of glyphs per widget kind.
[⍙«list»=[∂#0>][0]?[∂⊃«L»=[1@⍖][∂⊃«B»=[ⓑ][∂⊃«E»=[ⓔ][∂⊃«C»=[ⓒ][∂⊃«P»=[ⓖ][∂⊃«I»=[1@[⍕«• »⇅⧺]∵][∂⊃«S»=[⌫⟨«─»⟩][∂⊃«V»=[1@[ⓛ]∵⟨⟩[⧺]⍀][∂⊃«H»=[1@[ⓛ]∵ⓗ][∂⊃«W»=[∂2@ⓛ⇅1@ⓩ][«⌺ not a widget: »⇅⍕⧺↯]?]?]?]?]?]?]?]?]?]?][«⌺ not a widget: »⇅⍕⧺↯]?]≔ⓛ

[«»⇒ｉⓛ[⍞]∀]≔⌺           ※ draw: render a widget tree, nothing focused, print it

※ ── signals & slots ───────────────────────────────────────────────
※ The keymap walks a tree to ⟨key kind slot⟩ entries (edits carry
※ their value too: ⟨key «E» slot value⟩) — the widgets that listen,
※ in layout order, which is also the ⏵ focus order.
[∂⊃«B»=[∂3@⟨⇅⟩«B»⇅ⓐ⇅2@⇅ⓐ⟨⇅⟩][∂⊃«E»=[∂3@⟨⇅⟩⇅∂4@⥀ⓐ«E»⇅ⓐ⇅1@⇅ⓐ⟨⇅⟩][∂⊃«C»=[∂4@⟨⇅⟩«C»⇅ⓐ⇅2@⇅ⓐ⟨⇅⟩][∂⊃∂«V»=⇅«H»=∨[1@[ⓚ]∵⟨⟩[⧺]⍀][∂⊃«W»=[2@ⓚ][⌫⟨⟩]?]?]?]?]?]≔ⓚ

※ run a slot with stack discipline: args… [slot] n ⓢ, n being the
※ number of arguments the slot takes. Everything beneath them belongs
※ to the application: whatever the slot leaves on top is dropped, and
※ a slot that ate into the application's values is reported as a ✗
※ status message. (ｄ holds the expected depth — a slot must not draw.)
[≢2-⇅-⇒ｄ!≢ｄ-∂0>[[⌫]⍣][∂0<[±⍕«✗ slot consumed »⇅⧺« values»⧺✎][⌫]?]?]≔ⓢ

※ dispatch one input line against the current tree ｗ: run the slot of
※ the widget whose key matches (line edits get the argument text — the
※ rest of the line after the key, without its leading spaces).
[ｌ⍭∂#0=[⌫][⊃⇒ｋ ｌ∂ｋ⍷ｋ#+⊥⇒ｕ[ｕ#0>[ｕ0 1⊂« »=][0]?][ｕ1ｕ#⊂⇒ｕ]⟳
⋮ｗⓚ[⊃ｋ=]⌿⇒ｍ ｍ#0=[«? »ｌ⧺✎][ｍ⊃∂1@«E»=[2@ｕ⇅1ⓢ][2@0ⓢ]?]?]?]≔ⓓ

※ ── the application object ────────────────────────────────────────
[⍕⇒ｓ]≔✎                  ※ status bar    «message»✎ — shown under the next frame (any value; stringified)
[0⇒ｑ]≔◼                  ※ quit          ◼ — ends the event loop
※ exec (scripted): [view]▶ — Qt's app.exec() on the offscreen
※ platform. Each turn: rebuild the tree from the view quotation, draw
※ it, show the status line, read a line of stdin, dispatch it. A
※ glitch in a slot becomes a ✗ status message — the loop survives.
※ EOF (∅) or ◼ ends the loop.
[⇒ｖ 1⇒ｑ«»⇒ｓ[ｑ][ｖ⇒ｗ ｗ⌺ ｓ#0>[ｓ⍞«»⇒ｓ][]?⌨∂∅=[⌫0⇒ｑ][⇒ｌ ｌ#0>[[ⓓ][⍕«✗ »⇅⧺✎]⍥][]?]?]⟳]≔▶

※ ── the live platform: ⏵ ──────────────────────────────────────────
※ [view]⏵ is ▶ on a real terminal: it reads ⌥ events instead of
※ lines. ⇥/↓/→ and ↑/← move focus through the keymap in layout
※ order, ↵ or space activates the focused widget, printable keys
※ (single characters from code point 32 up that name no key) type
※ straight into a focused line edit (its slot runs on every keystroke
※ with the new text), ⌫ deletes, any other key — control chords and
※ navigation keys included — fires the widget with that mnemonic, and
※ a mouse click ⟨«⌖» x y⟩ lands on whatever drew the «(key)» under
※ the pointer. ^C (Ctrl-C), ⎋ or end of input ends the loop, as does
※ ◼ in a slot.

[∂1@«E»=[⌫][2@0ⓢ]?]≔ⓤ    ※ activate an entry: run its slot (edits just take focus)

※ type one character into the focused edit: slot gets value⧺char
[ｅｆ@∂1@«E»=[∂3@⥀⧺⇅2@1ⓢ][⌫⌫]?]≔ⓣ
※ backspace in the focused edit: slot gets the value minus one char
[ｅｆ@∂1@«E»=[∂3@∂#1-0⊔ 0⇅⊂⇅2@1ⓢ][⌫]?]≔ⓞ

※ hit-test: line ci ⓧ → the key of the widget drawn under character
※ index ci, or «». A widget's segment runs from its first glyph to the
※ «)» closing its «(key)» affordance; the frame's │, the two-space
※ gutters between hbox blocks and the padding after a segment belong
※ to no widget. So: ci must not sit on a │ or on a space next to a
※ space, a ) or a │ (or at either end of the line); the text from ci
※ to the next ) must hold no │ and no gutter; the key is what the
※ last ( before that ) opens.
[⇒ｚ⇒ｙ ｚ0<ｚｙ#≥∨[«»][ｙｚ@⇒ｔ ｔ«│»=ｔ« »=[ｚ0=ｚｙ#1-=∨[1][« )│»ｙｚ1-@∈« │»ｙｚ1+@∈∨]?][0]?∨[«»]
⋮[ｙｚｙ#⊂«)»⍷⇒ｔ ｔ¯1=[«»][ｔｚ+⇒ｔ ｙｚｔ1+⊂⇒ｕ ｕ«│»∈ｕ«  »∈∨[«»]
⋮[ｙ0ｔ⊂⌽«(»⍷⇒ｕ ｕ¯1=[«»][ｙｔｕ-ｔ⊂]?]?]?]?]?]≔ⓧ

※ dispatch one ⌥ event ｌ against entries ｅ, focus ｆ, frame ｄ
[ｌ∅=ｌ«^C»=∨ｌ«⎋»=∨[0⇒ｑ][
⋮ｌ«⇥»=ｌ«↓»=∨ｌ«→»=∨[ｅ#0>[ｆ1+ｅ#%⇒ｆ][]?][
⋮ｌ«↑»=ｌ«←»=∨[ｅ#0>[ｆｅ#+1-ｅ#%⇒ｆ][]?][
⋮ｌ⍙⇅⌫«list»=[ｌ2@1-⇒ｒ ｒ0≥ｒｄ#<∧[ｄｒ@ｌ1@1-ⓧ⇒ｋ ｋ«»≠[ｅ#⍸[ｅ⇅@⊃ｋ=]⌿⇒ｍ ｍ#0>[ｍ⊃⇒ｆ ｅｆ@∂1@«E»=[⌫][ⓤ]?][]?][]?][]?][
⋮ｌ«↵»=[ｅ#0>[ｅｆ@ⓤ][]?][
⋮ｌ«⌫»=[ｅ#0>[ⓞ][]?][
⋮ｌ« »=[ｅ#0>[ｅｆ@1@«E»=[« »ⓣ][ｅｆ@ⓤ]?][]?][
⋮ｅ#0>[ｅｆ@1@«E»=ｌ#1=∧[ｌ⌗32≥«↵⇥⌫⌦↑↓←→⇱⇲⇞⇟⎀⎋»ｌ∈¬∧][0]?][0]?[ｌⓣ][ｅ#⍸[ｅ⇅@⊃ｌ=]⌿⇒ｍ ｍ#0=[«? »ｌ⧺✎][ｍ⊃∂ｅ⇅@∂1@«E»=[⌫⇒ｆ][ⓤ⌫]?]?]?
⋮]?]?]?]?]?]?]?]≔ⓨ

※ the live loop: home+clear, draw with focus, status, ⌥, dispatch.
[⇒ｖ 1⇒ｑ«»⇒ｓ 0⇒ｆ[ｑ][ｖ⇒ｗ ｗⓚ⇒ｅ ｅ#0>[ｆｅ#%⇒ｆ ｅｆ@⊃][0⇒ｆ«»]?⇒ｉ
⋮27⍘«[H»⧺27⍘⧺«[2J»⧺⊸ ｗⓛ∂⇒ｄ[⍞]∀ ｓ#0>[ｓ⍞«»⇒ｓ][]?
⋮⌥⇒ｌ[ⓨ][⍕«✗ »⇅⧺✎]⍥]⟳]≔⏵
