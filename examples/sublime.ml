※ SUBLIMINAL — Sublime Text, jacked into the Matrix.
※
※ MatrixPad (examples/editor.ml) is Notepad; this is the power tool.
※ A code editor for MLang source with the Sublime Text signature moves:
※   · syntax highlighting, live as you type — comments, strings,
※     numbers, brackets, each in its own color
※   · multiple cursors: ^D selects the word under the cursor, ^D again
※     adds its next occurrence (wrapping, skipping what's selected);
※     typing replaces every selection at once, ⌫ deletes at every
※     cursor, ⎋ collapses back to one
※   · the command palette: ^P, fuzzy-matched («dup» finds «duplicate
※     line»), plus Goto Anything — «:42» jumps to line 42, «#boom»
※     finds boom
※   · tabs: ^N new, ^E next, ^W close, ^O opens into a new tab, and
※     every command-line argument opens as its own tab — click a tab
※     to switch
※   · a minimap: the document in miniature on the right edge with the
※     viewport shaded — click it to jump
※   · line numbers, auto-indent on ↵, auto-closing [ ⟨ « pairs
※     (typing the closer types over it), ⇥ indents two spaces
※   · line surgery: ^X cut  ^C copy  ^V paste a line; ^K delete,
※     ^J join, ^_ toggle ※ comment; sort, reverse, case and trim
※     live in the palette
※   · ^S save (asks a name)  ^F find (wraps)  ^G goto line  ^Z undo
※     ^Y redo  ^Q quit — warns once if anything is unsaved
※
※ In a real terminal the runtime jacks in raw keys, SGR mouse
※ reporting and the alternate screen for you:
※   mlang run examples/sublime.ml [file …]
※ or weld a standalone editor:  mlang build examples/sublime.ml -o subl
※
※ Three strands, the same event loop as MatrixPad:
※   strand 0  keyboard — ⌥ reads raw input events    → k
※   strand 1  the editor: buffers, tabs, dispatch    → o
※   strand 2  screen — ⊸ prints ANSI frames from o
※ Every buffer is an immutable list of lines; every edit is
※ slice-and-concat; undo is a list of ⟨buffer cursor⟩ snapshots and a
※ tab is the whole editor state packed into one list. A multi-cursor
※ selection is a single integer, row×Π+column (Π=2²⁰), so a flat ⍋
※ sorts the set and an edit replays left-to-right with per-line
※ offsets. Dispatch runs inside ⍥: a glitch becomes a status-bar
※ message, and the document survives by construction.
27⍘≔⎋                                     ※ the escape character
187⍘≔◗                                    ※ the » glyph — no string literal can hold it
1048576≔Π                                 ※ selection encoding: row×Π + column
[⎋«[»⧺⇅⧺]≔E                               ※ «7m»E → ␛[7m
[⇒ϝ⇒ϛ«»ϝ[ϛ⧺]⍣]≔J                          ※ repeat:  c n J → ccc…
[⇒ϡ∂# ϡ⇅- 0⊔« »⇅J⧺]≔K                     ※ pad:  s u K → s␣␣… (width u)
[⇒ϻ∂⍕« »⧺ϻ⧺⇅1=[][«s»⧺]?]≔Z                ※ 3«line»Z → «3 lines»
[⇒ϗ⇒ϟ⎋«[»⧺ϟ⍕⧺«;»⧺ϗ⍕⧺«H»⧺]≔G               ※ i j G → ␛[i;jH (goto)
[∂#[∂⌷«⏎»=[∂#1- 0⇅⊂][]?][]?]≔T            ※ trim one trailing ⏎
[⇒κ κ«a»≥ κ«z»≤∧ κ«A»≥ κ«Z»≤∧∨ κ«0»≥ κ«9»≤∧∨ κ«_»=∨]≔Ψ    ※ word character?
[h⟨⟨b y x⟩⟩⧺⇒h ⟨⟩⇒z]≔N                    ※ snapshot ⟨buffer cursor⟩
[h#[z⟨⟨b y x⟩⟩⧺⇒z h⌷∂0@⇒b ∂1@⇒y 2@⇒x h 0 h#1-⊂⇒h 1⇒d «undid»⇒m][«nothing to undo»⇒m]?]≔U
[z#[h⟨⟨b y x⟩⟩⧺⇒h z⌷∂0@⇒b ∂1@⇒y 2@⇒x z 0 z#1-⊂⇒z 1⇒d «redid»⇒m][«nothing to redo»⇒m]?]≔Y
[λ 0≠[σ⌷∂Π÷⌊⇒y Π% ω+ b y@#⊓⇒x 0⇒λ ⟨⟩⇒σ 0⇒ω ¯1⇒e][]?]≔Λ    ※ collapse to one cursor
[[n⍇T∂«»=[⌫⟨«»⟩][«⏎»⊆]?⇒b «opened »n⧺« · »⧺b#«line»Z⧺⇒m][⌫⟨«»⟩⇒b «new file »n⧺⇒m]⍥
0⇒y 0⇒x 0⇒v 0⇒d ⟨⟩⇒h ⟨⟩⇒z ¯1⇒e 0⇒λ ⟨⟩⇒σ 0⇒ω]≔O            ※ open file n
[b«⏎»⊇«⏎»⧺ n⍈ 0⇒d «saved »n⧺« · »⧺b#«line»Z⧺⇒m]≔V
[⇒t«»⇒u 1⇒f [f][r 1 G«7m»E⧺t⧺u⧺«K»E⧺«0m»E⧺↥o ↧k⇒a
a∅=[∅⇒u 0⇒f 0⇒g][a«↵»=[0⇒f][a«⎋»=[∅⇒u 0⇒f][a«⌫»=[u«»≠[u∂#1- 0⇅⊂⇒u][]?][a#1=[u a⧺⇒u][]?]?]?]?]?]⟳ u]≔A
[n«»=[«save as: »A ∂∅=[⌫«save cancelled»⇒m][∂«»=[⌫«save cancelled»⇒m][⇒n S]?]?][[V][⍕⇒m]⍥]?]≔S
[⇒γ y e≠[N y⇒e][]? b y@⇒t t 0 x⊂γ⧺ t x t#⊂⧺⇒t b 0 y⊂⟨t⟩⧺ b y 1+ b#⊂⧺⇒b x γ#+⇒x 1⇒d]≔I
[N b y@⇒t 0⇒j [j x<[t j@« »=][0]?][j 1+⇒j]⟳
b 0 y⊂⟨t 0 x⊂⟩⧺⟨« »j J t x t#⊂⧺⟩⧺ b y 1+ b#⊂⧺⇒b y 1+⇒y j⇒x 1⇒d ¯1⇒e]≔R
[x 0>[y e≠[N y⇒e][]? b y@⇒t t 0 x 1-⊂ t x t#⊂⧺⇒t b 0 y⊂⟨t⟩⧺ b y 1+ b#⊂⧺⇒b x 1-⇒x 1⇒d]
[y 0>[N b y 1-@⇒t t#⇒x t b y@⧺⇒t b 0 y 1-⊂⟨t⟩⧺ b y 1+ b#⊂⧺⇒b y 1-⇒y 1⇒d ¯1⇒e][]?]?]≔B
[b y@⇒t x t#<[y e≠[N y⇒e][]? t 0 x⊂ t x 1+ t#⊂⧺⇒t b 0 y⊂⟨t⟩⧺ b y 1+ b#⊂⧺⇒b 1⇒d]
[y b#1-<[N t b y 1+@⧺⇒t b 0 y⊂⟨t⟩⧺ b y 2+ b#⊂⧺⇒b 1⇒d ¯1⇒e][]?]?]≔D
※ H — one syntax-highlighted, selection-overlaid, width-τ text field for doc line ι
[⇒ι b ι@⇒t «»⇒ϕ 0⇒ν «0»⇒χ «»⇒δ 0⇒ς 0⇒ξ 0⇒ε
t«»⊆[⇒γ
ξ[«0;90»][ς[γ◗=[0⇒ς][]?«0;93»][γ«※»=[1⇒ξ«0;90»][γ««»=[1⇒ς«0;93»][γ« »=[χ][
γ«0»≥ γ«9»≤∧ γ«¯»=∨ γ«.»=∨[«0;95»][«[]⟨⟩»γ∈[«0;91»][«0»]?]?]?]?]?]?]?⇒χ
ι Π× ε+⇒α
λ 1=[σ 0[⇒κ κ α≤ κ ω+ α>∧∨]⍀][λ 2=[σ α∈][0]?]?⇒β
χ β[«;7»][«»]?⧺⇒ψ
ν τ<[ψ δ≠[⎋«[»⧺ψ⧺«m»⧺ϕ⇅⧺⇒ϕ ψ⇒δ][]? ϕ γ⧺⇒ϕ ν 1+⇒ν][]?
ε 1+⇒ε]∀
λ 2=[ι Π× t#+⇒α σ α∈ ν τ<∧[δ«0;7»≠[⎋«[0;7m»⧺ϕ⇅⧺⇒ϕ«0;7»⇒δ][]? ϕ« »⧺⇒ϕ ν 1+⇒ν][]?][]?
δ«»≠ δ«0»≠∧[ϕ⎋⧺«[0m»⧺⇒ϕ][]?
ϕ]≔H
※ M — the minimap strip for content row q, viewport shaded
[⇒q b# ρ≤[q][q b#× ρ÷⌊]?⇒j
j b#<[b j@⇒a «»⇒ϑ 8⍸[⇒κ a κ 4× κ 1+ 4×⊂⍭# 0>[«▄»][« »]?ϑ⇅⧺⇒ϑ]∀ ϑ][«        »]?
j v≥ j v ρ+<∧ j b#<∧[«90;100m»][«90m»]?E⇅⧺⎋⧺«[0m»⧺]≔M
※ F — draw one full frame: tab bar, gutter+code+minimap rows, status bar → o
[y v<[y⇒v][]? y v ρ+ 1->[y ρ- 1+⇒v][]?
«7m»E⇒ϑ 0⇒υ
l#⍸[⇒κ κ i=[n d][l κ@∂0@⇅5@]?⇒f⇒a
« »a«»=[«untitled»][a]?⧺f[« ●»][«»]?⧺« »⧺⇒j
υ j#+ 1+⇒υ
κ i=[«0m»E j⧺«7m»E⧺][j]?ϑ⇅⧺«▏»⧺⇒ϑ]∀
ϑ« »c υ- 0⊔ J⧺«0m»E⧺⇒ϑ
«H»E ϑ⧺
ρ⍸[⇒q q 2+ 1 G⧺
v q+ b#<[«90m»E⧺ v q+ 1+⍕⇒a « »3 a#- 0⊔ J⧺a⧺« »⧺ v q+ H⧺][]?
«K»E⧺ q 2+ c 8- G⧺«90m»E⧺«▏»⧺ q M⧺]∀
r 1 G⧺«7m»E⧺
m«»=[λ 0≠[«^D adds the next — ⎋ collapses»][«^S save  ^P cmd  ^D multi  ^F find  ^Q quit»]?][m]?
« ▏Ln »⧺y 1+⍕⧺« Col »⧺x 1+⍕⧺λ 0≠[« ▏»σ#⍕⧺« sel»⧺][«»]?⧺
«MLang ▏▲ SUBLIMINAL »⇒a 0 c a#-⊂ c a#- K a⧺⧺«0m»E⧺
y v- 2+ x τ⊓ 5+ G⧺«?25h»E⧺↥o «»⇒m]≔F
※ Θ — replace every selection with the string on the stack; selections become cursors
[⇒γ σ⍋⇒σ ⟨⟩⇒ν 0⇒δ ¯1⇒ε
σ[⇒κ κ Π÷⌊⇒j j ε≠[0⇒δ j⇒ε][]?
κ Π% δ+⇒q b j@⇒t
t 0 q⊂γ⧺ t q ω+ t#⊂⧺⇒t
b 0 j⊂⟨t⟩⧺ b j 1+ b#⊂⧺⇒b
δ γ# ω-+⇒δ
ν⟨j Π× q+ γ#+⟩⧺⇒ν]∀
ν⇒σ 0⇒ω 2⇒λ 1⇒d
σ⌷∂Π÷⌊⇒y Π% b y@#⊓⇒x]≔Θ
※ Δ — ^D: select the word under the cursor, then its next occurrence, and the next…
[λ 0=[
b y@⇒t
x t#<[t x@Ψ][0]?[x⇒ζ][x 0>[t x 1-@Ψ][0]?[x 1-⇒ζ][¯1⇒ζ]?]?
ζ ¯1=[«no word under cursor»⇒m][
ζ⇒η [η 0>[t η 1-@Ψ][0]?][η 1-⇒η]⟳
ζ 1+⇒ι [ι t#<[t ι@Ψ][0]?][ι 1+⇒ι]⟳
t η ι⊂⇒θ ι η-⇒ω N
⟨y Π× η+⟩⇒σ 1⇒λ ι⇒x «1 selection — ^D adds next»⇒m]?][
λ 1=[
σ⌷∂Π÷⌊⇒ι Π% 1+⇒ζ 0⇒η 0⇒ø
[ø 0= η b#≤∧][
b ι@⇒t t ζ t#⊂ θ⍷⇒q
q ¯1≠[ζ q+⇒q ι Π× q+⇒α σ α∈[q 1+⇒ζ][σ⟨α⟩⧺⇒σ 1⇒ø ι⇒y q ω+⇒x σ#⍕« selections»⧺⇒m]?]
[η 1+⇒η ι 1+ b#%⇒ι 0⇒ζ]?]⟳
ø 0=[«all occurrences selected»⇒m][]?][
«typing multi-cursor — ⎋ collapses»⇒m]?]?]≔Δ
※ Ξ — find the pattern on the stack, wrapping around the document
[⇒θ ¯1⇒ø b#⍸[⇒η ø ¯1=[y 1+ η+ b#%⇒ζ b ζ@ θ⍷ ¯1≠[ζ⇒ø][]?][]?]∀
ø ¯1≠[ø⇒y b y@ θ⍷ 0⊔⇒x «found: »θ⧺⇒m][«not found: »θ⧺⇒m]?]≔Ξ
※ Φ — fuzzy match: does the input (top) appear in order inside the label?
[⇩⇒γ ⇩ 0⇒β «»⊆[⇒κ β γ#<[κ γ β@=[β 1+⇒β][]?][]?]∀ β γ#=]≔Φ
※ tabs — W packs the live buffer into its slot; Γ loads slot κ; ⊕ opens a fresh tab
[l 0 i⊂⟨⟨n b y x v d h z e⟩⟩⧺ l i 1+ l#⊂⧺⇒l]≔W
[⇒κ l κ@∂0@⇒n ∂1@⇒b ∂2@⇒y ∂3@⇒x ∂4@⇒v ∂5@⇒d ∂6@⇒h ∂7@⇒z 8@⌫ κ⇒i ¯1⇒e 0⇒λ ⟨⟩⇒σ 0⇒ω 0⇒w]≔Γ
[W ⟨«»⟩⇒b «»⇒n 0⇒y 0⇒x 0⇒v 0⇒d ⟨⟩⇒h ⟨⟩⇒z ¯1⇒e 0⇒λ ⟨⟩⇒σ 0⇒ω 0⇒w «new tab»⇒m
l⟨⟨n b y x v d h z e⟩⟩⧺⇒l l#1-⇒i]≔⊕
※ Ω — the command palette's commands
⟨⟨«duplicate line»[N b y@⇒t b 0 y 1+⊂⟨t⟩⧺ b y 1+ b#⊂⧺⇒b y 1+⇒y 1⇒d ¯1⇒e «line duplicated»⇒m]⟩
⟨«delete line»[N b#1>[b 0 y⊂ b y 1+ b#⊂⧺⇒b y b#1-⊓⇒y][⟨«»⟩⇒b 0⇒y]? x b y@#⊓⇒x 1⇒d ¯1⇒e «line deleted»⇒m]⟩
⟨«join lines»[y b#1-<[N b y@⇒t b 0 y⊂⟨t« »⧺b y 1+@⧺⟩⧺ b y 2+ b#⊂⧺⇒b t#⇒x 1⇒d ¯1⇒e][«nothing to join»⇒m]?]⟩
⟨«toggle comment»[N b y@⇒t t 0 2⊂«※ »=[t 2 t#⊂][t 0 1⊂«※»=[t 1 t#⊂][«※ »t⧺]?]?⇒t
b 0 y⊂⟨t⟩⧺ b y 1+ b#⊂⧺⇒b x t#⊓⇒x 1⇒d ¯1⇒e]⟩
⟨«sort lines»[N b⍋⇒b 0⇒y 0⇒x 0⇒v 1⇒d ¯1⇒e «sorted»⇒m]⟩
⟨«reverse lines»[N b⌽⇒b 0⇒y 0⇒x 0⇒v 1⇒d ¯1⇒e «reversed»⇒m]⟩
⟨«upper case line»[N b y@⇑⇒t b 0 y⊂⟨t⟩⧺ b y 1+ b#⊂⧺⇒b 1⇒d ¯1⇒e]⟩
⟨«lower case line»[N b y@⇩⇒t b 0 y⊂⟨t⟩⧺ b y 1+ b#⊂⧺⇒b 1⇒d ¯1⇒e]⟩
⟨«trim trailing spaces»[N b[[∂#0>[∂∂#1-@« »=][0]?][∂#1- 0⇅⊂]⟳]∵⇒b x b y@#⊓⇒x 1⇒d ¯1⇒e «trimmed»⇒m]⟩
⟨«save file»[S]⟩
⟨«about»[«SUBLIMINAL — Sublime Text, jacked into the Matrix»⇒m]⟩⟩≔Ω
※ P — the command palette: fuzzy commands, «:42» goto, «#text» find
[Λ«»⇒u 0⇒η 1⇒f
[f][
Ω[∂0@ u Φ]⌿⇒ϑ
u 0 1⊂«:»= u 0 1⊂«#»=∨[⟨⟩⇒ϑ][]?
ϑ#0=[0⇒η][η ϑ#1-⊓⇒η]?
« ⌕ »u⧺«▏»⧺46 K⇒j 2 6 G«7m»E⧺j⧺«0m»E⧺
8⍸[⇒κ κ 3+ 6 G⧺ κ ϑ#<[κ η=[«1;7m»][«2m»]?E⧺« »ϑ κ@0@⧺« »⧺46 K⧺«0m»E⧺][«0m»E⧺« »46 J⧺]?]∀
2 u# 9+ G⧺«?25h»E⧺↥o
↧k⇒a
a∅=[0⇒f 0⇒g][a«⎋»=[0⇒f][a«↑»=[η 1- 0⊔⇒η][a«↓»=[η 1+⇒η][a«↵»=[0⇒f
u 0 1⊂«:»=[[u 1 u#⊂⍎ 1⊔ b#⊓ 1-⇒y 0⇒x «line »y 1+⍕⧺⇒m][⌫«not a line number»⇒m]⍥][
u 0 1⊂«#»=[u 1 u#⊂ Ξ][
ϑ#[ϑ η@1@!][«no match»⇒m]?]?]?
][a«⌫»=[u«»≠[u∂#1- 0⇅⊂⇒u 0⇒η][]?][a#1=[u a⧺⇒u 0⇒η][]?]?]?]?]?]?]?
]⟳]≔P
※ C — the keymap: one event in, one edit out
[L«↑»=[Λ y 0>[y 1-⇒y][]? x b y@#⊓⇒x ¯1⇒e][
L«↓»=[Λ y b#1-<[y 1+⇒y][]? x b y@#⊓⇒x ¯1⇒e][
L«←»=[Λ x 0>[x 1-⇒x][y 0>[y 1-⇒y b y@#⇒x][]?]?¯1⇒e][
L«→»=[Λ x b y@#<[x 1+⇒x][y b#1-<[y 1+⇒y 0⇒x][]?]?¯1⇒e][
L«⇱»=[Λ 0⇒x][
L«⇲»=[Λ b y@#⇒x][
L«⇞»=[Λ y ρ- 0⊔⇒y x b y@#⊓⇒x ¯1⇒e][
L«⇟»=[Λ y ρ+ b#1-⊓⇒y x b y@#⊓⇒x ¯1⇒e][
L«↵»=[Λ R][
L«⌫»=[λ 2=[σ 1[Π% 0>∧]⍀[σ[1-]∵⇒σ 1⇒ω«»Θ][«a cursor is at a line start»⇒m]?][λ 1=[«»Θ][B]?]?][
L«⌦»=[Λ D][
L«⇥»=[Λ«  »I][
L«^D»=[Δ][
L«⎋»=[Λ«»⇒m][
L«^S»=[S][
L«^O»=[«open: »A ∂∅=[⌫][∂«»=[⌫][⇒a ⊕ a⇒n O]?]?][
L«^P»=[P][
L«^F»=[«find: »A ∂∅=[⌫][∂«»=[⌫][Λ Ξ]?]?][
L«^G»=[«goto line: »A ∂∅=[⌫][∂«»=[⌫][Λ[⍎ 1⊔ b#⊓ 1-⇒y 0⇒x «line »y 1+⍕⧺⇒m][⌫«not a line number»⇒m]⍥]?]?][
L«^Z»=[Λ U][
L«^Y»=[Λ Y][
L«^N»=[⊕][
L«^E»=[l#1>[W i 1+ l#% Γ][«only one tab»⇒m]?][
L«^W»=[l#1>[d w¬∧[1⇒w«unsaved — ^W again closes»⇒m][l 0 i⊂ l i 1+ l#⊂⧺⇒l i l#1-⊓ Γ]?][«last tab — ^Q quits»⇒m]?][
L«^X»=[Λ N b y@⇒p b#1>[b 0 y⊂ b y 1+ b#⊂⧺⇒b y b#1-⊓⇒y][⟨«»⟩⇒b 0⇒y]? x b y@#⊓⇒x 1⇒d ¯1⇒e «line cut»⇒m][
L«^C»=[b y@⇒p «line copied»⇒m][
L«^V»=[Λ p⍙«∅»≠⇅⌫[N b 0 y⊂⟨p⟩⧺ b y b#⊂⧺⇒b y 1+⇒y 1⇒d ¯1⇒e «line pasted»⇒m][«clipboard empty»⇒m]?][
L«^K»=[Λ N b#1>[b 0 y⊂ b y 1+ b#⊂⧺⇒b y b#1-⊓⇒y][⟨«»⟩⇒b 0⇒y]? x b y@#⊓⇒x 1⇒d ¯1⇒e][
L«^J»=[Λ y b#1-<[N b y@⇒t b 0 y⊂⟨t« »⧺b y 1+@⧺⟩⧺ b y 2+ b#⊂⧺⇒b t#⇒x 1⇒d ¯1⇒e][]?][
L«^_»=[Λ N b y@⇒t t 0 2⊂«※ »=[t 2 t#⊂][t 0 1⊂«※»=[t 1 t#⊂][«※ »t⧺]?]?⇒t
b 0 y⊂⟨t⟩⧺ b y 1+ b#⊂⧺⇒b x t#⊓⇒x 1⇒d ¯1⇒e][
L«^Q»=[d⇒j l#⍸[⇒κ κ i≠[l κ@5@ j∨⇒j][]?]∀ j w¬∧[1⇒w«unsaved — ^Q again discards, ^S saves»⇒m][0⇒g]?][
L⍙«list»=⇅⌫[Λ L 1@ c 8->[L 2@ 2- 0⊔ ρ 1-⊓⇒q b# ρ≤[q][q b#× ρ÷⌊]? b#1-⊓ 0⊔⇒y 0⇒x ¯1⇒e][
L 2@ 1=[0⇒υ ¯1⇒ζ l#⍸[⇒κ κ i=[n d][l κ@∂0@⇅5@]?⇒f⇒a a«»=[«untitled»][a]?# 2+ f[2+][]?⇒j
L 1@ υ> L 1@ υ j+≤∧[κ⇒ζ][]? υ j+ 1+⇒υ]∀ ζ ¯1≠ ζ i≠∧[W ζ Γ][]?][
L 2@ 2- v+ b#1-⊓ 0⊔⇒y L 1@ 5- 0⊔ b y@#⊓⇒x ¯1⇒e]?]?][
L#1=[λ 0=[«]⟩»L∈ L ◗=∨ x b y@#<∧[b y@x@ L=][0]?[x 1+⇒x][L I L«[»=[«]»I x 1-⇒x][L«⟨»=[«⟩»I x 1-⇒x][L««»=[◗ I x 1-⇒x][]?]?]?]?][L Θ]?][]?
]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]≔C
⇊
[⌥∂∅≠][↥k]⟳⌫∅↥k
⍜∂0@⇒r 1@⇒c r 2-⇒ρ c 13-⇒τ ⟨«»⟩⇒b «»⇒n 0⇒y 0⇒x 0⇒v 0⇒d ⟨⟩⇒h ⟨⟩⇒z ¯1⇒e 0⇒w «»⇒m 1⇒g ∅⇒p 0⇒λ ⟨⟩⇒σ 0⇒ω 0⇒i ⟨⟨n b y x v d h z e⟩⟩⇒l
⋮⌂#[⌂⊃⇒n O ⌂⍫[⇒a ⊕ a⇒n O]∀ W 0 Γ][]? «2J»E↥o
⋮[g][F ↧k⇒L L∅=[0⇒g][L«^Q»≠ L«^W»≠∧[0⇒w][]?[C][⍕⇒m]⍥]?]⟳ «2J»E«H»E⧺↥o ∅↥o
[↧o∂∅≠][⊸]⟳⌫
