※ The Operator — the MLang JSON library.
※
※ Like the operator at the console, it reads the wire format of the
※ outside world and renders it as values — and types values back onto
※ the wire. Written in MLang and woven into a program's boot strand
※ automatically when the program references any Operator sigil it does
※ not define itself (SPEC §6.1). `mlang json` prints this source.
※
※ Public sigils:
※   «text»⒥ → v        parse JSON: objects → ⟨⟨«key» value⟩ …⟩ pair
※                      lists, arrays → lists, strings → strings,
※                      numbers → ints/floats, true/false → 1/0,
※                      null → ∅. Malformed input glitches
※                      «⒥ bad JSON at i» (0-based glyph index).
※   v⒮ → «text»        serialize: a non-empty list whose items are all
※                      ⟨«key» value⟩ pairs becomes an object, any other
※                      list an array; ∅ → null; 1/0 stay numbers.
※                      Quotations glitch.
※   obj «key»⒢ → v|∅   look a key up in a parsed object; ∅ if absent.
※   v ⟨steps…⟩⒫ → v|∅  dig a path: string steps look up object keys,
※                      int steps index arrays; ∅ as soon as a step
※                      has nothing to offer, and ∅ stays ∅.
※
※ Internals use the other parenthesized letters (⒜ ⒝ …) and the
※ fullwidth strand-locals ｊ ｐ — treat all of them as reserved, like
※ std's. \u escapes cover the Basic Multilingual Plane, and a high+low
※ surrogate escape pair decodes to the one code point it encodes.
※ Parse positions are 0-based indexes into the input string.

※ ── shared plumbing ───────────────────────────────────────────────
[⇅⟨⇅⟩⇅⧺]≔⒞               ※ cons                x L ⒞ → ⟨x …L⟩

※ ── the reader: «text»⒥ ──────────────────────────────────────────
※ Parse functions thread the position on the stack (i → v i′) so the
※ recursion through ⒱ is reentrant; the input string lives in ｊ.

