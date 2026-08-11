※ SUBLIMINAL — Sublime Text as a real window, written in MLang.
※
※ ⌸ opens a 960×600 pixel canvas and the whole editor is drawn with
※ two ops — ▦ rectangles and ⌶ text from the baked font — presented
※ with ⎙. On a desktop that canvas is an OS window and ⌥ reads its
※ keyboard and mouse; piped (or MLANG_HEADLESS=1) the same program
※ renders headless, each ⎙ printing the frame's hash, which is how the
※ recorded conformance golden pins every pixel of this editor.
※
※ The Sublime signature moves, in Mariana colors:
※   · a FOLDERS sidebar — ⌹ lists the working directory; click a file
※     to open it in a tab (or jump to its tab if it's already open)
※   · syntax highlighting, live as you type — comments, strings,
※     numbers, brackets, and every MLang sigil in its own color
※   · multiple cursors: ^D selects the word under the cursor, ^D again
※     adds its next occurrence (wrapping, skipping what's selected);
※     typing replaces every selection at once, ⎋ collapses to one
※   · the command palette: ^P, fuzzy-matched («dup» finds «duplicate
※     line»), plus Goto Anything — «:42» jumps to line 42, «#boom»
※     finds boom
※   · tabs with ● unsaved dots and ✕ close buttons; ^N new, ^E next,
※     ^W close, ^O opens into a new tab, arguments open as tabs
※   · a real minimap: every line's words in miniature on the right,
※     viewport shaded — click it to jump anywhere in the file
※   · line numbers, current-line highlight, auto-indent on ↵,
※     auto-closing [ ⟨ « pairs, ⇥ indents two spaces
※   · line surgery: ^X cut  ^C copy  ^V paste  ^K delete  ^J join
※     ^_ toggle ※ comment; sort, reverse, case, trim in the palette
※   · ^S save  ^F find (wraps)  ^G goto  ^Z undo  ^Y redo  ^Q quit
※
※   mlang run examples/sublime.ml [file …]      — or weld it:
※   mlang build examples/sublime.ml -o subl
※
※ Every buffer is an immutable list of lines; every edit is
※ slice-and-concat; undo is a list of ⟨buffer cursor⟩ snapshots and a
※ tab is the whole editor state packed into one list. A multi-cursor
※ selection is a single integer, row×Π+column (Π=2²⁰), so a flat ⍋
※ sorts the set and an edit replays left-to-right with per-line
※ offsets. Dispatch runs inside ⍥: a glitch becomes a status-bar
※ message, and the document survives by construction.
※
※ ── geometry (pixels, 8×16 glyph cells) and the Mariana palette ──
1048576≔Π                    ※ selection encoding: row×Π + column
187⍘≔◗                       ※ the » glyph — no string literal can hold it
34≔ρ 82≔ϣ                    ※ code pane: visible rows and columns
176≔⍺ 28≔ϐ 576≔ϒ 48≔⍵        ※ sidebar width, tab-bar height, status top, gutter
80≔∿ 274≔ϗ                   ※ minimap: width, strip rows (2 px per line)
3160129≔▩ 2831163≔▤ 3884366≔▣ 5134949≔▨      ※ editor bg, panels, raised, selection
14212841≔◇ 8095635≔⍪ 10923192≔⌭ 10078100≔⌮   ※ text, dim, comment, string
13014470≔⌯ 15490918≔⌰ 16363096≔◆             ※ sigil, bracket, accent
«∂⇅⌫⊚⥀≢+-×÷%^√⌊⌈±=≠<≤>≥∧∨¬⊻!?⟳⍣∵∀⌿⍀⍸#⧺@⊂⊆⊇⍕⍎⌗⍘⚡⋈⍳≣⌛⍥↯⍞⊸⌨⌥⍟⍙⌽⍋∈⍷⍇⍈⌂⍜⌸▦⌶⎙⌹≔⇒↥↧⇂⇈⇟⇉∅»≔⌇
[⇒ϝ⇒ϛ«»ϝ[ϛ⧺]⍣]≔J                          ※ repeat:  c n J → ccc…
[⇒ϻ∂⍕« »⧺ϻ⧺⇅1=[][«s»⧺]?]≔Z                ※ 3«line»Z → «3 lines»
[∂#[∂⌷«⏎»=[∂#1- 0⇅⊂][]?][]?]≔T            ※ trim one trailing ⏎
[⇒κ κ«a»≥ κ«z»≤∧ κ«A»≥ κ«Z»≤∧∨ κ«0»≥ κ«9»≤∧∨ κ«_»=∨]≔Ψ    ※ word character?
[h⟨⟨b y x⟩⟩⧺⇒h ⟨⟩⇒z]≔N                    ※ snapshot ⟨buffer cursor⟩
[h#[z⟨⟨b y x⟩⟩⧺⇒z h⌷∂0@⇒b ∂1@⇒y 2@⇒x h 0 h#1-⊂⇒h 1⇒d «undid»⇒m][«nothing to undo»⇒m]?]≔U
[z#[h⟨⟨b y x⟩⟩⧺⇒h z⌷∂0@⇒b ∂1@⇒y 2@⇒x z 0 z#1-⊂⇒z 1⇒d «redid»⇒m][«nothing to redo»⇒m]?]≔Y
[λ 0≠[σ⌷∂Π÷⌊⇒y Π% ω+ b y@#⊓⇒x 0⇒λ ⟨⟩⇒σ 0⇒ω ¯1⇒e][]?]≔Λ    ※ collapse to one cursor
[[n⍇T∂«»=[⌫⟨«»⟩][«⏎»⊆]?⇒b «opened »n⧺« · »⧺b#«line»Z⧺⇒m][⌫⟨«»⟩⇒b «new file »n⧺⇒m]⍥
0⇒y 0⇒x 0⇒v 0⇒d ⟨⟩⇒h ⟨⟩⇒z ¯1⇒e 0⇒λ ⟨⟩⇒σ 0⇒ω]≔O            ※ open file n
[b«⏎»⊇«⏎»⧺ n⍈ 0⇒d «saved »n⧺« · »⧺b#«line»Z⧺⇒m]≔V
※ A — modal one-line prompt over the status bar; edited text (∅ cancelled)
[⇒ϟ«»⇒u 1⇒f [f][F 0 ϒ 960 24 ▣▦ ϟ u⧺∂ 12 ϒ 4+ ◇⌶ #8× 12+ ϒ 4+ 2 16 ◆▦ ⎙ ⌥⇒a
a∅=[∅⇒u 0⇒f 0⇒g][a«↵»=[0⇒f][a«⎋»=[∅⇒u 0⇒f][a«⌫»=[u«»≠[u∂#1- 0⇅⊂⇒u][]?][a#1=[u a⧺⇒u][]?]?]?]?]?]⟳ u]≔A
[n«»=[«save as: »A ∂∅=[⌫«save cancelled»⇒m][∂«»=[⌫«save cancelled»⇒m][⇒n S]?]?][[V][⍕⇒m]⍥]?]≔S
[⇒γ y e≠[N y⇒e][]? b y@⇒t t 0 x⊂γ⧺ t x t#⊂⧺⇒t b 0 y⊂⟨t⟩⧺ b y 1+ b#⊂⧺⇒b x γ#+⇒x 1⇒d]≔I
[N b y@⇒t 0⇒j [j x<[t j@« »=][0]?][j 1+⇒j]⟳
b 0 y⊂⟨t 0 x⊂⟩⧺⟨« »j J t x t#⊂⧺⟩⧺ b y 1+ b#⊂⧺⇒b y 1+⇒y j⇒x 1⇒d ¯1⇒e]≔R
[x 0>[y e≠[N y⇒e][]? b y@⇒t t 0 x 1-⊂ t x t#⊂⧺⇒t b 0 y⊂⟨t⟩⧺ b y 1+ b#⊂⧺⇒b x 1-⇒x 1⇒d]
[y 0>[N b y 1-@⇒t t#⇒x t b y@⧺⇒t b 0 y 1-⊂⟨t⟩⧺ b y 1+ b#⊂⧺⇒b y 1-⇒y 1⇒d ¯1⇒e][]?]?]≔B
[b y@⇒t x t#<[y e≠[N y⇒e][]? t 0 x⊂ t x 1+ t#⊂⧺⇒t b 0 y⊂⟨t⟩⧺ b y 1+ b#⊂⧺⇒b 1⇒d]
[y b#1-<[N t b y 1+@⧺⇒t b 0 y⊂⟨t⟩⧺ b y 2+ b#⊂⧺⇒b 1⇒d ¯1⇒e][]?]?]≔D
※ H — doc line ι at pane row υ: gutter number, then the glyphs — syntax
※ colors from a tiny state machine, selection cells shaded first
[⇒υ⇒ι ϐ 4+ υ 16×+⇒ø
ι 1+⍕∂ ⍺ ⍵+ 8- ⇅#8×- ø ι y=[◇][⍪]?⌶
b ι@⇒t 0⇒ν «0»⇒χ 0⇒ς 0⇒ξ 0⇒ε
t«»⊆[⇒γ
ξ[⌭][ς[γ◗=[0⇒ς][]?⌮][γ«※»=[1⇒ξ⌭][γ««»=[1⇒ς⌮][
γ«0»≥ γ«9»≤∧ γ«¯»=∨ γ«.»=∨[◆][«[]⟨⟩»γ∈[⌰][⌇γ∈[⌯][◇]?]?]?]?]?]?]?⇒χ
ι Π× ε+⇒α
λ 1=[σ 0[⇒κ κ α≤ κ ω+ α>∧∨]⍀][λ 2=[σ α∈][0]?]?⇒β
ν ϣ<[⍺ ⍵+ ν 8×+⇒ϑ
β[ϑ ø 8 16 ▨▦][]?
γ« »≠[γ ϑ ø χ⌶][]? ν 1+⇒ν][]?
ε 1+⇒ε]∀
λ 1=[ι Π× t#+⇒α σ α∈ ν ϣ<∧[⍺ ⍵+ ν 8×+ ø 8 16 ▨▦][]?][]?]≔H
※ F — one full frame: bg, current line, code, carets, minimap, sidebar,
※ tabs, status bar. Callers ⎙ when the frame (plus overlays) is whole.
[y v<[y⇒v][]? y v ρ+ 1->[y ρ- 1+⇒v][]?
0 0 960 600 ▩▦
λ 0= y v- 0≥∧ y v- ρ<∧[⍺ ϐ 4+ y v- 16×+ 704 16 ▣▦][]?
ρ⍸[⇒q v q+ b#<[v q+ q H][]?]∀
y v- 0≥ y v- ρ<∧[⍺ ⍵+ x ϣ⊓ 8×+ ϐ 4+ y v- 16×+ 2 16 ◆▦][]?
λ 0≠[σ[⇒κ κ Π÷⌊⇒j j v≥ j v ρ+<∧[⍺ ⍵+ κ Π% λ 1=[ω+][]? ϣ⊓ 8×+ ϐ 4+ j v- 16×+ 2 16 ◆▦][]?]∀][]?
ϗ⍸[⇒κ b# ϗ≤[κ][κ b#× ϗ÷⌊]?⇒j
j b#<[j v≥ j v ρ+<∧[880 ϐ κ 2×+ ∿ 2 ▣▦][]?
b j@« »⊆⇒a 0⇒ν a[⇒γ γ#0> ν 72<∧[884 ν+ ϐ κ 2×+ 1+ γ# 72 ν-⊓ 1 ⍪▦][]? ν γ#1++⇒ν]∀][]?]∀
0 0 ⍺ 600 ▤▦ «FOLDERS»12 8 ⍪⌶
ϧ#⍸[⇒κ ϧ κ@⇒a 32 κ 20×+⇒j j ϒ 24-<[
a n=[0 j 2- ⍺ 20 ▣▦][]?
a∂#1-@«/»=[«▸ »a⧺∂#1-0⇅⊂ 10 j ⍪⌶][a 18 j a n=[◇][⍪]?⌶]?][]?]∀
⍺ 0 784 ϐ ▤▦
0⇒ν l#⍸[⇒κ κ i=[n d][l κ@∂0@⇅5@]?⇒ϙ⇒a
« »a«»=[«untitled»][a]?⧺ϙ[« ●»][«»]?⧺« »⧺⇒j j#8× 16+⇒ϑ
κ i=[⍺ ν+ 0 ϑ ϐ ▩▦][]?
j ⍺ ν+ 6 κ i=[◇][⍪]?⌶ «✕»⍺ ν+ ϑ 14-+ 6 ⍪⌶
ν ϑ+⇒ν]∀
0 ϒ 960 24 ▤▦
m«»=[λ 0≠[«^D adds the next — ⎋ collapses»][«^S save  ^P cmd  ^D multi  ^F find  ^Q quit»]?][m]?12 ϒ 4+ ⍪⌶
«Ln »y 1+⍕⧺« Col »⧺x 1+⍕⧺λ 0≠[«  »σ#⍕⧺« sel»⧺][«»]?⧺ 440 ϒ 4+ ⍪⌶
«MLang»800 ϒ 4+ ⍪⌶ «▲ SUBLIMINAL»860 ϒ 4+ ◆⌶
«»⇒m]≔F
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
※ P — the command palette, floating mid-screen: fuzzy commands, «:42» goto, «#text» find
[Λ«»⇒u 0⇒η 1⇒f
[f][F
Ω[∂0@ u Φ]⌿⇒ϑ
u 0 1⊂«:»= u 0 1⊂«#»=∨[⟨⟩⇒ϑ][]?
ϑ#0=[0⇒η][η ϑ#1-⊓⇒η]?
238 58 484 214 1843752▦ 240 60 480 28 ▣▦
« ⌕ »u⧺∂ 248 66 ◇⌶ #8× 248+ 66 2 16 ◆▦
8⍸[⇒κ 88 κ 22×+⇒j κ ϑ#<[
240 j 480 22 κ η=[▨][▤]?▦
ϑ κ@0@ 252 j 3+ κ η=[◇][⍪]?⌶][240 j 480 22 ▤▦]?]∀
⎙ ⌥⇒a
a∅=[0⇒f 0⇒g][a«⎋»=[0⇒f][a«↑»=[η 1- 0⊔⇒η][a«↓»=[η 1+⇒η][a«↵»=[0⇒f
u 0 1⊂«:»=[[u 1 u#⊂⍎ 1⊔ b#⊓ 1-⇒y 0⇒x «line »y 1+⍕⧺⇒m][⌫«not a line number»⇒m]⍥][
u 0 1⊂«#»=[u 1 u#⊂ Ξ][
ϑ#[ϑ η@1@!][«no match»⇒m]?]?]?
][a«⌫»=[u«»≠[u∂#1- 0⇅⊂⇒u 0⇒η][]?][a#1=[u a⧺⇒u 0⇒η][]?]?]?]?]?]?]?
]⟳]≔P
※ C — the keymap: one event in, one edit out. Mouse zones in pixels:
※ tabs, sidebar, minimap, text — each maps a click back through the
※ same arithmetic the frame was drawn with.
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
L⍙«list»=⇅⌫[L 1@⇒ζ L 2@⇒ξ
ξ ϐ< ζ ⍺≥∧[0⇒ν ¯1⇒ø l#⍸[⇒κ κ i=[n d][l κ@∂0@⇅5@]?⇒ϙ⇒a
« »a«»=[«untitled»][a]?⧺ϙ[« ●»][«»]?⧺« »⧺#8× 16+⇒ϑ
ζ ⍺ ν+≥ ζ ⍺ ν+ ϑ+<∧[ζ ⍺ ν+ ϑ+ 16-≥[κ 1000+][κ]?⇒ø][]? ν ϑ+⇒ν]∀
ø ¯1≠[ø 1000≥[ø 1000-⇒ζ ζ i≠[W ζ Γ][]?
l#1>[d w¬∧[1⇒w«unsaved — ✕ again closes»⇒m][l 0 i⊂ l i 1+ l#⊂⧺⇒l i l#1-⊓ Γ]?][«last tab — ^Q quits»⇒m]?][ø i≠[W ø Γ][]?]?][]?][
ζ ⍺< ξ 32≥∧[ξ 32- 20÷⌊⇒q q ϧ#<[ϧ q@⇒a a∂#1-@«/»≠[
¯1⇒ø l#⍸[⇒κ κ i=[n][l κ@0@]?a=[κ⇒ø][]?]∀
ø ¯1≠[ø i≠[W ø Γ][]?][⊕ a⇒n O]?][«a folder — files open on click»⇒m]?][]?][
ξ ϒ≥[][
ζ 880≥ ξ ϐ≥∧[Λ ξ ϐ- 2÷⌊⇒q b# ϗ≤[q][q b#× ϗ÷⌊]? b#1-⊓ 0⊔⇒y 0⇒x ¯1⇒e][
ξ ϐ≥[Λ ξ 32- 0⊔ 16÷⌊ v+ b#1-⊓⇒y ζ 224- 0⊔ 8÷⌊ b y@#⊓⇒x ¯1⇒e][]?]?]?]?]?][
L#1=[λ 0=[«]⟩»L∈ L ◗=∨ x b y@#<∧[b y@x@ L=][0]?[x 1+⇒x][L I L«[»=[«]»I x 1-⇒x][L«⟨»=[«⟩»I x 1-⇒x][L««»=[◗ I x 1-⇒x][]?]?]?]?][L Θ]?][]?
]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]?]≔C
⇊
960 600«SUBLIMINAL»⌸
⋮«.»⌹⇒a a[∂#1-@«/»=]⌿ a[∂#1-@«/»≠]⌿⧺⇒ϧ
⋮⟨«»⟩⇒b «»⇒n 0⇒y 0⇒x 0⇒v 0⇒d ⟨⟩⇒h ⟨⟩⇒z ¯1⇒e 0⇒w «»⇒m 1⇒g ∅⇒p 0⇒λ ⟨⟩⇒σ 0⇒ω 0⇒i ⟨⟨n b y x v d h z e⟩⟩⇒l
⋮⌂#[⌂⊃⇒n O ⌂⍫[⇒a ⊕ a⇒n O]∀ W 0 Γ][]?
⋮[g][F ⎙ ⌥⇒L L∅=[0⇒g][L«^Q»≠ L«^W»≠∧ L⍙⇅⌫«list»≠∧[0⇒w][]?[C][⍕⇒m]⍥]?]⟳
