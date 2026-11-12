//! `mlang check`'s stack-effect pass: run the program on *shapes* instead
//! of values, and say where a glyph is certain to glitch.
//!
//! Dense syntax lets most one-token mistakes weave clean (bench/README),
//! and the runtime then announces them only when execution arrives. Two
//! of the commonest — a glyph that reaches for more than the stack holds,
//! and a glyph handed the wrong type — are visible in the text. This pass
//! walks every strand with an abstract stack (how deep, and what type
//! each slot is when that is known), follows both arms of `?`, iterates
//! loops to a fixed point, and inlines definitions at their call sites.
//!
//! The rule is **certain or silent**. A finding means: if execution gets
//! here, this glyph glitches. Wherever the pass cannot know — a value off
//! a channel, a quotation of unknown origin, arms that leave different
//! depths, a recursive definition — it forgets what it knew about the
//! stack rather than guess, and reports nothing past that point until it
//! knows again. A glitch inside a `⍥` body is the program's own business
//! and is not reported. What it cannot see at all: wrong *values* (an
//! off-by-one, a swapped operand of the same type) — those still run.

use crate::values::{Instr, Op, Pos, Value};
use crate::vm::{CompiledProgram, STD_ROWS};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

pub struct Finding {
    pub pos: Pos,
    pub msg: String,
}

/// What is known about one stack slot.
#[derive(Clone)]
enum T {
    /// Not known — and taken to be data, not a quotation: it came off a
    /// channel, out of a list, or from input.
    Any,
    /// Not known, and it may be a quotation: naming or applying it could
    /// do anything.
    Code,
    Num,
    Str,
    List,
    Nil,
    Quot(Arc<Vec<Instr>>),
    Mark,
}

impl T {
    fn of(v: &Value) -> T {
        match v {
            Value::Int(_) | Value::Big(_) | Value::Float(_) => T::Num,
            Value::Str(_) => T::Str,
            Value::List(_) => T::List,
            Value::Quot(q) => T::Quot(q.clone()),
            Value::Nil => T::Nil,
            Value::Mark => T::Mark,
        }
    }

    /// The type's name when it is definitely known.
    fn known(&self) -> Option<&'static str> {
        match self {
            T::Num => Some("a number"),
            T::Str => Some("a string"),
            T::List => Some("a list"),
            T::Nil => Some("∅"),
            T::Quot(_) => Some("a [quotation]"),
            T::Any | T::Code | T::Mark => None,
        }
    }

    fn same(&self, other: &T) -> bool {
        match (self, other) {
            (T::Quot(a), T::Quot(b)) => Arc::ptr_eq(a, b),
            _ => std::mem::discriminant(self) == std::mem::discriminant(other),
        }
    }

    fn join(&self, other: &T) -> T {
        if self.same(other) {
            return self.clone();
        }
        match (self, other) {
            (T::Quot(_) | T::Code, _) | (_, T::Quot(_) | T::Code) => T::Code,
            _ => T::Any,
        }
    }
}

/// What an operand must be.
#[derive(Clone, Copy)]
enum W {
    Any,
    Num,
    Str,
    List,
    Seq,
    Quot,
}

impl W {
    fn admits(self, t: &T) -> bool {
        match (self, t) {
            (_, T::Any | T::Code | T::Mark) | (W::Any, _) => true,
            (W::Num, T::Num) | (W::Str, T::Str) | (W::List, T::List) => true,
            (W::Seq, T::Str | T::List) | (W::Quot, T::Quot(_)) => true,
            _ => false,
        }
    }
    fn name(self) -> &'static str {
        match self {
            W::Any => "a value",
            W::Num => "a number",
            W::Str => "a string",
            W::List => "a list",
            W::Seq => "a list or string",
            W::Quot => "a [quotation]",
        }
    }
}