※ skip whitespace: i → i′
[[∂ｊ#<[ｊ⊚@⌗33<][0]?][1+]⟳]≔⒲

※ literals: i → v i′
[ｊ⊚∂4+⊂«true»=[1⇅4+][«⒥ bad JSON at »⇅⍕⧺↯]?]≔⒳
[ｊ⊚∂5+⊂«false»=[0⇅5+][«⒥ bad JSON at »⇅⍕⧺↯]?]≔⒴
[ｊ⊚∂4+⊂«null»=[∅⇅4+][«⒥ bad JSON at »⇅⍕⧺↯]?]≔⒵

※ number: scan the maximal -+.eE0-9 run, let ⍎ judge it: i → n i′
[∂[∂ｊ#<[ｊ⊚@«-+.eE0123456789»⇅∈][0]?][1+]⟳
⋮⇅⊚ｊ⥀⥀⊂[⍎][⌫«⒥ bad JSON at »⇅⍕⧺↯]⍥⇅]≔⒩

※ four hex digits' value, ¯1 when malformed: «00e9»⒣ → 233
[∂#4=⊚1[«0123456789abcdefABCDEF»⇅∈∧]⍀∧[0[⇅16×⇅⌗∂96>[87-][∂64>[55-][48-]?]?+]⍀][⌫¯1]?]≔⒣

※ \u escape: i (at the backslash) → c i′. A high surrogate must be
※ followed by a \u low surrogate; the pair joins into one code point.
[∂2+∂4+ｊ⥀⥀⊂⒣∂0<[⌫«⒥ bad JSON at »⇅⍕⧺↯][]?∂55296<⊚57344≥∨[⍘⇅6+]
⋮[∂56320≥[⌫«⒥ bad JSON at »⇅⍕⧺↯][]?⇅∂6+∂2+ｊ⥀⥀⊂«\u»≠[«⒥ bad JSON at »⇅⍕⧺↯][]?
⋮∂8+∂4+ｊ⥀⥀⊂⒣∂56320≥⊚57344<∧¬[⌫«⒥ bad JSON at »⇅⍕⧺↯][]?⥀55296-1024×+9216+⍘⇅12+]?]≔⒰

※ escape: i (at the backslash) → c i′
[∂1+ｊ#≥[«⒥ bad JSON at »⇅⍕⧺↯][]?ｊ⊚1+@∂«u»=[⌫⒰]
⋮[∂«n»=[⌫10⍘][∂«t»=[⌫9⍘][∂«r»=[⌫13⍘][∂«b»=[⌫8⍘][∂«f»=[⌫12⍘][∂«"»=[][∂«\»=[][∂«/»=[][«⒥ bad JSON at »⥀⍕⧺⇅⌫↯]?]?]?]?]?]?]?]?⇅2+]?]≔⒝

※ string: i (at the opening ") → s i′; the growing text rides in ｐ.
※ Each turn scans ahead to the next " or \ and takes the run before it
※ in one slice, so an escape-free string costs a single pass.
[∂ｊ#≥[«⒥ bad JSON at »⇅⍕⧺↯][]?ｊ⊚@«"»≠[«⒥ bad JSON at »⇅⍕⧺↯][]?«»⇒ｐ1+
⋮[ｊ⊚ｊ#⊂∂«"»⍷∂¯1=[«⒥ bad JSON at »ｊ#⍕⧺↯][]?⇅«\»⍷∂¯1=[⌫ｊ#][]?⊚⊚>]
⋮[⇅⌫⊚+∂⥀⇅ｊ⥀⥀⊂ｐ⇅⧺⇒ｐ⒝⇅ｐ⇅⧺⇒ｐ]⟳
⋮⌫⊚+∂⥀⇅ｊ⥀⥀⊂ｐ⇅⧺⇅1+]≔⒯

※ object entries: acc i (past one k:v boundary) → acc′ i′ (at the })
[⒯⒲∂ｊ#≥[«⒥ bad JSON at »⇅⍕⧺↯][]?ｊ⊚@«:»≠[«⒥ bad JSON at »⇅⍕⧺↯][]?1+⒱
⋮⥀⥀⟨⟩⒞⒞⥀⇅⟨⇅⟩⧺⇅⒲∂ｊ#<[ｊ⊚@«,»=][0]?[1+⒲⒠][]?]≔⒠

※ object: i (at the {) → pairs i′
[1+⒲∂ｊ#≥[«⒥ bad JSON at »⇅⍕⧺↯][]?⟨⟩⇅ｊ⊚@«}»=[1+][⒠∂ｊ#≥[«⒥ bad JSON at »⇅⍕⧺↯][]?
⋮ｊ⊚@«}»=[1+][«⒥ bad JSON at »⇅⍕⧺↯]?]?]≔⒪

※ array entries: acc i → acc′ i′ (at the ])
[⒱⇅⟨⇅⟩⥀⇅⧺⇅⒲∂ｊ#<[ｊ⊚@«,»=][0]?[1+⒲⒡][]?]≔⒡

※ array: i (at the [) → items i′
[1+⒲∂ｊ#≥[«⒥ bad JSON at »⇅⍕⧺↯][]?⟨⟩⇅ｊ⊚@«]»=[1+][⒡∂ｊ#≥[«⒥ bad JSON at »⇅⍕⧺↯][]?
⋮ｊ⊚@«]»=[1+][«⒥ bad JSON at »⇅⍕⧺↯]?]?]≔⒜

※ any value: i → v i′
[⒲∂ｊ#≥[«⒥ bad JSON at »⇅⍕⧺↯][]?ｊ⊚@∂«"»=[⌫⒯][∂«{»=[⌫⒪][∂«[»=[⌫⒜]
⋮[∂«t»=[⌫⒳][∂«f»=[⌫⒴][∂«n»=[⌫⒵][⌫⒩]?]?]?]?]?]?]≔⒱

※ parse: the whole input must be one value plus whitespace
[⇒ｊ0⒱⒲∂ｊ#<[«⒥ bad JSON at »⇅⍕⧺↯][]?⌫]≔⒥

※ ── the writer: v⒮ ───────────────────────────────────────────────
[∂10<[48+][87+]?⍘]≔⒨     ※ nibble → hex glyph  n ⒨ → «0»…«f»

※ quote a string, escaping " \ and control characters
[«»⊆[∂«"»=[⌫«\"»][∂«\»=[⌫«\\»][∂10⍘=[⌫«\n»][∂9⍘=[⌫«\t»][∂13⍘=[⌫«\r»]
⋮[∂⌗32<[⌗∂16÷⌊⒨⇅16%⒨⧺«\u00»⇅⧺][]?]?]?]?]?]?]∵«»⊇«"»⇅⧺«"»⧺]≔⒬

※ is this a ⟨«key» value⟩ pair?  v → 1|0
[⍙«list»=[∂#2=[⊃⍙«str»=⇅⌫][⌫0]?][⌫0]?]≔⒤

※ serialize a list: all-pair lists become objects, the rest arrays
[∂#0>[∂1[⒤∧]⍀][0]?[[∂0@⒬«:»⧺⇅1@⒮⧺]∵«,»⊇«{»⇅⧺«}»⧺][[⒮]∵«,»⊇«[»⇅⧺«]»⧺]?]≔⒭

※ numbers: JSON has no inf or nan, so those glitch rather than lie
[⍙«∅»=[⌫«null»][⍙«str»=[⒬][⍙«list»=[⒭][⍙«quot»=[«⒮ cannot serialize a quotation»↯]
⋮[∂⍕⟨«inf»«¯inf»«nan»⟩⇅∈[«⒮ cannot serialize »⇅⍕⧺↯][]?∂0<[±⍕«-»⇅⧺][⍕]?]?]?]?]?]≔⒮

※ ── navigation ────────────────────────────────────────────────────
※ look up one key: obj «key»⒢ → v|∅ (∅ when obj is not a list at all)
[⇒ｐ⍙«list»=[[⍙«list»=[∂#2=[⊃ｐ=][⌫0]?][⌫0]?]⌿∂#0=[⌫∅][⊃1@]?][⌫∅]?]≔⒢

※ dig a path of keys and indices: v ⟨steps…⟩⒫ → v|∅. A string step
※ looks up a key, an int step indexes a list; any other pairing of
※ step and value has nothing to offer and answers ∅.
[[⊚∅=[⌫][⍙«str»=[⒢][⍙«int»=[⇅⍙«list»=[⇅∂0≥[⊚#⊚>[@][⌫⌫∅]?][⌫⌫∅]?][⌫⌫∅]?][⌫⌫∅]?]?]?]∀]≔⒫
