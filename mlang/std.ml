※ The MLang standard library.
※
※ This file is written in MLang and woven into every program before its
※ boot section runs, by both engines. Every entry is an ordinary ≔
※ definition, so redefining a std sigil glitches with «already defined».
※ Names resolve late: definitions may reference each other in any order.
※ Library internals use fullwidth letters (ａ ｂ ｘ) as strand-locals —
※ treat those as reserved.

※ ── constants ─────────────────────────────────────────────────────
3.141592653589793≔π      ※ π                     π → 3.14159…
6.283185307179586≔τ      ※ τ = 2π                τ → 6.28318…
2.718281828459045≔ℯ      ※ Euler's number        ℯ → 2.71828…
«1e999»⍎≔∞               ※ positive infinity     ∞ → inf

※ ── numbers ───────────────────────────────────────────────────────
[∂0<[±][]?]≔∣            ※ absolute value        n∣ → |n|
[⊚⊚>[⇅][]?⌫]≔⊓           ※ minimum               a b⊓ → smaller
[⊚⊚<[⇅][]?⌫]≔⊔           ※ maximum               a b⊔ → larger
[⍸[1+]∵∏]≔‼              ※ factorial             n‼ → n!
[[∂0≠][⇅⊚%]⟳⌫]≔⟌         ※ greatest common divisor  a b⟌ → gcd

※ ── lists (and strings where it makes sense) ──────────────────────
[0[+]⍀]≔∑                ※ sum                   L∑ → total
[1[×]⍀]≔∏                ※ product               L∏ → total
[∂#⇅∑⇅÷]≔µ               ※ mean                  L µ → average (⟨⟩µ glitches)
[0@]≔⊃                   ※ head                  s⊃ → first item
[∂#1-@]≔⌷                ※ last                  s⌷ → last item
[∂#1⇅⊂]≔⍫                ※ tail                  s⍫ → all but first
[0⇅⊂]≔⊤                  ※ take                  s n⊤ → first n
[⊚#⊂]≔⊥                  ※ drop                  s n⊥ → all but first n
[⍋⌽]≔⍒                   ※ sort descending       s⍒ → sorted high→low
[⇒ｂ⇒ａ ａ#ｂ#⊓⍸[⇒ｘ⟨ａｘ@ｂｘ@⟩]∵]≔⍚   ※ zip   A B⍚ → ⟨⟨a b⟩ …⟩

※ ── text (case ops are ASCII-only) ────────────────────────────────
[«»⊆[⌗∂∂97≥⇅123<∧[32-][]?⍘]∵«»⊇]≔⇑   ※ uppercase   s⇑ → S
[«»⊆[⌗∂∂65≥⇅91<∧[32+][]?⍘]∵«»⊇]≔⇩    ※ lowercase   S⇩ → s
[« »⊆[«»≠]⌿]≔⍭           ※ words                 s⍭ → non-empty fields
[«⏎»⊆]≔⍖                 ※ lines                 s⍖ → split at newlines