/// One path's knowledge at one point in the program.
#[derive(Clone)]
struct St {
    stack: Vec<T>,
    /// True while `stack` is the whole stack. Once false, anything may
    /// lie beneath it and reaching past it is not an underflow.
    floor: bool,
    /// Locals this path is known to have stored.
    locals: BTreeMap<char, T>,
    /// This path has glitched or raised: nothing after it runs.
    dead: bool,
}

impl St {
    fn fresh() -> St {
        St { stack: Vec::new(), floor: true, locals: BTreeMap::new(), dead: false }
    }

    /// Something ran whose effect is not known.
    fn forget(&mut self) {
        self.stack.clear();
        self.floor = false;
        self.locals.clear();
    }

    fn same(&self, o: &St) -> bool {
        self.dead == o.dead
            && self.floor == o.floor
            && self.stack.len() == o.stack.len()
            && self.stack.iter().zip(&o.stack).all(|(a, b)| a.same(b))
            && self.locals.len() == o.locals.len()
            && self.locals.iter().zip(&o.locals).all(|((k, a), (j, b))| k == j && a.same(b))
    }

    /// Where two paths meet: keep what both agree on.
    fn merge(a: St, b: St) -> St {
        if a.dead {
            return b;
        }
        if b.dead {
            return a;
        }
        let n = a.stack.len().min(b.stack.len());
        let floor = a.floor && b.floor && a.stack.len() == b.stack.len();
        let stack = a.stack[a.stack.len() - n..]
            .iter()
            .zip(&b.stack[b.stack.len() - n..])
            .map(|(x, y)| x.join(y))
            .collect();
        let locals = a
            .locals
            .iter()
            .filter_map(|(k, x)| b.locals.get(k).map(|y| (*k, x.join(y))))
            .collect();
        St { stack, floor, locals, dead: false }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Loop {
    While,
    Repeat,
    Map,
    Each,
    Pump,
}

/// Abstract steps before the pass stops looking. Definitions are inlined
/// at every call site, so a large program can cost far more than its
/// length; past this the pass keeps what it found and says no more.
const BUDGET: usize = 3_000_000;
const MAX_DEPTH: usize = 120;
const MAX_PASSES: usize = 8;

struct Ck {
    globals: HashMap<char, T>,
    /// Sigils bound somewhere other than a plain top-level boot `≔`, or
    /// more than once: their value is not known here.
    dynamic: HashSet<char>,
    /// Every sigil some `⇒` stores into, anywhere in the program.
    stored: HashSet<char>,
    out: Vec<Finding>,
    seen: HashSet<(Pos, String)>,
    quiet: u32,
    chain: Vec<(char, Pos)>,
    depth: usize,
    steps: usize,
    gave_up: bool,
    in_boot: bool,
}

fn is_lib(pos: Pos) -> bool {
    pos.0 >= STD_ROWS
}

impl Ck {
    /// This glyph glitches on this path. The path ends here either way;
    /// the finding is recorded unless a `⍥` (or a re-run of a loop body)
    /// has silenced it.
    fn fault(&mut self, st: &mut St, pos: Pos, msg: String) {
        st.dead = true;
        if self.quiet > 0 {
            return;
        }
        // A fault inside a library definition is reported where the
        // program called into the library.
        let (at, msg) = if is_lib(pos) {
            match self.chain.iter().find(|(_, site)| !is_lib(*site)) {
                Some(&(c, site)) => (site, format!("{c} glitches inside the library: {msg}")),
                None => return,
            }
        } else {
            match self.chain.last() {
                Some(&(c, site)) if !is_lib(site) => {
                    (pos, format!("{msg} — in {c}, called at {}:{}", site.0, site.1))
                }
                _ => (pos, msg),
            }
        };
        if self.seen.insert((at, msg.clone())) {
            self.out.push(Finding { pos: at, msg });
        }
    }

    /// Pop `want.len()` operands (bottom-most first), checking depth and
    /// then types. None means the glyph glitches here.
    fn take(&mut self, st: &mut St, pos: Pos, ch: char, want: &[W]) -> Option<Vec<T>> {
        let n = want.len();
        if st.floor && st.stack.len() < n {
            let held = match st.stack.len() {
                0 => "the stack is empty".to_string(),
                1 => "the stack holds 1".to_string(),
                k => format!("the stack holds {k}"),
            };
            let needs = if n == 1 { "a value".to_string() } else { format!("{n} values") };
            self.fault(st, pos, format!("{ch} needs {needs} and {held} here"));
            return None;
        }
        let have = st.stack.len().min(n);
        let mut args = vec![T::Any; n - have];
        args.extend(st.stack.drain(st.stack.len() - have..));
        for (t, w) in args.iter().zip(want) {
            if !w.admits(t) {
                let got = t.known().unwrap_or("something else");
                self.fault(st, pos, format!("{ch} expects {} and is given {got}", w.name()));
                return None;
            }
        }
        Some(args)
    }

    fn simple(&mut self, st: &mut St, pos: Pos, ch: char, want: &[W], gives: &[T]) {
        if self.take(st, pos, ch, want).is_some() {
            st.stack.extend(gives.iter().cloned());
        }
    }

    fn call(&mut self, q: &Arc<Vec<Instr>>, st: &mut St) {
        if self.depth >= MAX_DEPTH {
            st.forget();
            return;
        }
        self.depth += 1;
        self.run(q, st);
        self.depth -= 1;
    }

    /// Run a value the way `?` does: a quotation runs, anything else is
    /// pushed.
    fn pick(&mut self, t: T, st: &mut St) {
        match t {
            T::Quot(q) => self.call(&q, st),
            T::Code => st.forget(),
            other => st.stack.push(other),
        }
    }

    fn name(&mut self, c: char, pos: Pos, st: &mut St) {
        if let Some(t) = st.locals.get(&c).cloned() {
            return self.pick(t, st);
        }
        let global = self.globals.contains_key(&c) || self.dynamic.contains(&c);
        if !global {
            // A local this path has not stored yet (a caller may have),
            // or a name bound nowhere — which `check` reports separately.
            st.stack.push(T::Any);
            return;
        }
        // A name that is both a definition and, somewhere, a local: which
        // one this reference finds depends on what ran before.
        if self.stored.contains(&c) || self.dynamic.contains(&c) {
            return st.forget();
        }
        match self.globals.get(&c).cloned() {
            Some(T::Quot(q)) => {
                if self.chain.iter().any(|&(d, _)| d == c) {
                    return st.forget(); // recursion: effect not known
                }
                self.chain.push((c, pos));
                self.call(&q, st);
                self.chain.pop();
            }
            Some(t) => self.pick(t, st),
            None => st.forget(),
        }
    }

    /// One trip round a loop from `st`: the state the loop would exit
    /// with from here, and the state at the end of the body.
    fn trip(&mut self, kind: Loop, cond: Option<&Arc<Vec<Instr>>>, body: &Arc<Vec<Instr>>, pos: Pos, mut st: St) -> (St, St) {
        match kind {
            Loop::While => {
                self.call(cond.unwrap(), &mut st);
                if !st.dead && st.floor && st.stack.is_empty() {
                    self.fault(&mut st, pos, "⟳ needs its condition to leave a value and it leaves none".into());
                }
                st.stack.pop();
                let exit = st.clone();
                self.call(body, &mut st);
                (exit, st)
            }
            Loop::Repeat => {
                let exit = st.clone();
                self.call(body, &mut st);
                (exit, st)
            }
            Loop::Map | Loop::Pump => {
                let exit = st.clone();
                st.stack.push(T::Any);
                self.call(body, &mut st);
                if !st.dead && st.floor && st.stack.is_empty() {
                    let what = if kind == Loop::Pump { "⇉" } else { "the loop" };
                    self.fault(&mut st, pos, format!("{what} needs its body to leave a result and it leaves none"));
                }
                st.stack.pop();
                (exit, st)
            }
            Loop::Each => {
                let exit = st.clone();
                st.stack.push(T::Any);
                self.call(body, &mut st);
                (exit, st)
            }
        }
    }

    /// A loop runs its body any number of times. The first trip is taken
    /// from the entry state and reports what it finds; the state at the
    /// loop head is then widened until another trip no longer changes
    /// it, silently, and the exit state of that stable trip is the
    /// loop's result.
    fn looped(&mut self, kind: Loop, cond: Option<&Arc<Vec<Instr>>>, body: &Arc<Vec<Instr>>, pos: Pos, st: &mut St) {
        let (first_exit, end) = self.trip(kind, cond, body, pos, st.clone());
        if first_exit.dead {
            // the condition itself glitches: the loop never gets going
            *st = first_exit;
            return;
        }
        let mut head = St::merge(st.clone(), end);
        self.quiet += 1;
        let mut result = None;
        for _ in 0..MAX_PASSES {
            let (exit, end) = self.trip(kind, cond, body, pos, head.clone());
            let next = St::merge(head.clone(), end);
            if next.same(&head) {
                result = Some(exit);
                break;
            }
            head = next;
        }
        self.quiet -= 1;
        match result {
            Some(exit) => *st = exit,
            None => {
                st.forget();
            }
        }
    }

    fn run(&mut self, code: &[Instr], st: &mut St) {
        let mut i = 0;
        while i < code.len() {
            if st.dead || self.gave_up {
                return;
            }
            self.steps += 1;
            if self.steps > BUDGET {
                self.gave_up = true;
                return;
            }
            let pos = code[i].pos;
            match &code[i].op {
                Op::Push(v) => st.stack.push(T::of(v)),
                Op::LMark => st.stack.push(T::Mark),
                Op::LBuild => {
                    loop {
                        match st.stack.pop() {
                            Some(T::Mark) => break,
                            Some(_) => {}
                            None => {
                                if st.floor {
                                    self.fault(st, pos, "⟩ has no ⟨ open on the stack here".into());
                                }
                                break;
                            }
                        }
                    }
                    if !st.dead {
                        st.stack.push(T::List);
                    }
                }
                Op::Name(c) => self.name(*c, pos, st),
                // ⇂X [got] [none] ? — the one idiom whose two arms start
                // from different depths by design.
                Op::B('⇂', _, _) => {
                    let arms = match (code.get(i + 1), code.get(i + 2), code.get(i + 3)) {
                        (Some(t), Some(e), Some(q)) => match (&t.op, &e.op, &q.op) {
                            (Op::Push(Value::Quot(t)), Op::Push(Value::Quot(e)), Op::B('?', _, _)) => {
                                Some((t.clone(), e.clone()))
                            }
                            _ => None,
                        },
                        _ => None,
                    };
                    let (mut got, mut none) = (st.clone(), st.clone());
                    got.stack.push(T::Any);
                    match arms {
                        Some((t, e)) => {
                            self.call(&t, &mut got);
                            self.call(&e, &mut none);
                            i += 3;
                        }
                        None => {
                            got.stack.push(T::Num);
                            none.stack.push(T::Num);
                        }
                    }
                    *st = St::merge(got, none);
                }
                Op::B(ch, arg, _) => self.op(*ch, *arg, pos, st),
            }
            i += 1;
        }
    }

    fn op(&mut self, ch: char, arg: char, pos: Pos, st: &mut St) {
        use W::{Any as A, List as L, Num as N, Quot as Q, Seq, Str as S};
        match ch {
            // ── stack ──
            '∂' => {
                if let Some(a) = self.take(st, pos, ch, &[A]) {
                    st.stack.push(a[0].clone());
                    st.stack.push(a[0].clone());
                }
            }
            '⇅' => {
                if let Some(a) = self.take(st, pos, ch, &[A, A]) {
                    st.stack.push(a[1].clone());
                    st.stack.push(a[0].clone());
                }
            }
            '⌫' => self.simple(st, pos, ch, &[A], &[]),
            '⊚' => {
                if let Some(a) = self.take(st, pos, ch, &[A, A]) {
                    st.stack.extend([a[0].clone(), a[1].clone(), a[0].clone()]);
                }
            }
            '⥀' => {
                if let Some(a) = self.take(st, pos, ch, &[A, A, A]) {
                    st.stack.extend([a[1].clone(), a[2].clone(), a[0].clone()]);
                }
            }
            '≢' | '⍳' | '≣' | '⌚' => st.stack.push(T::Num),
            // ── arithmetic, comparison, logic ──
            '+' | '-' | '×' | '÷' | '%' | '^' => self.simple(st, pos, ch, &[N, N], &[T::Num]),
            '√' | '⌊' | '⌈' | '±' => self.simple(st, pos, ch, &[N], &[T::Num]),
            '=' | '≠' | '∧' | '∨' | '⊻' => self.simple(st, pos, ch, &[A, A], &[T::Num]),
            '¬' => self.simple(st, pos, ch, &[A], &[T::Num]),
            '<' | '≤' | '>' | '≥' => {
                if let Some(a) = self.take(st, pos, ch, &[A, A]) {
                    if let (Some(x), Some(y)) = (a[0].known(), a[1].known()) {
                        let ok = matches!((&a[0], &a[1]), (T::Num, T::Num) | (T::Str, T::Str));
                        if !ok {
                            let msg = format!("{ch} compares two numbers or two strings and is given {x} and {y}");
                            return self.fault(st, pos, msg);
                        }
                    }
                    st.stack.push(T::Num);
                }
            }
            // ── control ──
            '!' => {
                if let Some(a) = self.take(st, pos, ch, &[Q]) {
                    match &a[0] {
                        T::Quot(q) => self.call(q, st),
                        _ => st.forget(),
                    }
                }
            }
            '?' => {
                if let Some(a) = self.take(st, pos, ch, &[A, A, A]) {
                    let (mut yes, mut no) = (st.clone(), st.clone());
                    self.pick(a[1].clone(), &mut yes);
                    self.pick(a[2].clone(), &mut no);
                    *st = St::merge(yes, no);
                }
            }
            '⟳' => {
                if let Some(a) = self.take(st, pos, ch, &[Q, Q]) {
                    match (&a[0], &a[1]) {
                        (T::Quot(c), T::Quot(b)) => self.looped(Loop::While, Some(c), b, pos, st),
                        _ => st.forget(),
                    }
                }
            }
            '⍣' => {
                if let Some(a) = self.take(st, pos, ch, &[N, Q]) {
                    match &a[1] {
                        T::Quot(b) => self.looped(Loop::Repeat, None, b, pos, st),
                        _ => st.forget(),
                    }
                }
            }
            '∵' | '⌿' | '∀' => {
                if let Some(a) = self.take(st, pos, ch, &[Seq, Q]) {
                    match &a[1] {
                        T::Quot(f) => {
                            let kind = if ch == '∀' { Loop::Each } else { Loop::Map };
                            self.looped(kind, None, f, pos, st);
                            if ch != '∀' && !st.dead {
                                st.stack.push(T::List);
                            }
                        }
                        _ => st.forget(),
                    }
                }
            }
            '⍀' => {
                if let Some(a) = self.take(st, pos, ch, &[Seq, A, Q]) {
                    st.stack.push(a[1].clone());
                    match &a[2] {
                        T::Quot(f) => self.looped(Loop::Each, None, f, pos, st),
                        _ => st.forget(),
                    }
                }
            }
            '⍥' => {
                if let Some(a) = self.take(st, pos, ch, &[Q, Q]) {
                    let (T::Quot(body), T::Quot(handler)) = (&a[0], &a[1]) else {
                        return st.forget();
                    };
                    // A glitch in the body is caught: not ours to report.
                    let mut tried = st.clone();
                    self.quiet += 1;
                    self.call(body, &mut tried);
                    self.quiet -= 1;
                    // The handler starts from wherever the body was when
                    // it glitched — unknown — with the glitch value on top.
                    let mut caught = St { stack: vec![T::Any], floor: false, locals: BTreeMap::new(), dead: false };
                    self.call(handler, &mut caught);
                    *st = St::merge(tried, caught);
                }
            }
            '↯' => {
                if self.take(st, pos, ch, &[A]).is_some() {
                    st.dead = true;
                }
            }
            // ── sequences ──
            '⍸' => self.simple(st, pos, ch, &[N], &[T::List]),
            '#' => self.simple(st, pos, ch, &[Seq], &[T::Num]),
            '⧺' => {
                if let Some(a) = self.take(st, pos, ch, &[A, A]) {
                    let r = match (&a[0], &a[1]) {
                        (T::Str, T::Str | T::Any | T::Code) | (T::Any | T::Code, T::Str) => T::Str,
                        (T::List, T::List | T::Any | T::Code) | (T::Any | T::Code, T::List) => T::List,
                        (x, y) => match (x.known(), y.known()) {
                            (Some(x), Some(y)) => {
                                let msg = format!("⧺ joins two strings or two lists and is given {x} and {y}");
                                return self.fault(st, pos, msg);
                            }
                            (Some(x), None) | (None, Some(x)) => {
                                let msg = format!("⧺ joins two strings or two lists and is given {x}");
                                return self.fault(st, pos, msg);
                            }
                            (None, None) => T::Any,
                        },
                    };
                    st.stack.push(r);
                }
            }
            '@' => {
                if let Some(a) = self.take(st, pos, ch, &[Seq, N]) {
                    st.stack.push(if matches!(a[0], T::Str) { T::Str } else { T::Any });
                }
            }
            '⊂' => {
                if let Some(a) = self.take(st, pos, ch, &[Seq, N, N]) {
                    st.stack.push(a[0].clone());
                }
            }
            '⌽' | '⍋' => {
                if let Some(a) = self.take(st, pos, ch, &[Seq]) {
                    st.stack.push(a[0].clone());
                }
            }
            '⊆' => self.simple(st, pos, ch, &[S, S], &[T::List]),
            '⊇' => self.simple(st, pos, ch, &[Seq, S], &[T::Str]),
            '∈' | '⍷' => self.simple(st, pos, ch, &[Seq, A], &[T::Num]),
            '⍕' => self.simple(st, pos, ch, &[A], &[T::Str]),
            '⍎' | '⌗' => self.simple(st, pos, ch, &[S], &[T::Num]),
            '⍘' => self.simple(st, pos, ch, &[N], &[T::Str]),
            // Code that asks a value its type is about to branch on the
            // answer, and this pass follows both arms: the arm written
            // for strings would be "certain" to glitch on a list it will
            // never be given. So a value that has been asked is, from
            // here on, of no known type.
            '⍙' => {
                if self.take(st, pos, ch, &[A]).is_some() {
                    st.stack.push(T::Any);
                    st.stack.push(T::Str);
                }
            }
            // ── bindings ──
            '≔' => {
                if let Some(a) = self.take(st, pos, ch, &[A]) {
                    if self.in_boot && self.depth == 0 && !self.dynamic.contains(&arg) {
                        match &a[0] {
                            T::Any | T::Code | T::Mark => {
                                self.dynamic.insert(arg);
                            }
                            t => {
                                self.globals.insert(arg, t.clone());
                            }
                        }
                    }
                }
            }
            '⇒' => {
                if let Some(a) = self.take(st, pos, ch, &[A]) {
                    st.locals.insert(arg, a[0].clone());
                }
            }
            // ── strands & channels ──
            '↥' | '⍞' | '⊸' => self.simple(st, pos, ch, &[A], &[]),
            '↧' | '⎆' | '⌨' | '⌥' => st.stack.push(T::Any),
            '⇈' => self.simple(st, pos, ch, &[Seq], &[]),
            '⇟' | '⌂' | '⍜' => st.stack.push(T::List),
            '⇉' => {
                if let Some(a) = self.take(st, pos, ch, &[Q]) {
                    match &a[0] {
                        T::Quot(f) => self.looped(Loop::Pump, None, f, pos, st),
                        _ => st.forget(),
                    }
                }
            }
            '⚡' => {
                if let Some(a) = self.take(st, pos, ch, &[Q]) {
                    if let T::Quot(q) = &a[0] {
                        // a new strand: an empty stack and a copy of the locals
                        let mut child = St { locals: st.locals.clone(), ..St::fresh() };
                        self.call(q, &mut child);
                    }
                    st.stack.push(T::Num);
                }
            }
            '⋈' => self.simple(st, pos, ch, &[N], &[]),
            '⌛' | '⍟' | '⎙' => {}
            // ── I/O ──
            '⌸' => self.simple(st, pos, ch, &[N, N, S], &[]),
            '▦' => self.simple(st, pos, ch, &[N, N, N, N, N], &[]),
            '⌶' => self.simple(st, pos, ch, &[A, N, N, N], &[]),
            '⌹' => self.simple(st, pos, ch, &[S], &[T::List]),
            '⍇' | '⍆' => self.simple(st, pos, ch, &[S], &[T::Str]),
            '⍈' => self.simple(st, pos, ch, &[S, S], &[]),
            '⍄' => self.simple(st, pos, ch, &[L], &[T::Str]),
            '⍅' => self.simple(st, pos, ch, &[L], &[]),
            '⟐' => st.stack.push(T::Str),
            '⟡' => self.simple(st, pos, ch, &[S], &[T::List]),
            // a glyph this pass has no model for
            _ => st.forget(),
        }
    }
}

/// Find the bindings the pass cannot treat as fixed: every `⇒` target,
/// and every `≔` that is not a single top-level boot definition.
fn prescan(code: &[Instr], top_boot: bool, ck: &mut Ck, defined: &mut HashSet<char>) {
    for ins in code {
        match &ins.op {
            Op::B('⇒', a, _) => {
                ck.stored.insert(*a);
            }
            Op::B('≔', a, _) => {
                if !top_boot || !defined.insert(*a) {
                    ck.dynamic.insert(*a);
                }
            }
            Op::Push(Value::Quot(q)) => prescan(q, false, ck, defined),
            _ => {}
        }
    }
}

pub struct Report {
    /// Every glyph in the program's own text that is certain to glitch
    /// when execution reaches it, in source order.
    pub findings: Vec<Finding>,
    /// False when the pass ran out of budget before it had walked the
    /// whole program: the findings stand, but silence past them means
    /// nothing.
    pub complete: bool,
}

pub fn stack_effects(prog: &CompiledProgram) -> Report {
    let mut ck = Ck {
        globals: HashMap::new(),
        dynamic: HashSet::new(),
        stored: HashSet::new(),
        out: Vec::new(),
        seen: HashSet::new(),
        quiet: 0,
        chain: Vec::new(),
        depth: 0,
        steps: 0,
        gave_up: false,
        in_boot: true,
    };
    let mut defined = HashSet::new();
    prescan(&prog.boot, true, &mut ck, &mut defined);
    for (_, code) in &prog.strands {
        prescan(code, false, &mut ck, &mut defined);
    }
    ck.run(&prog.boot, &mut St::fresh());
    ck.in_boot = false;
    for (_, code) in &prog.strands {
        ck.run(code, &mut St::fresh());
    }
    ck.out.sort_by_key(|f| f.pos);
    Report { findings: ck.out, complete: !ck.gave_up }
}
