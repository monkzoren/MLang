※ edit — a text editor in the ed/edlin lineage: the punch-tape era's
※ Notepad, written in MLang. Line-oriented, file-backed, fault-tolerant:
※ a bad command or unreadable file prints «✗ …» and the session lives on.
※
※   a text     append a line            p         print buffer
※   i N text   insert before line N     d N       delete line N
※   c N text   change line N            /text     find lines
※   w [file]   write (save)             o file    open file
※   h          help                     q         quit (or end of input)
※
※ The buffer B and filename Ｆ are strand-locals of the editor loop;
※ the boot-defined commands reach them through dynamic scoping.
[⟨⇅⟩B⇅⧺⇒B]≔A
[∂1+⍕«│»⧺⇅B⇅@⧺⍞]≔N
[B#⍸[N]∀]≔P
[⇒ｇｇ⍭⊃⍎ｇ« »⍷∂0<[⌫ｇ#][]?1+ｇ⇅ｇ#⊂]≔G
[1-⇒ｎｎ0<ｎB#≥∨[«no such line»↯][]?]≔V
[1-⇒ｎｎ0<ｎB#>∨[«no such line»↯][]?]≔U
[V Bｎ⊤Bｎ1+⊥⧺⇒B]≔D
[⇅U⟨⇅⟩Bｎ⊤⇅⧺Bｎ⊥⧺⇒B]≔I
[⇅V⟨⇅⟩Bｎ⊤⇅⧺Bｎ1+⊥⧺⇒B]≔C
[⇒ｇB#⍸[∂B⇅@ｇ∈[N][⌫]?]∀]≔F
[∂#0=[⌫][⇒Ｆ]?B#0=[«»][B«⏎»⊇«⏎»⧺]?Ｆ⍈«wrote »Ｆ⧺⍞]≔W
[∂#0=[⌫][⇒Ｆ]?Ｆ⍇⍖∂#0>[∂⌷«»=[∂#1-⊤][]?][]?⇒B«opened »Ｆ⧺⍞]≔O
[«a text │ append   i N text │ insert   c N text │ change   d N │ delete»⍞«p │ print   /text │ find   w [file] │ save   o file │ open   q │ quit»⍞]≔H
⇊
⟨⟩⇒B«untitled»⇒Ｆ«MLang edit ⋅ h for help ⋅ q to quit»⍞
⋮[«* »⊸⌨∂∅=[⌫0][∂«q»=[⌫0][1]?]?]
⋮[⇒ｌｌ#0=[][[ｌ0 1⊂⇒ｃｌ2ｌ#⊂ｃ«a»=[A][ｃ«p»=[⌫P][ｃ«d»=[⍎D][ｃ«i»=[G I][ｃ«c»=[G C][ｃ«/»=[⌫ｌ1ｌ#⊂F][ｃ«w»=[W][ｃ«o»=[O][ｃ«h»=[⌫H][⌫«? unknown ⋅ h for help»⍞]?]?]?]?]?]?]?]?]?][«✗ »⇅⍕⧺⍞]⍥]?]⟳
