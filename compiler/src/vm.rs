//! The native MLang virtual machine — semantics identical to the reference
//! implementation, including the deterministic scheduler (SLICE = 8) and
//! every diagnostic message. Same program + same input ⇒ same bytes out.

use crate::forms::Program;
use crate::loom::{self, BootShape, Loom, StrandAction};
use crate::lex::{lex_strand, LoadError};
use crate::values::{fmt, fmt_i64, truthy, type_name, val_eq, Instr, Op, Pos, Value};
use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{BufRead, Write};
use std::sync::Arc;

const SLICE: usize = 8;
/// Resource limits. Exhausting a resource is a glitch like any other
/// fault — never an allocation abort or a stack overflow — because a
/// strand that dies with coordinates is something an agent can act on.
const MAX_RANGE: i64 = 100_000_000;
const MAX_POW_BITS: u64 = 1 << 30;
pub const MAX_FRAMES: usize = 200_000;

#[derive(Clone, Copy, PartialEq)]
pub enum Status {
    Run,
    Blocked,
    Done,
    Dead,
}

#[derive(Clone, Copy, PartialEq)]
pub enum BlockOn {
    Chan(char),
    Strand(i64),
    Stdin,
}

pub enum Sig {
    Block(BlockOn, Pos),
    Yield,
    Glitch(Value, Pos),
}

type R<T> = Result<T, Sig>;

fn glitch<T>(msg: impl Into<String>, pos: Pos) -> R<T> {
    Err(Sig::Glitch(Value::str(msg.into()), pos))
}

#[derive(Clone, Copy, PartialEq)]
enum IterMode {
    Map,
    Each,
    Filter,
    Fold,
}

enum Frame {
    CF {
        code: Arc<Vec<Instr>>,
        ip: usize,
    },
    While {
        cond: Arc<Vec<Instr>>,
        body: Arc<Vec<Instr>>,
        phase: u8,
        pos: Pos,
    },
    Repeat {
        left: i64,
        body: Arc<Vec<Instr>>,
    },
    Iter {
        items: Arc<Vec<Value>>,
        i: usize,
        f: Arc<Vec<Instr>>,
        mode: IterMode,
        out: Vec<Value>,
        awaiting: bool,
        pos: Pos,
    },
    Try {
        handler: Arc<Vec<Instr>>,
        depth: usize,
    },
    Drain {
        chan: char,
        out: Vec<Value>,
        pos: Pos,
    },
    Pump {
        src: char,
        dst: char,
        f: Arc<Vec<Instr>>,
        phase: u8,
        pos: Pos,
    },
}

fn cf(code: Arc<Vec<Instr>>) -> Frame {
    Frame::CF { code, ip: 0 }
}

/// The frames a strand re-woven onto `code` resumes with: parked at the
/// iteration boundary of the code's first top-level loop (`[c][b]⟳` or
/// `[f]⇉xy`), so the loop's own prelude — the initialization that ran
/// once on the old code — is not run again. Code with no top-level loop
/// starts from its beginning.
fn frames_at_loop(code: &Arc<Vec<Instr>>) -> Vec<Frame> {
    for (k, i) in code.iter().enumerate() {
        match &i.op {
            Op::B('⟳', _, _) if k >= 2 => {
                if let (Op::Push(Value::Quot(c)), Op::Push(Value::Quot(b))) =
                    (&code[k - 2].op, &code[k - 1].op)
                {
                    return vec![
                        Frame::CF { code: code.clone(), ip: k + 1 },
                        Frame::While { cond: c.clone(), body: b.clone(), phase: 0, pos: i.pos },
                    ];
                }
            }
            Op::B('⇉', src, dst) if k >= 1 => {
                if let Op::Push(Value::Quot(f)) = &code[k - 1].op {
                    return vec![
                        Frame::CF { code: code.clone(), ip: k + 1 },
                        Frame::Pump { src: *src, dst: *dst, f: f.clone(), phase: 0, pos: i.pos },
                    ];
                }
            }
            _ => {}
        }
    }
    vec![cf(code.clone())]
}

pub struct Strand {
    pub sid: i64,
    pub label: String,
    frames: Vec<Frame>,
    stack: Vec<Value>,
    // Strands hold a handful of single-glyph locals; a linear scan beats
    // hashing at this size, and name references are the hottest path.
    locals: Vec<(char, Value)>,
    pub status: Status,
    pub block: Option<(BlockOn, Pos)>,
    pub glitch: Option<(Value, Pos)>,
    /// The call chain as it stood when the fatal glitch was raised —
    /// captured there because `catch` unwinds the frames while hunting for
    /// a handler, and an uncaught glitch therefore reports from an empty
    /// frame stack.
    pub glitch_chain: Vec<(char, Pos)>,
    /// Active named-definition calls: (sigil, call site, frame depth just
    /// after the call's frame was pushed). Entries whose depth exceeds the
    /// live frame count are stale and pruned before each push — which is
    /// sound because a completed call always returns through a shallower
    /// depth before any new call is made. Drives the fault report's call
    /// chain, so a glitch inside a definition names its caller.
    calls: Vec<(char, Pos, usize)>,
    /// Open ⟨ marks: (stack index of the mark, its position). A strand
    /// that finishes with one still open glitches there instead of
    /// silently ending with an unfinished list on its stack.
    marks: Vec<(usize, Pos)>,
    /// The code this strand was started (or last re-woven) on — its
    /// identity for matching against a patched source.
    origin: Arc<Vec<Instr>>,
    /// A hot patch waiting for this strand's next seam.
    pending: Option<Swap>,
}

/// What a hot patch asks of a running strand.
pub enum Swap {
    /// Continue on new code (with the new label) at the next seam, after
    /// running the migration, if any, on the old stack and locals.
    Replace(Arc<Vec<Instr>>, String, Option<Arc<Vec<Instr>>>),
    /// Finish at the next seam.
    Retire,
}

impl Strand {
    pub(crate) fn new(sid: i64, label: String, code: Arc<Vec<Instr>>, locals: Vec<(char, Value)>) -> Self {
        Strand {
            sid,
            label,
            frames: vec![cf(code.clone())],
            stack: Vec::new(),
            locals,
            status: Status::Run,
            block: None,
            glitch: None,
            glitch_chain: Vec::new(),
            calls: Vec::new(),
            marks: Vec::new(),
            origin: code,
            pending: None,
        }
    }

    fn placeholder() -> Self {
        Strand {
            sid: i64::MIN,
            label: String::new(),
            frames: Vec::new(),
            stack: Vec::new(),
            locals: Vec::new(),
            status: Status::Run,
            block: None,
            glitch: None,
            glitch_chain: Vec::new(),
            calls: Vec::new(),
            marks: Vec::new(),
            origin: Arc::new(Vec::new()),
            pending: None,
        }
    }

    /// Is this strand at a seam — a point where a hot patch can take
    /// over without leaving half an iteration behind? Seams are the
    /// boundary between two iterations of the strand's outermost loop
    /// (about to test the condition, or parked at the very first
    /// instruction of the body or condition — which is how a server
    /// waits for its next request), a pump between two values, a strand
    /// that has not started, and a strand that died.
    fn at_seam(&self) -> bool {
        match self.status {
            Status::Dead => return true,
            Status::Done => return false,
            _ => {}
        }
        match self.frames.as_slice() {
            [] => false,
            [Frame::CF { ip, .. }] => *ip == 0,
            [Frame::CF { .. }, Frame::While { phase: 0, .. }] => true,
            [Frame::CF { .. }, Frame::While { .. }, Frame::CF { ip: 0, .. }] => true,
            [Frame::CF { .. }, Frame::Pump { phase: 0, .. }] => true,
            _ => false,
        }
    }

    /// Take the pending patch at a seam: rebuild the frames on the new
    /// code, resuming inside its outermost loop, with stack and locals
    /// intact. A dead strand comes back to life with an empty stack.
    fn swap_in(&mut self) {
        let Some(swap) = self.pending.take() else { return };
        match swap {
            Swap::Retire => {
                self.frames.clear();
                self.status = Status::Done;
                self.block = None;
            }
            Swap::Replace(code, label, migrate) => {
                if self.status == Status::Dead {
                    self.stack.clear();
                    self.marks.clear();
                    self.glitch = None;
                    self.glitch_chain.clear();
                }
                self.frames = frames_at_loop(&code);
                // The migration runs first — on top of the new frames, so
                // when it returns the new loop is what continues.
                if let Some(m) = migrate {
                    self.frames.push(cf(m));
                }
                self.calls.clear();
                self.origin = code;
                self.label = label;
                self.status = Status::Run;
                self.block = None;
            }
        }
    }

    fn push(&mut self, v: Value) {
        self.stack.push(v);
    }

    /// The strand's stack, for fault reports.
    pub fn stack_view(&self) -> &[Value] {
        &self.stack
    }

    fn local_get(&self, c: char) -> Option<&Value> {
        self.locals.iter().find(|(k, _)| *k == c).map(|(_, v)| v)
    }

    fn local_set(&mut self, c: char, v: Value) {
        match self.locals.iter_mut().find(|(k, _)| *k == c) {
            Some(slot) => slot.1 = v,
            None => self.locals.push((c, v)),
        }
    }

    /// Unbounded recursion is a fault with coordinates, not a slow death
    /// by memory exhaustion: past MAX_FRAMES the call glitches.
    fn check_depth(&self, pos: Pos) -> R<()> {
        if self.frames.len() >= MAX_FRAMES {
            return glitch(
                format!("call depth exceeds {MAX_FRAMES} frames — unbounded recursion?"),
                pos,
            );
        }
        Ok(())
    }

    fn pop(&mut self, pos: Pos, what: &str) -> R<Value> {
        match self.stack.pop() {
            Some(v) => Ok(v),
            None => glitch(format!("stack underflow — needed {what}"), pos),
        }
    }

    /// Pop for an operation that can name itself: the report says which
    /// glyph went hungry and how deep the stack actually was.
    fn pop_for(&mut self, pos: Pos, op: &str, what: &str) -> R<Value> {
        match self.stack.pop() {
            Some(v) => Ok(v),
            None => glitch(
                format!("stack underflow — {op} needed {what} but the stack was empty"),
                pos,
            ),
        }
    }

    fn pop_any(&mut self, pos: Pos) -> R<Value> {
        self.pop(pos, "a value")
    }

    fn pop_num(&mut self, pos: Pos, op: &str) -> R<Value> {
        let v = self.pop_for(pos, op, "a number")?;
        if !v.is_num() {
            return glitch(format!("{op} expects numbers, got {}", type_name(&v)), pos);
        }
        Ok(v)
    }

    fn pop_i64(&mut self, pos: Pos, op: &str) -> R<i64> {
        let v = self.pop_num(pos, op)?;
        Ok(match v {
            Value::Int(i) => i,
            Value::Big(b) => b.to_i64().unwrap_or(i64::MAX),
            Value::Float(f) => f as i64,
            _ => unreachable!(),
        })
    }

    fn pop_quot(&mut self, pos: Pos, op: &str) -> R<Arc<Vec<Instr>>> {
        let v = self.pop_for(pos, op, "a [quotation]")?;
        match v {
            Value::Quot(q) => Ok(q),
            _ => glitch(
                format!("{op} expects a [quotation], got {}", type_name(&v)),
                pos,
            ),
        }
    }

    /// A list, or a string exploded into 1-char strings.
    fn pop_seq(&mut self, pos: Pos, op: &str) -> R<Arc<Vec<Value>>> {
        let v = self.pop(pos, "a list or string")?;
        match v {
            Value::Str(s) => Ok(Arc::new(
                s.chars().map(|c| Value::str(c.to_string())).collect(),
            )),
            Value::List(l) => Ok(l),
            _ => glitch(
                format!("{op} expects a list or string, got {}", type_name(&v)),
                pos,
            ),
        }
    }

    /// The named calls still on the frame stack, innermost last. Stale
    /// entries (whose frame has already returned) are dropped.
    fn live_calls(&self) -> Vec<(char, Pos)> {
        self.calls
            .iter()
            .filter(|&&(_, _, depth)| depth <= self.frames.len())
            .map(|&(c, pos, _)| (c, pos))
            .collect()
    }

    fn catch(&mut self, value: Value) -> bool {
        while let Some(top) = self.frames.last() {
            if let Frame::Try { handler, depth } = top {
                let (handler, depth) = (handler.clone(), *depth);
                self.stack.truncate(depth);
                self.marks.retain(|&(i, _)| i < depth);
                self.frames.pop();
                self.push(value);
                self.frames.push(cf(handler));
                return true;
            }
            self.frames.pop();
        }
        false
    }
}

// ── numeric helpers ────────────────────────────────────────────────────
/// Both operands are language-level ints, as BigInts (borrowed where the
/// value is already big) for the arbitrary-precision path.
fn both_big<'a>(a: &'a Value, b: &'a Value) -> Option<(Cow<'a, BigInt>, Cow<'a, BigInt>)> {
    let big = |v: &'a Value| -> Option<Cow<'a, BigInt>> {
        match v {
            Value::Int(i) => Some(Cow::Owned(BigInt::from(*i))),
            Value::Big(b) => Some(Cow::Borrowed(&**b)),
            _ => None,
        }
    };
    Some((big(a)?, big(b)?))
}

fn arith(op: char, a: &Value, b: &Value, pos: Pos) -> R<Value> {
    // i64 fast path; overflow (and i64::MIN edge cases, where checked ops
    // return None) falls through to the arbitrary-precision path below.
    if let (Value::Int(x), Value::Int(y)) = (a, b) {
        let (x, y) = (*x, *y);
        match op {
            '+' => {
                if let Some(r) = x.checked_add(y) {
                    return Ok(Value::Int(r));
                }
            }
            '-' => {
                if let Some(r) = x.checked_sub(y) {
                    return Ok(Value::Int(r));
                }
            }
            '×' => {
                if let Some(r) = x.checked_mul(y) {
                    return Ok(Value::Int(r));
                }
            }
            '÷' => {
                if y == 0 {
                    return glitch("÷ by zero", pos);
                }
                match x.checked_rem(y) {
                    Some(0) => return Ok(Value::Int(x / y)),
                    Some(_) => return Ok(Value::Float(x as f64 / y as f64)),
                    None => {}
                }
            }
            '%' => {
                if y == 0 {
                    return glitch("% by zero", pos);
                }
                if let Some(mut r) = x.checked_rem(y) {
                    if r != 0 && (r < 0) != (y < 0) {
                        r += y;
                    }
                    return Ok(Value::Int(r));
                }
            }
            // ^ keeps its semantics in one place, on the big path
            '^' => {}
            _ => unreachable!(),
        }
    }
    if let Some((x, y)) = both_big(a, b) {
        let (x, y) = (x.as_ref(), y.as_ref());
        return Ok(match op {
            '+' => Value::from_big(x + y),
            '-' => Value::from_big(x - y),
            '×' => Value::from_big(x * y),
            '÷' => {
                if y.is_zero() {
                    return glitch("÷ by zero", pos);
                }
                if (x % y).is_zero() {
                    Value::from_big(x / y)
                } else {
                    Value::Float(a.as_f64().unwrap() / b.as_f64().unwrap())
                }
            }
            '%' => {
                if y.is_zero() {
                    return glitch("% by zero", pos);
                }
                let mut r = x % y;
                if !r.is_zero() && (r.is_negative() != y.is_negative()) {
                    r += y;
                }
                Value::from_big(r)
            }
            '^' => {
                if y.is_negative() {
                    Value::Float(a.as_f64().unwrap().powf(b.as_f64().unwrap()))
                } else {
                    // The result has about bits(x)·e bits; refuse to build one
                    // that would exhaust memory — a glitch, never an abort.
                    match y.to_u32() {
                        Some(e) if x.bits().saturating_mul(u64::from(e)) <= MAX_POW_BITS => {
                            Value::from_big(x.pow(e))
                        }
                        _ => return glitch("^ result too large", pos),
                    }
                }
            }
            _ => unreachable!(),
        });
    }
    let (x, y) = (a.as_f64().unwrap(), b.as_f64().unwrap());
    Ok(match op {
        '+' => Value::Float(x + y),
        '-' => Value::Float(x - y),
        '×' => Value::Float(x * y),
        '÷' => {
            if y == 0.0 {
                return glitch("÷ by zero", pos);
            }
            Value::Float(x / y)
        }
        '%' => {
            if y == 0.0 {
                return glitch("% by zero", pos);
            }
            let mut r = x % y;
            if r != 0.0 && (r < 0.0) != (y < 0.0) {
                r += y;
            }
            Value::Float(r)
        }
        '^' => Value::Float(x.powf(y)),
        _ => unreachable!(),
    })
}

fn num_cmp(a: &Value, b: &Value) -> std::cmp::Ordering {
    if let (Value::Int(x), Value::Int(y)) = (a, b) {
        return x.cmp(y);
    }
    if matches!(a, Value::Big(_)) || matches!(b, Value::Big(_)) {
        if let Some((x, y)) = both_big(a, b) {
            return x.as_ref().cmp(y.as_ref());
        }
    }
    a.as_f64()
        .unwrap()
        .partial_cmp(&b.as_f64().unwrap())
        .unwrap_or(std::cmp::Ordering::Equal)
}

// ── the machine ────────────────────────────────────────────────────────
pub struct VM<'io> {
    pub globals: HashMap<char, Value>,
    pub channels: HashMap<char, VecDeque<Value>>,
    pub strands: Vec<Strand>,
    by_sid: HashMap<i64, usize>,
    pub main_count: usize,
    next_spawn_sid: i64,
    pub failed: bool,
    pub stdin: &'io mut dyn BufRead,
    pub out: &'io mut dyn Write,
    pub err: &'io mut dyn Write,
    /// The program's command-line arguments, pushed as a string list by ⌂.
    pub args: Vec<String>,
    /// Parallel-mode substrate. None = the deterministic sequential
    /// scheduler (the language default, pinned by the conformance corpus).
    pub bus: Option<Arc<crate::par::Bus>>,
    /// Bytes pushed back by the ⌥ event parser (an ESC that turned out
    /// not to open a CSI sequence hands its follower back).
    pushback: VecDeque<u8>,
    /// Live web mode (mlang serve / MLANG_PORT): the listener ⎆ accepts
    /// from and ⍅ answers through. None = replay mode, where ⎆ reads
    /// request frames from stdin and ⍅ writes response frames to stdout.
    pub http: Option<Arc<crate::http::HttpBridge>>,
    /// Replay-mode request ids (⎆ counts up from 1) still awaiting a ⍅.
    next_request_id: i64,
    open_requests: HashSet<i64>,
    /// A pinned clock for ⌚, in Unix milliseconds. None reads the real
    /// clock; the conformance harness and MLANG_CLOCK pin it, because the
    /// clock is part of a run's input.
    pub clock: Option<i64>,
    /// The program's physical source lines, one entry per version: the
    /// program as started, then each hot patch the loom accepted. Empty
    /// lines when the source is unavailable (payloads from older toolchains).
    sources: Vec<Vec<String>>,
    /// The loom: the shared version store hot patches go through. Set by
    /// `mlang serve` (shared with the web bridge) or created on the first
    /// replayed patch frame.
    pub loom: Option<Arc<Loom>>,
    /// The main strands as the live source lists them, by strand id, in
    /// grid order — the slots a patch replaces, retires, or inserts into.
    slots: Vec<i64>,
    /// Sigils the standard library and bundled libraries define; a patch
    /// may not rebind them.
    lib_sigils: HashSet<char>,
    /// The definitions and boot code of the live version, for deciding
    /// what a patch changes. Computed from `boot_code` on the first patch
    /// — after the boot ran, so its definitions can be evaluated against
    /// the globals they refer to.
    boot_shape: Option<BootShape>,
    boot_code: Vec<Instr>,
    /// Set while the loom evaluates a definition's expression: every
    /// effect — I/O, channels, spawning, binding — glitches, so a hot
    /// definition is exactly a pure one.
    pure: bool,
    /// Per-channel count of (send sites, receive sites) across the whole
    /// program, computed once at start. A channel with sites on only one
    /// side cannot ever complete a handoff — the fingerprint of a mistyped
    /// or renamed channel name, and the most common cause of deadlock.
    chan_sites: HashMap<char, (usize, usize)>,
    /// The canvas the GUI ops (⌸ ▦ ⌶ ⎙) draw into, once ⌸ opens it.
    pub gui: Option<crate::gui::Gui>,
    /// Recorded runs (conformance, benches) set this so ⌸ never opens a
    /// real window even when the process has a terminal and a display.
    pub force_headless: bool,
}

/// Count send and receive sites per channel, descending into quotations.
/// `↥ ⇈` send; `↧ ⇂ ⇟` receive; `⇉XY` receives from X and sends to Y.
pub fn channel_sites(code: &[Instr], sites: &mut HashMap<char, (usize, usize)>) {
    for i in code {
        match &i.op {
            Op::B('↥', c, _) | Op::B('⇈', c, _) => sites.entry(*c).or_default().0 += 1,
            Op::B('↧', c, _) | Op::B('⇂', c, _) | Op::B('⇟', c, _) => {
                sites.entry(*c).or_default().1 += 1
            }
            Op::B('⇉', src, dst) => {
                sites.entry(*src).or_default().1 += 1;
                sites.entry(*dst).or_default().0 += 1;
            }
            Op::Push(Value::Quot(q)) => channel_sites(q, sites),
            _ => {}
        }
    }
}

/// A position's row field carries more than a row. Program rows live in
/// bands of ROW_STRIDE per source version — version 0 is the program as
/// started, each accepted hot patch (the loom) the next — so a report can
/// name the version a position belongs to and excerpt that version's
/// source. Library code sits above every version band: std.ml rows at
/// +STD_ROWS, ui.ml rows at +UI_ROWS, json.ml rows at +JSON_ROWS.
pub const ROW_STRIDE: u32 = 1 << 16;
pub const MAX_VERSIONS: u32 = 1 << 14;
pub const STD_ROWS: u32 = ROW_STRIDE * MAX_VERSIONS;
pub const UI_ROWS: u32 = STD_ROWS + (1 << 28);
pub const JSON_ROWS: u32 = STD_ROWS + (1 << 29);

/// Split a position into (source label, version, display row, col).
/// Library positions carry version 0.
fn pos_origin(pos: Pos) -> (&'static str, u32, u32, u32) {
    match pos.0 {
        r if r >= JSON_ROWS => ("json.ml ", 0, r - JSON_ROWS, pos.1),
        r if r >= UI_ROWS => ("ui.ml ", 0, r - UI_ROWS, pos.1),
        r if r >= STD_ROWS => ("std.ml ", 0, r - STD_ROWS, pos.1),
        r => ("", r / ROW_STRIDE, r % ROW_STRIDE, pos.1),
    }
}

/// The label a report prints before coordinates: nothing for the
/// program as started, `v3 ` for code that arrived with the third patch.
fn version_label(src: &str, version: u32) -> String {
    if !src.is_empty() || version == 0 {
        src.to_string()
    } else {
        format!("v{version} ")
    }
}

/// The body of a glitch report: source excerpt, call chain, and the stack
/// as the fault left it. Shared by both engines so their reports are
/// identical in anatomy.
pub fn fault_detail(
    source: &[String],
    pos: Pos,
    chain: &[(char, Pos)],
    stack: &[Value],
) -> String {
    fault_detail_in(std::slice::from_ref(&source.to_vec()), pos, chain, stack)
}

/// `fault_detail` over every source version a run has seen.
pub fn fault_detail_in(
    sources: &[Vec<String>],
    pos: Pos,
    chain: &[(char, Pos)],
    stack: &[Value],
) -> String {
    let mut out = String::new();
    if let Some(x) = excerpt_in(sources, pos) {
        out.push_str(&x);
        out.push('\n');
    }
    // Innermost call first: a fault inside a definition (or a std-library
    // word) names the definition and where it was called.
    for &(name, site) in chain.iter().rev().take(4) {
        out.push_str(&format!("  in {name}, called at {}\n", coords(site)));
    }
    if chain.len() > 4 {
        let extra = chain.len() - 4;
        out.push_str(&format!(
            "  … and {extra} more call{}\n",
            if extra == 1 { "" } else { "s" }
        ));
    }
    let d = stack.len();
    let shown: Vec<String> = stack[d.saturating_sub(8)..]
        .iter()
        .map(|v| cap_value(&fmt(v, true), 48))
        .collect();
    out.push_str(&format!(
        "  stack: {}{}\n",
        if d > 8 { "… " } else { "" },
        if shown.is_empty() { "(empty)".into() } else { shown.join(" ") }
    ));
    out
}

/// Name channels the program can never complete a handoff on. A channel
/// written but never read (or read but never written) is almost always a
/// mistyped or renamed name — the most common way a working grid
/// deadlocks — so the report says so outright instead of leaving the wait
/// graph to imply it. Channels somebody is currently stuck on come first.
pub fn channel_census(sites: &HashMap<char, (usize, usize)>, waited: &[char]) -> String {
    let mut orphans: Vec<(char, usize, usize)> = sites
        .iter()
        .filter(|(_, (send, recv))| *send == 0 || *recv == 0)
        .map(|(&c, &(send, recv))| (c, send, recv))
        .collect();
    orphans.sort_by_key(|(c, _, _)| (!waited.contains(c), *c));
    let mut out = String::new();
    for (c, send, recv) in orphans.iter().take(4) {
        let (n, side, other) = if *send == 0 {
            (recv, "received", "never sent to")
        } else {
            (send, "sent to", "never received")
        };
        out.push_str(&format!(
            "  ⚠ channel {c} is {side} at {n} site{} and {other} \
             — check for a misspelled channel name\n",
            if *n == 1 { "" } else { "s" }
        ));
    }
    out
}

/// Shorten one rendered value for a report line, keeping both ends so a
/// long list still shows its shape: `⟨1 2 3 …+24 more⟩`-ish.
fn cap_value(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        return s.to_string();
    }
    let head: String = chars[..max.saturating_sub(12)].iter().collect();
    format!("{head}…({} chars)", chars.len())
}

fn coords(pos: Pos) -> String {
    if pos == (0, 0) {
        "?".into()
    } else {
        let (src, version, row, col) = pos_origin(pos);
        format!("{}{row}:{col}", version_label(src, version))
    }
}

/// Render a two-line source excerpt for a position: the (windowed) line
/// and a caret marking the exact glyph. `lines` are the program's physical
/// source lines; library positions resolve against the bundled sources.
/// Returns None when the position is unlocatable (e.g. a payload built by
/// an older toolchain, or eval'd code no longer at hand).
pub fn excerpt(lines: &[String], pos: Pos) -> Option<String> {
    excerpt_in(std::slice::from_ref(&lines.to_vec()), pos)
}

/// `excerpt` over every source version a run has seen: `sources[v]` is
/// the physical lines of version v.
pub fn excerpt_in(sources: &[Vec<String>], pos: Pos) -> Option<String> {
    if pos == (0, 0) {
        return None;
    }
    let (src, version, row, col) = pos_origin(pos);
    let line: String = match src {
        "" => sources.get(version as usize)?.get(row.checked_sub(1)? as usize)?.clone(),
        "std.ml " => STD_SOURCE.lines().nth(row.checked_sub(1)? as usize)?.to_string(),
        "json.ml " => JSON_SOURCE.lines().nth(row.checked_sub(1)? as usize)?.to_string(),
        _ => UI_SOURCE.lines().nth(row.checked_sub(1)? as usize)?.to_string(),
    };
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();
    // 1-based col; a position just past the end (e.g. end-of-strand) is legal.
    let ci = (col.max(1) as usize - 1).min(n);
    const WIN: usize = 61;
    let start = if n <= WIN { 0 } else { ci.saturating_sub(30).min(n - WIN) };
    let end = (start + WIN).min(n);
    let shown: String = chars[start..end].iter().collect();
    let pre = if start > 0 { "…" } else { "" };
    let post = if end < n { "…" } else { "" };
    let src = version_label(src, version);
    let label = format!("  {src}{row}│ ");
    let caret_at = label.chars().count() + pre.chars().count() + (ci - start);
    Some(format!(
        "{label}{pre}{shown}{post}\n{spaces}↑ {src}{row}:{col}",
        spaces = " ".repeat(caret_at)
    ))
}

impl<'io> VM<'io> {
    pub fn new(
        stdin: &'io mut dyn BufRead,
        out: &'io mut dyn Write,
        err: &'io mut dyn Write,
    ) -> Self {
        VM {
            globals: HashMap::new(),
            channels: HashMap::new(),
            strands: Vec::new(),
            by_sid: HashMap::new(),
            main_count: 0,
            next_spawn_sid: 0,
            failed: false,
            stdin,
            out,
            err,
            args: Vec::new(),
            bus: None,
            pushback: VecDeque::new(),
            http: None,
            next_request_id: 1,
            open_requests: HashSet::new(),
            clock: clock_env(),
            sources: Vec::new(),
            loom: None,
            slots: Vec::new(),
            lib_sigils: HashSet::new(),
            boot_shape: None,
            boot_code: Vec::new(),
            pure: false,
            chan_sites: HashMap::new(),
            gui: None,
            force_headless: false,
        }
    }

    /// Replay-mode ⎆: read one frame from this VM's own stdin.
    fn read_request_frame(&mut self) -> Result<Option<crate::http::Frame>, String> {
        let stdin = &mut *self.stdin;
        let mut next = move || {
            let buf = stdin.fill_buf().ok()?;
            if buf.is_empty() {
                return None;
            }
            let b = buf[0];
            stdin.consume(1);
            Some(b)
        };
        crate::http::read_framed(&mut next)
    }

    // ── ⌥ input events ─────────────────────────────────────────────────
    // One event per call, parsed from the same byte stream ⌨ reads, so a
    // recorded pipe replays exactly what a live terminal produced. Keys
    // become the glyph they are; a mouse press becomes ⟨«⌖» x y⟩.

    fn read_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.pushback.pop_front() {
            return Some(b);
        }
        // Parallel mode reads the shared stdin; a thread's own stdin is a
        // dummy. Pushback stays per-VM — it is parser state, not input.
        if let Some(bus) = &self.bus {
            return bus.read_byte();
        }
        let buf = self.stdin.fill_buf().ok()?;
        if buf.is_empty() {
            return None;
        }
        let b = buf[0];
        self.stdin.consume(1);
        Some(b)
    }

    /// Decode one UTF-8 scalar; malformed bytes become U+FFFD.
    fn read_char(&mut self) -> Option<char> {
        let b0 = self.read_byte()?;
        let need = match b0 {
            0x00..=0x7f => return Some(b0 as char),
            0xc0..=0xdf => 1,
            0xe0..=0xef => 2,
            0xf0..=0xf7 => 3,
            _ => return Some('\u{fffd}'),
        };
        let mut bytes = vec![b0];
        for _ in 0..need {
            match self.read_byte() {
                Some(b) if b & 0xc0 == 0x80 => bytes.push(b),
                Some(b) => {
                    self.pushback.push_back(b);
                    return Some('\u{fffd}');
                }
                None => return Some('\u{fffd}'),
            }
        }
        match std::str::from_utf8(&bytes) {
            Ok(s) => s.chars().next(),
            Err(_) => Some('\u{fffd}'),
        }
    }

    /// Parse one CSI sequence (the ⎋[ is already consumed). Returns a
    /// deliverable event, or None for sequences ⌥ swallows (releases,
    /// motion, wheel, unknown finals).
    fn read_csi(&mut self) -> Option<Option<Value>> {
        let mut params = String::new();
        loop {
            let b = self.read_byte()?;
            if (0x40..=0x7e).contains(&b) {
                let event = match b {
                    b'A' => Some(Value::str("↑")),
                    b'B' => Some(Value::str("↓")),
                    b'C' => Some(Value::str("→")),
                    b'D' => Some(Value::str("←")),
                    b'H' => Some(Value::str("⇱")),
                    b'F' => Some(Value::str("⇲")),
                    b'~' if params == "1" || params == "7" => Some(Value::str("⇱")),
                    b'~' if params == "4" || params == "8" => Some(Value::str("⇲")),
                    b'~' if params == "2" => Some(Value::str("⎀")),
                    b'~' if params == "3" => Some(Value::str("⌦")),
                    b'~' if params == "5" => Some(Value::str("⇞")),
                    b'~' if params == "6" => Some(Value::str("⇟")),
                    b'M' if params.starts_with('<') => {
                        let nums: Vec<i64> = params[1..]
                            .split(';')
                            .map(|p| p.parse().unwrap_or(-1))
                            .collect();
                        match nums.as_slice() {
                            // SGR press: button < 32 (no motion/wheel bits)
                            [b, x, y] if (0..32).contains(b) && *x >= 0 && *y >= 0 => {
                                Some(Value::List(Arc::new(vec![
                                    Value::str("⌖"),
                                    Value::int(*x),
                                    Value::int(*y),
                                ])))
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                };
                return Some(event);
            }
            params.push(b as char);
            if params.len() > 32 {
                return Some(None); // runaway sequence — bail out
            }
        }
    }

    /// Read the next input event for ⌥. ∅ at end of input.
    fn read_event(&mut self) -> Value {
        loop {
            let Some(c) = self.read_char() else {
                return Value::Nil;
            };
            match c {
                // «↵», not «⏎»: inside MLang strings the ⏎ glyph denotes a
                // newline, so a ⏎ event could never be written or compared.
                '\r' | '\n' => return Value::str("↵"),
                '\t' => return Value::str("⇥"),
                '\u{8}' | '\u{7f}' => return Value::str("⌫"),
                '\u{1b}' => match self.read_byte() {
                    None => return Value::Nil,
                    Some(b'[') => match self.read_csi() {
                        None => return Value::Nil,
                        Some(Some(event)) => return event,
                        Some(None) => continue,
                    },
                    Some(other) => {
                        self.pushback.push_back(other);
                        return Value::str("⎋");
                    }
                },
                c if (c as u32) < 32 => {
                    // control chords in caret notation: Ctrl-C is «^C»
                    let chord = char::from((c as u8) + 0x40);
                    return Value::str(format!("^{chord}"));
                }
                c => return Value::str(c.to_string()),
            }
        }
    }

    // ── the substrate switch ───────────────────────────────────────────
    // Sequential mode (bus: None) keeps all shared state — channels,
    // globals, spawned strands — inside this VM, and blocking ops signal
    // Sig::Block to the deterministic scheduler. Parallel mode routes the
    // same operations through the shared Bus, where blocking ops park the
    // OS thread instead; Sig::Block never occurs there.

    fn chan_send(&mut self, c: char, v: Value) {
        match &self.bus {
            Some(bus) => bus.send(c, v),
            None => self.channels.entry(c).or_default().push_back(v),
        }
    }

    fn chan_try_recv(&mut self, c: char) -> Option<Value> {
        match &self.bus {
            Some(bus) => bus.try_recv(c),
            None => self.channels.entry(c).or_default().pop_front(),
        }
    }

    /// Receive for blocking ops. Sequential: None means "signal Sig::Block".
    /// Parallel: parks until a value arrives (the Bus detects deadlock and
    /// aborts the process itself), so None is never returned.
    fn chan_recv(&mut self, c: char, sid: i64, label: &str, pos: Pos) -> Option<Value> {
        if let Some(bus) = &self.bus {
            let bus = bus.clone();
            let _ = self.out.flush(); // a prompt must survive a park
            Some(bus.recv(c, sid, label, pos))
        } else {
            self.channels.entry(c).or_default().pop_front()
        }
    }

    /// Resolve a global, consulting the shared table in parallel mode.
    /// Globals are single-assignment, so caching a hit locally is sound.
    fn global_lookup(&mut self, c: char) -> Option<Value> {
        if let Some(v) = self.globals.get(&c) {
            return Some(v.clone());
        }
        if let Some(bus) = &self.bus {
            if let Some(v) = bus.global_get(c) {
                self.globals.insert(c, v.clone());
                return Some(v);
            }
        }
        None
    }

    fn register(&mut self, strand: Strand) {
        self.by_sid.insert(strand.sid, self.strands.len());
        self.strands.push(strand);
    }

    fn report_glitch(&mut self, idx: usize) {
        self.failed = true;
        let s = &self.strands[idx];
        let (v, pos) = s.glitch.as_ref().unwrap();
        let _ = writeln!(
            self.err,
            "✗ glitch in strand {} ({}) at {}: {}",
            fmt_i64(s.sid),
            s.label,
            coords(*pos),
            fmt(v, false)
        );
        let detail = fault_detail_in(&self.sources, *pos, &s.glitch_chain, s.stack_view());
        let _ = write!(self.err, "{detail}");
        if let Some(loom) = &self.loom {
            loom.record_fault(format!(
                "✗ glitch in strand {} ({}) at {}: {}\n{detail}",
                fmt_i64(s.sid), s.label, coords(*pos), fmt(v, false)
            ));
        }
    }

    fn report_deadlock(&mut self, blocked: &[usize]) {
        self.failed = true;
        let _ = writeln!(self.err, "✗ deadlock — every remaining strand is blocked:");
        for &i in blocked {
            let s = &self.strands[i];
            let (on, pos) = s.block.unwrap();
            let what = match on {
                BlockOn::Chan(c) => format!("channel {c}"),
                BlockOn::Strand(id) => format!("strand {}", fmt_i64(id)),
                BlockOn::Stdin => "stdin".into(),
            };
            let _ = writeln!(
                self.err,
                "  strand {} ({}) waiting on {} at {}",
                fmt_i64(s.sid),
                s.label,
                what,
                coords(pos)
            );
            if let Some(x) = excerpt_in(&self.sources, pos) {
                let _ = writeln!(self.err, "{x}");
            }
        }
        let waited: Vec<char> = blocked
            .iter()
            .filter_map(|&i| match self.strands[i].block {
                Some((BlockOn::Chan(c), _)) => Some(c),
                _ => None,
            })
            .collect();
        let census = channel_census(&self.chan_sites, &waited);
        let _ = write!(self.err, "{census}");
        if let Some(loom) = &self.loom {
            let mut report = String::from("✗ deadlock — every remaining strand is blocked:\n");
            for &i in blocked {
                let s = &self.strands[i];
                let (on, pos) = s.block.unwrap();
                let what = match on {
                    BlockOn::Chan(c) => format!("channel {c}"),
                    BlockOn::Strand(id) => format!("strand {}", fmt_i64(id)),
                    BlockOn::Stdin => "stdin".into(),
                };
                report.push_str(&format!("  strand {} ({}) waiting on {} at {}\n", fmt_i64(s.sid), s.label, what, coords(pos)));
            }
            report.push_str(&census);
            loom.record_fault(report);
        }
    }

    fn try_unblock(&mut self, i: usize) {
        let (on, _) = self.strands[i].block.unwrap();
        let free = match on {
            BlockOn::Chan(c) => self.channels.get(&c).map(|q| !q.is_empty()).unwrap_or(false),
            BlockOn::Strand(id) => self
                .by_sid
                .get(&id)
                .map(|&t| matches!(self.strands[t].status, Status::Done | Status::Dead))
                .unwrap_or(false),
            BlockOn::Stdin => !self.others_active(self.strands[i].sid),
        };
        if free {
            self.strands[i].status = Status::Run;
            self.strands[i].block = None;
        }
    }

    /// True if any strand other than `me` could make progress right now:
    /// runnable, or blocked on something already available. Strands waiting
    /// on ⌨ don't count — they would defer the same way. Stdin reads carry
    /// the lowest scheduling priority (see the '⌨' arm), so this decides
    /// both when a read must defer and when a deferred read may wake.
    fn others_active(&self, me: i64) -> bool {
        self.strands.iter().any(|t| {
            t.sid != me
                && t.sid != i64::MIN
                && match t.status {
                    // A Run strand whose frames have emptied is finished in all
                    // but name — it is marked Done on its next visit and can
                    // produce nothing more.
                    Status::Run => !t.frames.is_empty(),
                    Status::Blocked => match t.block {
                        Some((BlockOn::Chan(c), _)) => self
                            .channels
                            .get(&c)
                            .map(|q| !q.is_empty())
                            .unwrap_or(false),
                        Some((BlockOn::Strand(id), _)) => self
                            .by_sid
                            .get(&id)
                            .map(|&x| {
                                matches!(self.strands[x].status, Status::Done | Status::Dead)
                            })
                            .unwrap_or(false),
                        Some((BlockOn::Stdin, _)) | None => false,
                    },
                    _ => false,
                }
        })
    }

    fn run_slice(&mut self, idx: usize) -> usize {
        let mut s = std::mem::replace(&mut self.strands[idx], Strand::placeholder());
        let executed = run_burst(self, &mut s, SLICE);
        self.strands[idx] = s;
        executed
    }

    pub fn run_scheduler(&mut self) {
        loop {
            let mut progressed = 0;
            let snapshot = self.strands.len();
            for i in 0..snapshot {
                if self.strands[i].pending.is_some() && self.strands[i].at_seam() {
                    self.strands[i].swap_in();
                }
                if self.strands[i].status == Status::Blocked {
                    self.try_unblock(i);
                }
                if self.strands[i].status == Status::Run {
                    progressed += self.run_slice(i);
                    if self.strands[i].status == Status::Dead {
                        self.report_glitch(i);
                    }
                }
            }
            let live: Vec<usize> = (0..self.strands.len())
                .filter(|&i| {
                    matches!(self.strands[i].status, Status::Run | Status::Blocked)
                })
                .collect();
            if live.is_empty() {
                return;
            }
            if progressed == 0 {
                let blocked: Vec<usize> = live
                    .iter()
                    .copied()
                    .filter(|&i| self.strands[i].status == Status::Blocked)
                    .collect();
                if !blocked.is_empty() && blocked.len() == live.len() {
                    // A strand waiting its turn at ⌨ is not deadlocked: the
                    // grid has gone quiet, so the next round wakes it and the
                    // read proceeds (blocking on the OS, not the scheduler).
                    let stdin_waiter = blocked.iter().any(|&i| {
                        matches!(self.strands[i].block, Some((BlockOn::Stdin, _)))
                    });
                    if !stdin_waiter {
                        self.report_deadlock(&blocked);
                        return;
                    }
                }
            }
        }
    }

    pub fn run_compiled(&mut self, prog: &CompiledProgram) -> i32 {
        self.main_count = prog.strands.len();
        self.next_spawn_sid = prog.strands.len() as i64;
        self.sources = vec![prog.source.clone()];
        self.slots = (0..prog.strands.len() as i64).collect();
        self.boot_code = program_boot(&prog.boot);
        // Reserved sigils are the ones a woven library actually defines:
        // std always, a bundled library only when this program pulled it
        // in. A program that owns a Construct sigil (§6.1) keeps owning
        // it under the loom.
        let woven: Vec<Instr> = prog.boot.iter().filter(|i| i.pos.0 >= STD_ROWS).cloned().collect();
        scan_names(&woven, &mut HashSet::new(), &mut self.lib_sigils);
        channel_sites(&prog.boot, &mut self.chan_sites);
        for (_, code) in &prog.strands {
            channel_sites(code, &mut self.chan_sites);
        }

        // The boot strand always runs: the standard library first, then the
        // program's own boot section (both already woven in at compile time).
        let boot = Strand::new(
            -1,
            "boot".into(),
            Arc::new(prog.boot.clone()),
            Vec::new(),
        );
        self.register(boot);
        self.run_scheduler();
        let boot_dead = self.strands[self.by_sid[&-1]].status == Status::Dead;
        if boot_dead || self.failed {
            return 1;
        }
        for (i, (label, code)) in prog.strands.iter().enumerate() {
            self.register(Strand::new(
                i as i64,
                label.clone(),
                Arc::new(code.clone()),
                Vec::new(),
            ));
        }
        self.run_scheduler();
        self.hold_for_patches();
        if self.failed {
            1
        } else {
            0
        }
    }

    /// A served grid with its loom open does not exit when its strands
    /// have all finished, died, or deadlocked: it holds the port, answers
    /// every request 503 with the fault reports, and waits for a patch
    /// — which can revive a dead strand or start a new one, whereupon
    /// the scheduler runs again. The grid never stops; it waits to be
    /// mended.
    fn hold_for_patches(&mut self) {
        loop {
            let (Some(bridge), Some(_)) = (self.http.clone(), self.loom.as_ref()) else { return };
            let dead: Vec<String> = self
                .strands
                .iter()
                .filter(|s| s.status == Status::Dead)
                .map(|s| format!("strand {} ({})", fmt_i64(s.sid), s.label))
                .collect();
            let why = if dead.is_empty() {
                "every strand has finished or is blocked".to_string()
            } else {
                format!("dead: {}", dead.join(", "))
            };
            let _ = writeln!(self.err, "⟡ the grid has stopped ({why}) — holding the port for a patch");
            let _ = self.err.flush();
            // Requests a dead strand accepted and never answered get the
            // reason now, not a timeout.
            bridge.fail_pending(503, &format!("the grid has stopped ({why}) — mend it: mlang pull / mlang patch\n"));
            loop {
                match bridge.accept() {
                    crate::http::Incoming::Request((id, _, _, _)) => {
                        bridge.respond(
                            id,
                            503,
                            "text/plain; charset=utf-8",
                            &format!("the grid has stopped ({why}) — mend it: mlang pull / mlang patch\n"),
                        );
                    }
                    crate::http::Incoming::Patch { id, base, text } => {
                        let (status, body) = match self.hot_patch(base, &text, None) {
                            Ok(report) => (200, report),
                            Err((status, why)) => (i64::from(status), why),
                        };
                        let _ = self.err.write_all(body.as_bytes());
                        let _ = self.err.flush();
                        bridge.respond(id, status, "text/plain; charset=utf-8", &body);
                        // Anything the patch can wake — a revived or a new
                        // strand — is at its seam already; run the grid.
                        for s in self.strands.iter_mut() {
                            if s.pending.is_some() && s.at_seam() {
                                s.swap_in();
                            }
                        }
                        if self.strands.iter().any(|s| s.status == Status::Run) {
                            break;
                        }
                    }
                }
            }
            self.run_scheduler();
        }
    }
}

/// The program's own boot instructions — everything below the library
/// row bands.
fn program_boot(boot: &[Instr]) -> Vec<Instr> {
    boot.iter().filter(|i| i.pos.0 < STD_ROWS).cloned().collect()
}

/// Operations a hot definition may not perform: anything that reaches
/// outside the expression. (⌂, ⌚, and ⍜ are reads of the run's input,
/// not effects, and stay allowed.)
const EFFECT_OPS: &str = "↥↧⇂⇈⇟⇉⚡⋈⌛⍞⊸⌨⌥⍟⍇⍈⍆⎆⍅⌸▦⌶⎙⌹≔";

/// How many steps a hot definition's expression may take. Past this it
/// is boot code — a patch cannot wait on it.
const PURE_BUDGET: usize = 1_000_000;

impl VM<'_> {
    /// Evaluate an instruction strip with no effects allowed, from an
    /// empty stack; Ok is the stack it leaves.
    fn eval_pure(&mut self, code: Arc<Vec<Instr>>) -> Result<Vec<Value>, String> {
        let mut s = Strand::new(i64::MIN, "hot definition".into(), code, Vec::new());
        self.pure = true;
        let mut steps = 0;
        while s.status == Status::Run && steps < PURE_BUDGET {
            steps += run_burst(self, &mut s, SLICE).max(1);
        }
        self.pure = false;
        match s.status {
            Status::Done => Ok(s.stack),
            Status::Dead => Err(fmt(&s.glitch.map(|(v, _)| v).unwrap_or(Value::Nil), false)),
            Status::Blocked => Err("blocks".into()),
            Status::Run => Err(format!("takes more than {PURE_BUDGET} steps")),
        }
    }

    /// Classify a boot strip (§4.7): each `≔X` closes a definition whose
    /// expression is everything since the previous one; it is a hot
    /// definition when that expression evaluates purely to one value.
    /// Everything else is boot code. Definitions see the ones before
    /// them, as they would at boot.
    fn shape_of(&mut self, boot: &[Instr]) -> BootShape {
        let saved = std::mem::take(&mut self.globals);
        self.globals = saved.clone();
        let mut defs = Vec::new();
        let mut code = Vec::new();
        let mut segment: Vec<Instr> = Vec::new();
        for i in boot {
            if let Op::B('≔', c, _) = &i.op {
                let expr = Arc::new(std::mem::take(&mut segment));
                match self.eval_pure(expr.clone()) {
                    Ok(stack) if stack.len() == 1 => {
                        let v: Value = stack.into_iter().next().unwrap();
                        self.globals.insert(*c, v.clone());
                        defs.push((*c, v, i.pos));
                    }
                    _ => {
                        code.extend(expr.iter().cloned());
                        code.push(i.clone());
                    }
                }
            } else {
                segment.push(i.clone());
            }
        }
        code.extend(segment);
        self.globals = saved;
        BootShape { defs, code }
    }

    /// Apply a hot patch (SPEC §4.7): merge `text`, written against
    /// version `base`, onto the live source; weave it; rebind changed
    /// literal definitions now; hand changed strands their new code for
    /// their next seam; start added strands; retire removed ones. `me`
    /// is the strand executing the accept that delivered the patch (it
    /// is checked out of the strand table while it runs). Ok carries the
    /// report, Err an HTTP-style status and the reason: 409 for a merge
    /// conflict, 422 for a patch the loom cannot apply.
    pub fn hot_patch(
        &mut self,
        base: usize,
        text: &str,
        mut me: Option<&mut Strand>,
    ) -> Result<String, (u16, String)> {
        if self.bus.is_some() {
            return Err((422, "✗ patch rejected: hot patching needs the deterministic scheduler — drop --parallel\n".into()));
        }
        let loom = match &self.loom {
            Some(l) => l.clone(),
            None => {
                let text = self.sources.first().map(|l| l.join("\n") + "\n").unwrap_or_default();
                let l = Loom::new(&text);
                self.loom = Some(l.clone());
                l
            }
        };
        let cur = loom.current();
        let version = cur + 1;
        if version as u32 >= MAX_VERSIONS {
            return Err((422, format!("✗ patch rejected: the loom holds at most {MAX_VERSIONS} versions\n")));
        }
        let merged = loom.merge(base, text).map_err(|e| (409, e))?;
        let (merged, migrations) = loom::split_migrations(&merged);
        let merged_lines: Vec<String> = merged.lines().map(String::from).collect();
        if !migrations.is_empty() && merged_lines.first().map(|l| l.trim() == "⇓").unwrap_or(false) {
            return Err((422, "✗ patch rejected: ⟲ migrations are written in flat form\n".into()));
        }
        if merged_lines.len() as u32 >= ROW_STRIDE {
            return Err((422, format!("✗ patch rejected: a source may have at most {} lines\n", ROW_STRIDE - 1)));
        }
        let prog = match compile_text(&merged) {
            Ok(p) => p,
            Err(e) => {
                let loc = match e.pos {
                    Some((r, c)) => format!(" at {r}:{c}"),
                    None => String::new(),
                };
                let mut out = format!("✗ weave error{loc}: {}\n", e.msg);
                if let Some(pos) = e.pos {
                    if let Some(x) = excerpt(&merged_lines, pos) {
                        out.push_str(&x);
                        out.push('\n');
                    }
                }
                return Err((422, out));
            }
        };
        let off = version as u32 * ROW_STRIDE;

        // The boot section: literal definitions may change; code may not.
        let mut new_boot = program_boot(&prog.boot);
        offset_rows(&mut new_boot, off);
        let old_shape = match self.boot_shape.take() {
            Some(shape) => shape,
            None => {
                let boot_code = self.boot_code.clone();
                self.shape_of(&boot_code)
            }
        };
        let new_shape = self.shape_of(&new_boot);
        if !loom::instrs_eq(&old_shape.code, &new_shape.code) {
            let at = new_shape
                .code
                .iter()
                .zip(old_shape.code.iter())
                .find(|(n, o)| !loom::instrs_eq(std::slice::from_ref(*n), std::slice::from_ref(*o)))
                .map(|(n, _)| n.pos)
                .or_else(|| new_shape.code.get(old_shape.code.len()).map(|i| i.pos))
                .or_else(|| old_shape.code.first().map(|i| i.pos));
            self.boot_shape = Some(old_shape);
            // The rejected text never became a version: plain coordinates.
            let at = at.map(|p| (p.0 % ROW_STRIDE, p.1));
            let mut out = format!(
                "✗ patch rejected: boot code changed{} — it ran once at start and cannot run again; only pure ≔ definitions and strands are hot\n",
                at.map(|p| format!(" at {}", coords(p))).unwrap_or_default()
            );
            if let Some(x) = at.and_then(|p| excerpt(&merged_lines, p)) {
                out.push_str(&x);
                out.push('\n');
            }
            return Err((422, out));
        }
        for (c, _, pos) in &new_shape.defs {
            if self.lib_sigils.contains(c) {
                self.boot_shape = Some(old_shape);
                return Err((422, format!(
                    "✗ patch rejected: ≔{c} at {} — {c} is defined by the standard library\n",
                    coords((pos.0 % ROW_STRIDE, pos.1))
                )));
            }
        }
        let mut rebound = Vec::new();
        let mut added = Vec::new();
        let mut removed = Vec::new();
        for (c, v, _) in &new_shape.defs {
            match old_shape.defs.iter().find(|(o, _, _)| o == c) {
                Some((_, ov, _)) if loom::value_eq(ov, v) => {}
                Some(_) => rebound.push(*c),
                None => added.push(*c),
            }
        }
        for (c, _, _) in &old_shape.defs {
            if !new_shape.defs.iter().any(|(n, _, _)| n == c) {
                removed.push(*c);
            }
        }

        // The strands: match the live slots against the patched grid.
        let mut new_strands: Vec<(String, Vec<Instr>)> = prog.strands.clone();
        for (_, code) in new_strands.iter_mut() {
            offset_rows(code, off);
        }
        // Migrations: each ⟲ line belongs to the first strand below it.
        let strand_rows: Vec<u32> = new_strands
            .iter()
            .map(|(label, _)| label.strip_prefix("row ").and_then(|r| r.parse().ok()).unwrap_or(0))
            .collect();
        let mut migrate_of: Vec<Option<Arc<Vec<Instr>>>> = vec![None; new_strands.len()];
        for (row, code) in &migrations {
            let Some(n) = strand_rows.iter().position(|&r| r > *row) else {
                self.boot_shape = Some(old_shape);
                return Err((422, format!("✗ patch rejected: ⟲ at {row}:1 has no strand below it to migrate\n")));
            };
            let cells: Vec<crate::lex::Cell> = code
                .chars()
                .enumerate()
                .map(|(k, ch)| crate::lex::Cell { ch, row: *row, col: k as u32 + 2 })
                .collect();
            let mut lexed = match lex_strand(cells, crate::lex::Axis::Row) {
                Ok(l) => l,
                Err(e) => {
                    self.boot_shape = Some(old_shape);
                    let loc = e.pos.map(|(r, c)| format!(" at {r}:{c}")).unwrap_or_default();
                    return Err((422, format!("✗ weave error{loc}: {} (in a ⟲ migration)\n", e.msg)));
                }
            };
            offset_rows(&mut lexed, off);
            migrate_of[n] = Some(Arc::new(lexed));
        }
        let origin_of = |vm: &VM, me: &Option<&mut Strand>, sid: i64| -> Arc<Vec<Instr>> {
            match me {
                Some(m) if m.sid == sid => m.origin.clone(),
                _ => vm.strands[vm.by_sid[&sid]].origin.clone(),
            }
        };
        let old: Vec<Arc<Vec<Instr>>> =
            self.slots.iter().map(|&sid| origin_of(self, &me, sid)).collect();
        let new_codes: Vec<Vec<Instr>> = new_strands.iter().map(|(_, c)| c.clone()).collect();
        let plan = loom::plan_strands(&old, &new_codes);

        // ── commit ──
        self.sources.push(merged_lines);
        for c in &removed {
            self.globals.remove(c);
        }
        for (c, v, _) in &new_shape.defs {
            if rebound.contains(c) || added.contains(c) {
                self.globals.insert(*c, v.clone());
            }
        }
        self.boot_shape = Some(new_shape);
        let mut slots = Vec::new();
        let (mut replaced, mut started, mut retired) = (Vec::new(), Vec::new(), Vec::new());
        for action in plan {
            match action {
                StrandAction::Keep(o, _) => slots.push(self.slots[o]),
                StrandAction::Replace(o, n) => {
                    let sid = self.slots[o];
                    let (label, code) = &new_strands[n];
                    let swap = Swap::Replace(Arc::new(code.clone()), label.clone(), migrate_of[n].clone());
                    match &mut me {
                        Some(m) if m.sid == sid => m.pending = Some(swap),
                        _ => self.strands[self.by_sid[&sid]].pending = Some(swap),
                    }
                    replaced.push((sid, label.clone()));
                    slots.push(sid);
                }
                StrandAction::Retire(o) => {
                    let sid = self.slots[o];
                    match &mut me {
                        Some(m) if m.sid == sid => m.pending = Some(Swap::Retire),
                        _ => self.strands[self.by_sid[&sid]].pending = Some(Swap::Retire),
                    }
                    retired.push(sid);
                }
                StrandAction::Start(n) => {
                    let sid = self.next_spawn_sid;
                    self.next_spawn_sid += 1;
                    let (label, code) = &new_strands[n];
                    let mut fresh = Strand::new(sid, label.clone(), Arc::new(code.clone()), Vec::new());
                    if let Some(m) = &migrate_of[n] {
                        fresh.frames.push(cf(m.clone()));
                    }
                    self.register(fresh);
                    started.push((sid, label.clone()));
                    slots.push(sid);
                }
            }
        }
        self.slots = slots;
        self.chan_sites.clear();
        channel_sites(&prog.boot, &mut self.chan_sites);
        for (_, code) in &prog.strands {
            channel_sites(code, &mut self.chan_sites);
        }

        // ── the report ──
        let sigils = |v: &[char]| v.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(" ");
        let mut parts = Vec::new();
        if !rebound.is_empty() {
            parts.push(format!("{} rebound ({})", plural(rebound.len(), "definition"), sigils(&rebound)));
        }
        if !added.is_empty() {
            parts.push(format!("{} added ({})", plural(added.len(), "definition"), sigils(&added)));
        }
        if !removed.is_empty() {
            parts.push(format!("{} removed ({})", plural(removed.len(), "definition"), sigils(&removed)));
        }
        if !replaced.is_empty() {
            parts.push(format!("{} replaced", plural(replaced.len(), "strand")));
        }
        if !started.is_empty() {
            parts.push(format!("{} started", plural(started.len(), "strand")));
        }
        if !retired.is_empty() {
            parts.push(format!("{} retired", plural(retired.len(), "strand")));
        }
        let note = if parts.is_empty() { "no change".to_string() } else { parts.join(", ") };
        let mut report = format!("⟡ v{version}: {note}\n");
        for (sid, label) in &replaced {
            let migrated = new_strands.iter().position(|(l, _)| l == label).and_then(|n| migrate_of[n].as_ref());
            report.push_str(&format!(
                "  strand {} continues as {label} at its next seam{}\n",
                fmt_i64(*sid),
                if migrated.is_some() { ", after its ⟲ migration" } else { "" }
            ));
        }
        for (sid, label) in &started {
            report.push_str(&format!("  strand {} started as {label}\n", fmt_i64(*sid)));
        }
        for sid in &retired {
            report.push_str(&format!("  strand {} retires at its next seam\n", fmt_i64(*sid)));
        }
        loom.push(merged, note);
        Ok(report)
    }
}

fn plural(n: usize, what: &str) -> String {
    format!("{n} {what}{}", if n == 1 { "" } else { "s" })
}

/// A fully compiled program: the standard library and boot section woven
/// into one boot instruction strip, plus one strip per main strand. This is
/// what `mlang build` serializes into a native binary.
#[derive(Debug)]
pub struct CompiledProgram {
    pub boot: Vec<Instr>,
    pub strands: Vec<(String, Vec<Instr>)>,
    /// The program's physical source lines, carried for report excerpts
    /// (and welded into built binaries). Empty when unavailable.
    pub source: Vec<String>,
}

/// Compile a parsed source form to a CompiledProgram.
pub fn compile(prog: &Program) -> Result<CompiledProgram, LoadError> {
    let mut strands = Vec::new();
    for (label, cells) in &prog.strands {
        let code = lex_strand(cells.clone(), prog.axis)?;
        if !code.is_empty() {
            // comment-only lines/columns are not strands
            strands.push((label.clone(), code));
        }
    }
    let program_boot = match &prog.boot_cells {
        Some(cells) => lex_strand(cells.clone(), prog.axis)?,
        None => Vec::new(),
    };
    let mut boot = std_code();
    let mut refs = HashSet::new();
    let mut defs = HashSet::new();
    scan_names(&program_boot, &mut refs, &mut defs);
    for (_, code) in &strands {
        scan_names(code, &mut refs, &mut defs);
    }
    for (_, source, band) in LIBS {
        let lib = lib_code(source, *band);
        let mut lib_defs = HashSet::new();
        scan_names(&lib, &mut HashSet::new(), &mut lib_defs);
        if refs.iter().any(|c| lib_defs.contains(c) && !defs.contains(c)) {
            boot.extend(lib);
        }
    }
    boot.extend(program_boot);
    Ok(CompiledProgram { boot, strands, source: Vec::new() })
}

/// Compile MLang source text (rain or flat form).
pub fn compile_text(text: &str) -> Result<CompiledProgram, LoadError> {
    let mut prog = compile(&crate::forms::parse_source(text)?)?;
    prog.source = text.lines().map(String::from).collect();
    Ok(prog)
}

pub const STD_SOURCE: &str = include_str!("../../std/std.ml");
pub const UI_SOURCE: &str = include_str!("../../std/ui.ml");
pub const JSON_SOURCE: &str = include_str!("../../std/json.ml");

/// Bundled libraries, in weave order. A library is woven into the boot
/// strand — after std, before the program's own boot section — exactly
/// when the program references a sigil the library defines without
/// defining that sigil itself (§6.1). Weaving is decided at compile time,
/// so welded binaries carry only the libraries they use.
const LIBS: &[(&str, &str, u32)] = &[("ui", UI_SOURCE, UI_ROWS), ("json", JSON_SOURCE, JSON_ROWS)];

/// Collect referenced names and defined sigils (≔ and ⇒ targets),
/// recursing into quotations.
fn scan_names(code: &[Instr], refs: &mut HashSet<char>, defs: &mut HashSet<char>) {
    for instr in code {
        match &instr.op {
            Op::Name(c) => {
                refs.insert(*c);
            }
            Op::B('≔', c, _) | Op::B('⇒', c, _) => {
                defs.insert(*c);
            }
            Op::Push(Value::Quot(q)) => scan_names(q, refs, defs),
            _ => {}
        }
    }
}

/// Does this program execute `op` anywhere (including inside quotations)?
fn program_uses(prog: &CompiledProgram, op: char) -> bool {
    fn has_op(code: &[Instr], op: char) -> bool {
        code.iter().any(|i| match &i.op {
            Op::B(c, _, _) if *c == op => true,
            Op::Push(Value::Quot(q)) => has_op(q, op),
            _ => false,
        })
    }
    has_op(&prog.boot, op) || prog.strands.iter().any(|(_, c)| has_op(c, op))
}

/// Does this program execute ⌥ anywhere? Decides whether the runner
/// should switch a real terminal into raw/mouse-reporting mode.
pub fn uses_interactive(prog: &CompiledProgram) -> bool {
    program_uses(prog, '⌥')
}

/// Does this program open a canvas (⌸)? A canvas program's input comes
/// from its window, so the runner leaves the terminal alone.
pub fn uses_gui(prog: &CompiledProgram) -> bool {
    program_uses(prog, '⌸')
}

/// Shift every position row (including inside nested quotations) into a
/// library's row band, so reports can name the source it points into.
fn offset_rows(code: &mut Vec<Instr>, off: u32) {
    for i in code.iter_mut() {
        if i.pos.0 != 0 {
            i.pos.0 += off;
        }
        if let Op::Push(Value::Quot(q)) = &mut i.op {
            offset_rows(Arc::make_mut(q), off);
        }
    }
}

/// Lex a library source into one instruction strip, positions shifted
/// into its row band. Infallible: bundled libraries are verified by CI.
fn lib_code(source: &str, row_band: u32) -> Vec<Instr> {
    let prog = crate::forms::parse_source(source).expect("library parses");
    let mut code = Vec::new();
    for (_, cells) in prog.strands {
        code.extend(lex_strand(cells, prog.axis).expect("library lexes"));
    }
    offset_rows(&mut code, row_band);
    code
}

/// The standard library, lexed. Infallible: std.ml is verified by CI.
fn std_code() -> Vec<Instr> {
    lib_code(STD_SOURCE, STD_ROWS)
}

// ── frame stepping ─────────────────────────────────────────────────────

/// Run up to `limit` counted steps of one strand. The counting is exactly
/// step()'s — one per executed instruction, frame retirement, or caught
/// glitch — because the deterministic schedule (and therefore the recorded
/// conformance corpus) observes it. The fast path below exists only to
/// clone the top code frame's Arc once per run instead of once per
/// instruction; it must never change what counts as a step.
pub(crate) fn run_burst(vm: &mut VM, s: &mut Strand, limit: usize) -> usize {
    let mut executed = 0;
    'outer: while executed < limit {
        if s.frames.is_empty() {
            // An unfinished ⟨ at the end of a strand is a fault, not a
            // silently successful run with a stray mark on the stack.
            if s.stack.iter().any(|v| matches!(v, Value::Mark)) {
                let pos = s.marks.last().map(|&(_, p)| p).unwrap_or((0, 0));
                s.status = Status::Dead;
                s.glitch = Some((Value::str("⟨ without matching ⟩"), pos));
                s.glitch_chain = Vec::new();
                executed += 1;
                break;
            }
            s.status = Status::Done;
            break;
        }
        let fi = s.frames.len() - 1;
        if let Frame::CF { code, ip } = &s.frames[fi] {
            let code = code.clone();
            let mut ip = *ip;
            loop {
                if ip >= code.len() {
                    s.frames.pop();
                    executed += 1;
                    continue 'outer;
                }
                match execute(vm, s, &code[ip]) {
                    Ok(()) => {
                        executed += 1;
                        ip += 1;
                        if s.frames.len() != fi + 1 {
                            // a frame was pushed — re-dispatch on the new top
                            if let Frame::CF { ip: slot, .. } = &mut s.frames[fi] {
                                *slot = ip;
                            }
                            continue 'outer;
                        }
                        if executed >= limit {
                            if let Frame::CF { ip: slot, .. } = &mut s.frames[fi] {
                                *slot = ip;
                            }
                            break 'outer;
                        }
                    }
                    Err(Sig::Yield) => {
                        // Yield completed — resume at the next instruction.
                        ip += 1;
                        executed += 1;
                        if let Frame::CF { ip: slot, .. } = &mut s.frames[fi] {
                            *slot = ip;
                        }
                        break 'outer;
                    }
                    Err(Sig::Block(on, pos)) => {
                        // Blocked ops re-execute: ip stays on this instruction.
                        if let Frame::CF { ip: slot, .. } = &mut s.frames[fi] {
                            *slot = ip;
                        }
                        s.status = Status::Blocked;
                        s.block = Some((on, pos));
                        break 'outer;
                    }
                    Err(Sig::Glitch(v, pos)) => {
                        if let Frame::CF { ip: slot, .. } = &mut s.frames[fi] {
                            *slot = ip;
                        }
                        executed += 1;
                        let chain = s.live_calls();
                        if !s.catch(v.clone()) {
                            s.status = Status::Dead;
                            s.glitch = Some((v, pos));
                            s.glitch_chain = chain;
                            break 'outer;
                        }
                        continue 'outer;
                    }
                }
            }
        } else {
            match step(vm, s) {
                Ok(()) => executed += 1,
                Err(Sig::Block(on, pos)) => {
                    s.status = Status::Blocked;
                    s.block = Some((on, pos));
                    break;
                }
                Err(Sig::Yield) => {
                    executed += 1;
                    break;
                }
                Err(Sig::Glitch(v, pos)) => {
                    executed += 1;
                    let chain = s.live_calls();
                    if !s.catch(v.clone()) {
                        s.status = Status::Dead;
                        s.glitch = Some((v, pos));
                        s.glitch_chain = chain;
                        break;
                    }
                }
            }
        }
    }
    executed
}

fn step(vm: &mut VM, s: &mut Strand) -> R<()> {
    let fi = s.frames.len() - 1;
    match &s.frames[fi] {
        Frame::CF { code, ip } => {
            let (code, ip) = (code.clone(), *ip);
            if ip >= code.len() {
                s.frames.pop();
                return Ok(());
            }
            match execute(vm, s, &code[ip]) {
                Ok(()) => {}
                Err(Sig::Yield) => {
                    // Yield completed — resume at the next instruction.
                    if let Frame::CF { ip, .. } = &mut s.frames[fi] {
                        *ip += 1;
                    }
                    return Err(Sig::Yield);
                }
                Err(e) => return Err(e), // Block re-executes; Glitch unwinds
            }
            if let Frame::CF { ip, .. } = &mut s.frames[fi] {
                *ip += 1;
            }
            Ok(())
        }
        Frame::While { .. } => {
            let (phase, cond, body, pos) = match &s.frames[fi] {
                Frame::While { phase, cond, body, pos } => {
                    (*phase, cond.clone(), body.clone(), *pos)
                }
                _ => unreachable!(),
            };
            if phase == 0 {
                if let Frame::While { phase, .. } = &mut s.frames[fi] {
                    *phase = 1;
                }
                s.frames.push(cf(cond));
            } else {
                if let Frame::While { phase, .. } = &mut s.frames[fi] {
                    *phase = 0;
                }
                let flag = s.pop_for(pos, "⟳", "the condition's result")?;
                if truthy(&flag) {
                    s.frames.push(cf(body));
                } else {
                    s.frames.pop();
                }
            }
            Ok(())
        }
        Frame::Repeat { .. } => {
            let (left, body) = match &s.frames[fi] {
                Frame::Repeat { left, body } => (*left, body.clone()),
                _ => unreachable!(),
            };
            if left <= 0 {
                s.frames.pop();
            } else {
                if let Frame::Repeat { left, .. } = &mut s.frames[fi] {
                    *left -= 1;
                }
                s.frames.push(cf(body));
            }
            Ok(())
        }
        Frame::Iter { .. } => {
            let (mode, awaiting, i, len, f, pos) = match &s.frames[fi] {
                Frame::Iter { mode, awaiting, i, items, f, pos, .. } => {
                    (*mode, *awaiting, *i, items.len(), f.clone(), *pos)
                }
                _ => unreachable!(),
            };
            if awaiting {
                if let Frame::Iter { awaiting, .. } = &mut s.frames[fi] {
                    *awaiting = false;
                }
                match mode {
                    IterMode::Map => {
                        let v = s.pop_for(pos, "map", "the body's result for each item")?;
                        if let Frame::Iter { out, .. } = &mut s.frames[fi] {
                            out.push(v);
                        }
                    }
                    IterMode::Filter => {
                        let flag = s.pop_for(pos, "filter", "the body's verdict for each item")?;
                        if truthy(&flag) {
                            if let Frame::Iter { out, items, i, .. } = &mut s.frames[fi] {
                                let item = items[*i - 1].clone();
                                out.push(item);
                            }
                        }
                    }
                    _ => {}
                }
            }
            if i >= len {
                let frame = s.frames.pop().unwrap();
                if let Frame::Iter { out, mode, .. } = frame {
                    if matches!(mode, IterMode::Map | IterMode::Filter) {
                        s.push(Value::List(Arc::new(out)));
                    }
                }
                return Ok(());
            }
            let item = match &s.frames[fi] {
                Frame::Iter { items, .. } => items[i].clone(),
                _ => unreachable!(),
            };
            s.push(item);
            if let Frame::Iter { i, awaiting, .. } = &mut s.frames[fi] {
                *i += 1;
                *awaiting = true;
            }
            s.frames.push(cf(f));
            Ok(())
        }
        Frame::Try { .. } => {
            s.frames.pop(); // body finished cleanly — disarm
            Ok(())
        }
        Frame::Drain { .. } => {
            let (chan, pos) = match &s.frames[fi] {
                Frame::Drain { chan, pos, .. } => (*chan, *pos),
                _ => unreachable!(),
            };
            let Some(v) = vm.chan_recv(chan, s.sid, &s.label, pos) else {
                return Err(Sig::Block(BlockOn::Chan(chan), pos));
            };
            if matches!(v, Value::Nil) {
                let frame = s.frames.pop().unwrap();
                if let Frame::Drain { out, .. } = frame {
                    s.push(Value::List(Arc::new(out)));
                }
            } else if let Frame::Drain { out, .. } = &mut s.frames[fi] {
                out.push(v);
            }
            Ok(())
        }
        Frame::Pump { .. } => {
            let (src, dst, f, phase, pos) = match &s.frames[fi] {
                Frame::Pump { src, dst, f, phase, pos } => {
                    (*src, *dst, f.clone(), *phase, *pos)
                }
                _ => unreachable!(),
            };
            if phase == 0 {
                let Some(v) = vm.chan_recv(src, s.sid, &s.label, pos) else {
                    return Err(Sig::Block(BlockOn::Chan(src), pos));
                };
                if matches!(v, Value::Nil) {
                    vm.chan_send(dst, Value::Nil);
                    s.frames.pop();
                    return Ok(());
                }
                if let Frame::Pump { phase, .. } = &mut s.frames[fi] {
                    *phase = 1;
                }
                s.push(v);
                s.frames.push(cf(f));
            } else {
                if let Frame::Pump { phase, .. } = &mut s.frames[fi] {
                    *phase = 0;
                }
                let v = s.pop(pos, "the pump body's result")?;
                vm.chan_send(dst, v);
            }
            Ok(())
        }
    }
}

// ── instruction execution ──────────────────────────────────────────────
fn execute(vm: &mut VM, s: &mut Strand, instr: &Instr) -> R<()> {
    let pos = instr.pos;
    match &instr.op {
        Op::Push(v) => {
            s.push(v.clone());
            Ok(())
        }
        Op::Name(c) => {
            let v = if let Some(v) = s.local_get(*c) {
                v.clone()
            } else if let Some(v) = vm.global_lookup(*c) {
                v
            } else {
                return glitch(format!("undefined sigil '{c}'"), pos);
            };
            if let Value::Quot(q) = v {
                s.check_depth(pos)?;
                // Record the call so a fault inside the definition can
                // name it and its call site.
                // Entries are pushed in depth order, so pruning the stale
                // tail is enough — and O(1) amortized, which matters for a
                // deep recursion racing toward MAX_FRAMES.
                while s.calls.last().is_some_and(|&(_, _, d)| d > s.frames.len()) {
                    s.calls.pop();
                }
                s.frames.push(cf(q));
                s.calls.push((*c, pos, s.frames.len()));
            } else {
                s.push(v);
            }
            Ok(())
        }
        Op::LMark => {
            s.marks.push((s.stack.len(), pos));
            s.push(Value::Mark);
            Ok(())
        }
        Op::LBuild => {
            let mut items = Vec::new();
            loop {
                match s.stack.pop() {
                    Some(Value::Mark) => {
                        s.marks.pop();
                        break;
                    }
                    Some(v) => items.push(v),
                    None => return glitch("⟩ without matching ⟨", pos),
                }
            }
            items.reverse();
            s.push(Value::List(Arc::new(items)));
            Ok(())
        }
        Op::B(ch, arg, arg2) => builtin(vm, s, *ch, *arg, *arg2, pos),
    }
}

fn builtin(vm: &mut VM, s: &mut Strand, ch: char, arg: char, arg2: char, pos: Pos) -> R<()> {
    if vm.pure && EFFECT_OPS.contains(ch) {
        return glitch(format!("{ch} has an effect — a hot definition must be pure"), pos);
    }
    match ch {
        // ── stack ──
        '∂' => {
            let v = s.pop_any(pos)?;
            s.push(v.clone());
            s.push(v);
        }
        '⇅' => {
            let b = s.pop_any(pos)?;
            let a = s.pop_any(pos)?;
            s.push(b);
            s.push(a);
        }
        '⌫' => {
            s.pop_any(pos)?;
        }
        '⊚' => {
            let b = s.pop_any(pos)?;
            let a = s.pop_any(pos)?;
            s.push(a.clone());
            s.push(b);
            s.push(a);
        }
        '⥀' => {
            let c = s.pop_any(pos)?;
            let b = s.pop_any(pos)?;
            let a = s.pop_any(pos)?;
            s.push(b);
            s.push(c);
            s.push(a);
        }
        '≢' => {
            let d = s.stack.len();
            s.push(Value::int(d as i64));
        }
        // ── arithmetic ──
        '+' | '-' | '×' | '÷' | '%' | '^' => {
            let b = s.pop_num(pos, &ch.to_string())?;
            let a = s.pop_num(pos, &ch.to_string())?;
            let r = arith(ch, &a, &b, pos)?;
            s.push(r);
        }
        '√' => {
            let v = s.pop_num(pos, "√")?;
            let x = v.as_f64().unwrap();
            if x < 0.0 {
                return glitch("√ of a negative number", pos);
            }
            s.push(Value::Float(x.sqrt()));
        }
        '⌊' | '⌈' => {
            let v = s.pop_num(pos, &ch.to_string())?;
            match v {
                Value::Int(_) | Value::Big(_) => s.push(v),
                Value::Float(f) => {
                    if !f.is_finite() {
                        return glitch(format!("{ch} of {} has no integer value", fmt(&v, false)), pos);
                    }
                    let r = if ch == '⌊' { f.floor() } else { f.ceil() };
                    let big = BigInt::from_f64(r).unwrap_or_default();
                    s.push(Value::from_big(big));
                }
                _ => unreachable!(),
            }
        }
        '±' => {
            let v = s.pop_num(pos, "±")?;
            match v {
                Value::Int(i) => match i.checked_neg() {
                    Some(r) => s.push(Value::Int(r)),
                    None => s.push(Value::from_big(-BigInt::from(i))),
                },
                Value::Big(b) => s.push(Value::from_big(-&*b)),
                Value::Float(f) => s.push(Value::Float(-f)),
                _ => unreachable!(),
            }
        }
        // ── comparison ──
        '=' | '≠' => {
            let b = s.pop_any(pos)?;
            let a = s.pop_any(pos)?;
            let eq = val_eq(&a, &b);
            s.push(Value::int(if eq == (ch == '=') { 1 } else { 0 }));
        }
        '<' | '≤' | '>' | '≥' => {
            let b = s.pop_any(pos)?;
            let a = s.pop_any(pos)?;
            let ord = if a.is_num() && b.is_num() {
                num_cmp(&a, &b)
            } else if let (Value::Str(x), Value::Str(y)) = (&a, &b) {
                x.cmp(y)
            } else {
                return glitch(
                    format!(
                        "{ch} compares two numbers or two strings, got {} {}",
                        type_name(&a),
                        type_name(&b)
                    ),
                    pos,
                );
            };
            use std::cmp::Ordering::*;
            let r = match ch {
                '<' => ord == Less,
                '≤' => ord != Greater,
                '>' => ord == Greater,
                '≥' => ord != Less,
                _ => unreachable!(),
            };
            s.push(Value::int(if r { 1 } else { 0 }));
        }
        // ── logic ──
        '∧' | '∨' | '⊻' => {
            let b = truthy(&s.pop_any(pos)?);
            let a = truthy(&s.pop_any(pos)?);
            let r = match ch {
                '∧' => a && b,
                '∨' => a || b,
                '⊻' => a != b,
                _ => unreachable!(),
            };
            s.push(Value::int(if r { 1 } else { 0 }));
        }
        '¬' => {
            let v = truthy(&s.pop_any(pos)?);
            s.push(Value::int(if v { 0 } else { 1 }));
        }
        // ── control ──
        '!' => {
            let q = s.pop_quot(pos, "!")?;
            s.check_depth(pos)?;
            s.frames.push(cf(q));
        }
        '?' => {
            let e = s.pop_any(pos)?;
            let t = s.pop_any(pos)?;
            let c = s.pop_any(pos)?;
            let pick = if truthy(&c) { t } else { e };
            if let Value::Quot(q) = pick {
                s.check_depth(pos)?;
                s.frames.push(cf(q));
            } else {
                s.push(pick);
            }
        }
        '⟳' => {
            let body = s.pop_quot(pos, "⟳")?;
            let cond = s.pop_quot(pos, "⟳")?;
            s.frames.push(Frame::While { cond, body, phase: 0, pos });
        }
        '⍣' => {
            let body = s.pop_quot(pos, "⍣")?;
            let n = s.pop_i64(pos, "⍣")?;
            s.frames.push(Frame::Repeat { left: n, body });
        }
        // ── iteration ──
        '∵' | '∀' | '⌿' => {
            let mode = match ch {
                '∵' => IterMode::Map,
                '∀' => IterMode::Each,
                _ => IterMode::Filter,
            };
            let name = match ch {
                '∵' => "map",
                '∀' => "each",
                _ => "filter",
            };
            let f = s.pop_quot(pos, name)?;
            let items = s.pop_seq(pos, name)?;
            s.frames.push(Frame::Iter { items, i: 0, f, mode, out: Vec::new(), awaiting: false, pos });
        }
        '⍀' => {
            let f = s.pop_quot(pos, "⍀")?;
            let acc = s.pop(pos, "a fold seed")?;
            let items = s.pop_seq(pos, "⍀")?;
            s.push(acc);
            s.frames.push(Frame::Iter {
                items,
                i: 0,
                f,
                mode: IterMode::Fold,
                out: Vec::new(),
                awaiting: false,
                pos,
            });
        }
        '⍸' => {
            let n = s.pop_i64(pos, "⍸")?;
            if n > MAX_RANGE {
                return glitch(
                    format!("⍸ {} is too many items (limit {MAX_RANGE})", fmt_i64(n)),
                    pos,
                );
            }
            let items: Vec<Value> = (0..n.max(0)).map(Value::int).collect();
            s.push(Value::List(Arc::new(items)));
        }
        // ── sequences ──
        '#' => {
            let v = s.pop(pos, "a list or string")?;
            let n = match &v {
                Value::Str(x) => x.chars().count(),
                Value::List(l) => l.len(),
                _ => {
                    return glitch(
                        format!("# expects a list or string, got {}", type_name(&v)),
                        pos,
                    )
                }
            };
            s.push(Value::int(n as i64));
        }
        '⧺' => {
            let b = s.pop_any(pos)?;
            let a = s.pop_any(pos)?;
            match (&a, &b) {
                (Value::Str(x), Value::Str(y)) => s.push(Value::str(format!("{x}{y}"))),
                (Value::List(x), Value::List(y)) => {
                    let mut v = x.as_ref().clone();
                    v.extend(y.iter().cloned());
                    s.push(Value::List(Arc::new(v)));
                }
                _ => {
                    return glitch(
                        format!(
                            "⧺ joins two strings or two lists, got {} {}",
                            type_name(&a),
                            type_name(&b)
                        ),
                        pos,
                    )
                }
            }
        }
        '@' => {
            let i = s.pop_i64(pos, "@")?;
            let v = s.pop(pos, "a list or string")?;
            let len = match &v {
                Value::Str(x) => x.chars().count(),
                Value::List(l) => l.len(),
                _ => {
                    return glitch(
                        format!("@ expects a list or string, got {}", type_name(&v)),
                        pos,
                    )
                }
            };
            if i < 0 || i as usize >= len {
                return glitch(
                    format!("@ index {} out of bounds (length {len})", fmt_i64(i)),
                    pos,
                );
            }
            match &v {
                Value::Str(x) => {
                    s.push(Value::str(x.chars().nth(i as usize).unwrap().to_string()))
                }
                Value::List(l) => s.push(l[i as usize].clone()),
                _ => unreachable!(),
            }
        }
        '⊂' => {
            let j = s.pop_i64(pos, "⊂")?.max(0) as usize;
            let i = s.pop_i64(pos, "⊂")?.max(0) as usize;
            let v = s.pop(pos, "a list or string")?;
            match &v {
                Value::Str(x) => {
                    let chars: Vec<char> = x.chars().collect();
                    let i = i.min(chars.len());
                    let j = j.min(chars.len()).max(i);
                    s.push(Value::str(chars[i..j].iter().collect::<String>()));
                }
                Value::List(l) => {
                    let i = i.min(l.len());
                    let j = j.min(l.len()).max(i);
                    s.push(Value::List(Arc::new(l[i..j].to_vec())));
                }
                _ => {
                    return glitch(
                        format!("⊂ expects a list or string, got {}", type_name(&v)),
                        pos,
                    )
                }
            }
        }
        '⊆' => {
            let sep = s.pop(pos, "a separator string")?;
            let v = s.pop(pos, "a string")?;
            match (&v, &sep) {
                (Value::Str(x), Value::Str(y)) => {
                    let parts: Vec<Value> = if y.is_empty() {
                        x.chars().map(|c| Value::str(c.to_string())).collect()
                    } else {
                        x.split(y.as_str()).map(Value::str).collect()
                    };
                    s.push(Value::List(Arc::new(parts)));
                }
                _ => {
                    return glitch(
                        format!(
                            "⊆ expects string sep-string, got {} {}",
                            type_name(&v),
                            type_name(&sep)
                        ),
                        pos,
                    )
                }
            }
        }
        '⊇' => {
            let sep = s.pop(pos, "a separator string")?;
            let items = s.pop_seq(pos, "⊇")?;
            let Value::Str(sep) = &sep else {
                return glitch(
                    format!("⊇ expects a string separator, got {}", type_name(&sep)),
                    pos,
                );
            };
            let joined: Vec<String> = items.iter().map(|x| fmt(x, false)).collect();
            s.push(Value::str(joined.join(sep)));
        }
        '⍕' => {
            let v = s.pop_any(pos)?;
            s.push(Value::str(fmt(&v, false)));
        }
        '⍎' => {
            let v = s.pop(pos, "a string")?;
            let Value::Str(x) = &v else {
                return glitch(format!("⍎ expects a string, got {}", type_name(&v)), pos);
            };
            let t = x.trim().replace('¯', "-");
            let parsed = if t.contains('.') || t.contains('e') || t.contains('E') {
                t.parse::<f64>().ok().map(Value::Float)
            } else {
                t.parse::<i64>()
                    .ok()
                    .map(Value::Int)
                    .or_else(|| t.parse::<BigInt>().ok().map(Value::from_big))
            };
            match parsed {
                Some(n) => s.push(n),
                None => return glitch(format!("⍎ cannot parse «{x}» as a number"), pos),
            }
        }
        '⌗' => {
            let v = s.pop(pos, "a 1-char string")?;
            match &v {
                Value::Str(x) if x.chars().count() == 1 => {
                    s.push(Value::int(x.chars().next().unwrap() as i64));
                }
                _ => return glitch("⌗ expects a 1-character string", pos),
            }
        }
        '⍘' => {
            let n = s.pop_i64(pos, "⍘")?;
            match u32::try_from(n).ok().and_then(char::from_u32) {
                Some(c) => s.push(Value::str(c.to_string())),
                None => {
                    return glitch(
                        format!("⍘ code point {} out of range", fmt_i64(n)),
                        pos,
                    )
                }
            }
        }
        // ── inspection & rearrangement ──
        '⍙' => {
            let Some(top) = s.stack.last() else {
                return glitch("stack underflow — needed a value", pos);
            };
            let name = type_name(top);
            s.push(Value::str(name));
        }
        '⌽' => {
            let v = s.pop(pos, "a list or string")?;
            match &v {
                Value::Str(x) => s.push(Value::str(x.chars().rev().collect::<String>())),
                Value::List(l) => {
                    s.push(Value::List(Arc::new(l.iter().rev().cloned().collect())))
                }
                _ => {
                    return glitch(
                        format!("⌽ expects a list or string, got {}", type_name(&v)),
                        pos,
                    )
                }
            }
        }
        '⍋' => {
            let v = s.pop(pos, "a list or string")?;
            match &v {
                Value::Str(x) => {
                    let mut chars: Vec<char> = x.chars().collect();
                    chars.sort();
                    s.push(Value::str(chars.into_iter().collect::<String>()));
                }
                Value::List(l) => {
                    let nums = l.iter().all(|x| x.is_num());
                    let strs = l.iter().all(|x| matches!(x, Value::Str(_)));
                    if nums {
                        let mut v2 = l.as_ref().clone();
                        v2.sort_by(|a, b| num_cmp(a, b));
                        s.push(Value::List(Arc::new(v2)));
                    } else if strs {
                        let mut v2 = l.as_ref().clone();
                        v2.sort_by(|a, b| match (a, b) {
                            (Value::Str(x), Value::Str(y)) => x.cmp(y),
                            _ => unreachable!(),
                        });
                        s.push(Value::List(Arc::new(v2)));
                    } else {
                        return glitch("⍋ needs all numbers or all strings", pos);
                    }
                }
                _ => {
                    return glitch(
                        format!("⍋ expects a list or string, got {}", type_name(&v)),
                        pos,
                    )
                }
            }
        }
        '∈' => {
            let v = s.pop_any(pos)?;
            let seq = s.pop(pos, "a list or string")?;
            match &seq {
                Value::Str(x) => {
                    let Value::Str(needle) = &v else {
                        return glitch(
                            format!("∈ searching a string needs a string, got {}", type_name(&v)),
                            pos,
                        );
                    };
                    s.push(Value::int(if x.contains(needle.as_str()) { 1 } else { 0 }));
                }
                Value::List(l) => {
                    let found = l.iter().any(|x| val_eq(x, &v));
                    s.push(Value::int(if found { 1 } else { 0 }));
                }
                _ => {
                    return glitch(
                        format!("∈ expects a list or string, got {}", type_name(&seq)),
                        pos,
                    )
                }
            }
        }
        '⍷' => {
            let v = s.pop_any(pos)?;
            let seq = s.pop(pos, "a list or string")?;
            match &seq {
                Value::Str(x) => {
                    let Value::Str(needle) = &v else {
                        return glitch(
                            format!("⍷ searching a string needs a string, got {}", type_name(&v)),
                            pos,
                        );
                    };
                    match x.find(needle.as_str()) {
                        Some(byte_idx) => {
                            let ci = x[..byte_idx].chars().count();
                            s.push(Value::int(ci as i64));
                        }
                        None => s.push(Value::int(-1)),
                    }
                }
                Value::List(l) => {
                    match l.iter().position(|x| val_eq(x, &v)) {
                        Some(i) => s.push(Value::int(i as i64)),
                        None => s.push(Value::int(-1)),
                    }
                }
                _ => {
                    return glitch(
                        format!("⍷ expects a list or string, got {}", type_name(&seq)),
                        pos,
                    )
                }
            }
        }
        // ── bindings ──
        '≔' => {
            if vm.global_lookup(arg).is_some() {
                return glitch(format!("sigil '{arg}' is already defined"), pos);
            }
            let v = s.pop(pos, "a value to bind")?;
            if let Some(bus) = &vm.bus {
                if !bus.global_define(arg, v.clone()) {
                    return glitch(format!("sigil '{arg}' is already defined"), pos);
                }
            }
            vm.globals.insert(arg, v);
        }
        '⇒' => {
            let v = s.pop(pos, "a value to store")?;
            s.local_set(arg, v);
        }
        // ── strands & channels ──
        '↥' => {
            let v = s.pop(pos, "a value to send")?;
            vm.chan_send(arg, v);
        }
        '↧' => {
            match vm.chan_recv(arg, s.sid, &s.label, pos) {
                Some(v) => s.push(v),
                None => return Err(Sig::Block(BlockOn::Chan(arg), pos)),
            }
        }
        '⇂' => {
            match vm.chan_try_recv(arg) {
                Some(v) => {
                    s.push(v);
                    s.push(Value::int(1));
                }
                None => s.push(Value::int(0)),
            }
        }
        '⇈' => {
            let items = s.pop_seq(pos, "⇈")?;
            for v in items.iter() {
                vm.chan_send(arg, v.clone());
            }
            vm.chan_send(arg, Value::Nil);
        }
        '⇟' => {
            s.frames.push(Frame::Drain { chan: arg, out: Vec::new(), pos });
        }
        '⇉' => {
            let f = s.pop_quot(pos, "⇉")?;
            s.frames.push(Frame::Pump { src: arg, dst: arg2, f, phase: 0, pos });
        }
        '⚡' => {
            let q = s.pop_quot(pos, "⚡")?;
            let label = format!("⚡ of strand {}", fmt_i64(s.sid));
            let sid = if let Some(bus) = &vm.bus {
                bus.clone().spawn(label, q, s.locals.clone())
            } else {
                let sid = vm.next_spawn_sid;
                vm.next_spawn_sid += 1;
                vm.register(Strand::new(sid, label, q, s.locals.clone()));
                sid
            };
            s.push(Value::int(sid));
        }
        '⋈' => {
            // Peek, don't pop: blocked ops re-execute.
            let Some(top) = s.stack.last() else {
                return glitch("stack underflow — ⋈ needs a strand id", pos);
            };
            if !top.is_num() {
                return glitch(
                    format!("⋈ expects a strand id, got {}", type_name(top)),
                    pos,
                );
            }
            let sid = top.as_f64().unwrap() as i64;
            if let Some(bus) = &vm.bus {
                let bus = bus.clone();
                if !bus.knows_strand(sid) {
                    return glitch(format!("⋈ no strand with id {}", fmt_i64(sid)), pos);
                }
                let _ = vm.out.flush();
                bus.join_wait(sid, s.sid, &s.label, pos);
            } else {
                let Some(&t) = vm.by_sid.get(&sid) else {
                    return glitch(format!("⋈ no strand with id {}", fmt_i64(sid)), pos);
                };
                if !matches!(vm.strands[t].status, Status::Done | Status::Dead) {
                    return Err(Sig::Block(BlockOn::Strand(sid), pos));
                }
            }
            s.stack.pop();
        }
        '⍳' => s.push(Value::int(s.sid)),
        '≣' => s.push(Value::int(vm.main_count as i64)),
        '⌛' => return Err(Sig::Yield),
        // ── glitches ──
        '⍥' => {
            let handler = s.pop_quot(pos, "⍥")?;
            let body = s.pop_quot(pos, "⍥")?;
            let depth = s.stack.len();
            s.frames.push(Frame::Try { handler, depth });
            s.frames.push(cf(body));
        }
        '↯' => {
            let v = s.pop(pos, "a value to raise")?;
            return Err(Sig::Glitch(v, pos));
        }
        // ── the canvas ──
        '⌸' => {
            if vm.bus.is_some() {
                return glitch("⌸ needs the deterministic scheduler — drop --parallel", pos);
            }
            if vm.gui.is_some() {
                return glitch("⌸ — a canvas is already open", pos);
            }
            let t = s.pop(pos, "a window title")?;
            let Value::Str(title) = &t else {
                return glitch(
                    format!("⌸ expects a title string, got {}", type_name(&t)),
                    pos,
                );
            };
            let h = s.pop_i64(pos, "⌸")?;
            let w = s.pop_i64(pos, "⌸")?;
            if !(1..=4096).contains(&w) || !(1..=4096).contains(&h) {
                return glitch("⌸ size must be 1…4096 pixels on each side", pos);
            }
            let gui = crate::gui::Gui::open(w as usize, h as usize, title, vm.force_headless);
            if !gui.is_windowed() {
                let _ = writeln!(vm.out, "⌸ {w}×{h} «{title}»");
            }
            vm.gui = Some(gui);
        }
        '▦' => {
            let color = s.pop_i64(pos, "▦")?;
            let rh = s.pop_i64(pos, "▦")?;
            let rw = s.pop_i64(pos, "▦")?;
            let y = s.pop_i64(pos, "▦")?;
            let x = s.pop_i64(pos, "▦")?;
            let Some(gui) = vm.gui.as_mut() else {
                return glitch("▦ — no canvas; open one with ⌸ first", pos);
            };
            gui.rect(x, y, rw, rh, (color & 0xff_ffff) as u32);
        }
        '⌶' => {
            let color = s.pop_i64(pos, "⌶")?;
            let y = s.pop_i64(pos, "⌶")?;
            let x = s.pop_i64(pos, "⌶")?;
            let v = s.pop_any(pos)?;
            let Some(gui) = vm.gui.as_mut() else {
                return glitch("⌶ — no canvas; open one with ⌸ first", pos);
            };
            gui.text(&fmt(&v, false), x, y, (color & 0xff_ffff) as u32);
        }
        '⎙' => {
            let Some(gui) = vm.gui.as_mut() else {
                return glitch("⎙ — no canvas; open one with ⌸ first", pos);
            };
            if let Err(e) = gui.present(&mut *vm.out) {
                return glitch(format!("⎙ {e}"), pos);
            }
        }
        '⌹' => {
            let v = s.pop(pos, "a directory path")?;
            let Value::Str(path) = &v else {
                return glitch(
                    format!("⌹ expects a path string, got {}", type_name(&v)),
                    pos,
                );
            };
            let Ok(rd) = std::fs::read_dir(path.as_str()) else {
                return glitch(format!("⌹ cannot read «{path}»"), pos);
            };
            // Sorted, directories marked with a trailing /: the listing is
            // deterministic for a fixed tree, like every other observable.
            let mut names: Vec<String> = Vec::new();
            for entry in rd.flatten() {
                let mut name = entry.file_name().to_string_lossy().into_owned();
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    name.push('/');
                }
                names.push(name);
            }
            names.sort();
            s.push(Value::List(Arc::new(
                names.into_iter().map(Value::str).collect(),
            )));
        }
        // ── i/o ──
        '⍞' => {
            let v = s.pop_any(pos)?;
            let _ = writeln!(vm.out, "{}", fmt(&v, false));
        }
        '⊸' => {
            let v = s.pop_any(pos)?;
            let _ = write!(vm.out, "{}", fmt(&v, false));
        }
        '⍇' => {
            let v = s.pop(pos, "a file path")?;
            let Value::Str(path) = &v else {
                return glitch(
                    format!("⍇ expects a path string, got {}", type_name(&v)),
                    pos,
                );
            };
            match std::fs::read_to_string(path.as_str()) {
                Ok(text) => s.push(Value::str(text)),
                // No OS detail in the message: glitches are part of the
                // language's deterministic, conformance-pinned output.
                Err(_) => return glitch(format!("⍇ cannot read «{path}»"), pos),
            }
        }
        '⍈' => {
            let pth = s.pop(pos, "a file path")?;
            let Value::Str(path) = &pth else {
                return glitch(
                    format!("⍈ expects a path string, got {}", type_name(&pth)),
                    pos,
                );
            };
            let content = s.pop(pos, "a string to write")?;
            let Value::Str(text) = &content else {
                return glitch(
                    format!("⍈ expects a string to write, got {}", type_name(&content)),
                    pos,
                );
            };
            if std::fs::write(path.as_str(), text.as_bytes()).is_err() {
                return glitch(format!("⍈ cannot write «{path}»"), pos);
            }
        }
        '⍆' => {
            let v = s.pop(pos, "a url")?;
            let Value::Str(url) = &v else {
                return glitch(
                    format!("⍆ expects a url string, got {}", type_name(&v)),
                    pos,
                );
            };
            // The network is part of a run's input, like files and argv:
            // identical responses produce identical runs. A fetch carries a
            // hard deadline — it either delivers or glitches, never hangs —
            // and glitch messages name only the url and the HTTP status,
            // never an operating-system error string.
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                return glitch(format!("⍆ cannot fetch «{url}»"), pos);
            }
            match fetch_url(url) {
                Ok(body) => s.push(Value::str(body)),
                Err(Some(status)) => {
                    return glitch(format!("⍆ «{url}» answered {status}"), pos)
                }
                Err(None) => return glitch(format!("⍆ cannot fetch «{url}»"), pos),
            }
        }
        '⎆' => {
            // Accepting a request shares ⌨'s lowest scheduling priority:
            // the whole grid goes quiet — every pending response written —
            // before the server waits on the outside world.
            if vm.others_active(s.sid) {
                return Err(Sig::Block(BlockOn::Stdin, pos));
            }
            let _ = vm.out.flush();
            // Hot patches travel in the request stream (SPEC §4.7). The
            // runtime applies each one as it is pulled and answers it
            // itself; the program only ever sees requests.
            let accepted = loop {
                if let Some(bridge) = &vm.http {
                    let bridge = bridge.clone();
                    match bridge.accept() {
                        crate::http::Incoming::Request(r) => break Some(r),
                        crate::http::Incoming::Patch { id, base, text } => {
                            let (status, body) = match vm.hot_patch(base, &text, Some(s)) {
                                Ok(report) => (200, report),
                                Err((status, why)) => (i64::from(status), why),
                            };
                            let _ = vm.err.write_all(body.as_bytes());
                            let _ = vm.err.flush();
                            bridge.respond(id, status, "text/plain; charset=utf-8", &body);
                            if s.pending.is_some() {
                                // The patch touches this very strand. ⎆ has
                                // consumed nothing, so step back and let the
                                // scheduler re-weave it before the next
                                // request is served on the old code.
                                return Err(Sig::Block(BlockOn::Stdin, pos));
                            }
                        }
                    }
                } else if let Some(bus) = &vm.bus {
                    let bus = bus.clone();
                    match bus.read_request() {
                        Ok(r) => break r,
                        Err(bad) => return glitch(format!("⎆ bad request frame «{bad}»"), pos),
                    }
                } else {
                    match vm.read_request_frame() {
                        Ok(Some(crate::http::Frame::Request(method, path, body))) => {
                            let id = vm.next_request_id;
                            vm.next_request_id += 1;
                            vm.open_requests.insert(id);
                            break Some((id, method, path, body));
                        }
                        Ok(Some(crate::http::Frame::Patch { base, text })) => {
                            let (status, body) = match vm.hot_patch(base, &text, Some(s)) {
                                Ok(report) => (200, report),
                                Err((status, why)) => (status, why),
                            };
                            let frame = crate::http::write_patch_framed(status, &body);
                            let _ = vm.out.write_all(frame.as_bytes());
                            if s.pending.is_some() {
                                return Err(Sig::Block(BlockOn::Stdin, pos));
                            }
                        }
                        Ok(None) => break None,
                        Err(bad) => return glitch(format!("⎆ bad request frame «{bad}»"), pos),
                    }
                }
            };
            match accepted {
                Some((id, method, path, body)) => s.push(Value::List(Arc::new(vec![
                    Value::int(id),
                    Value::str(method),
                    Value::str(path),
                    Value::str(body),
                ]))),
                None => s.push(Value::Nil),
            }
        }
        '⍅' => {
            let v = s.pop(pos, "a ⟨id status type body⟩ response")?;
            let Value::List(items) = &v else {
                return glitch(
                    format!("⍅ expects ⟨id status type body⟩, got {}", type_name(&v)),
                    pos,
                );
            };
            let (Some(Value::Int(id)), Some(Value::Int(status)), Some(Value::Str(ctype)), Some(Value::Str(body))) =
                (items.first(), items.get(1), items.get(2), items.get(3))
            else {
                return glitch("⍅ expects ⟨id status type body⟩", pos);
            };
            if items.len() != 4 {
                return glitch("⍅ expects ⟨id status type body⟩", pos);
            }
            let (id, status) = (*id, *status);
            if let Err(why) = crate::http::validate_response(status, ctype) {
                return glitch(why, pos);
            }
            if let Some(bridge) = &vm.http {
                let bridge = bridge.clone();
                if !bridge.respond(id, status, ctype, body) {
                    return glitch(format!("⍅ no pending request {}", fmt_i64(id)), pos);
                }
            } else {
                let known = if let Some(bus) = &vm.bus {
                    bus.close_request(id)
                } else {
                    vm.open_requests.remove(&id)
                };
                if !known {
                    return glitch(format!("⍅ no pending request {}", fmt_i64(id)), pos);
                }
                let frame = crate::http::write_framed(id, status, ctype, body);
                let _ = vm.out.write_all(frame.as_bytes());
            }
        }
        '⌨' => {
            // Stdin has the lowest scheduling priority: the read happens
            // only once no other strand can make progress, so a pipeline
            // flushes its pending work — greetings, prompts, responses —
            // before the program waits on the user. The interleaving stays
            // deterministic because it never depends on input timing.
            if vm.others_active(s.sid) {
                return Err(Sig::Block(BlockOn::Stdin, pos));
            }
            // An interactive prompt written with ⊸ must be visible before
            // the program blocks on input.
            let _ = vm.out.flush();
            let mut line = String::new();
            let n = match &vm.bus {
                Some(bus) => bus.read_line(&mut line),
                None => vm.stdin.read_line(&mut line).unwrap_or(0),
            };
            if n == 0 {
                s.push(Value::Nil);
            } else {
                if line.ends_with('\n') {
                    line.pop();
                }
                // Windows consoles hand lines to programs CRLF-terminated;
                // the terminator is not part of the line's content.
                if line.ends_with('\r') {
                    line.pop();
                }
                s.push(Value::str(line));
            }
        }
        '⌥' => {
            // Stdin reads share ⌨'s lowest scheduling priority: other
            // strands flush their pending work before the UI waits.
            if vm.others_active(s.sid) {
                return Err(Sig::Block(BlockOn::Stdin, pos));
            }
            // Prompts written with ⊸ must appear before blocking, like ⌨.
            let _ = vm.out.flush();
            // A windowed canvas owns the input: events come from its
            // keyboard and mouse. Headless (and windowless) programs keep
            // reading the stdin byte stream, which is what recorded
            // goldens replay.
            let event = match vm.gui.as_mut() {
                Some(gui) if gui.is_windowed() => gui.wait_event(),
                _ => vm.read_event(),
            };
            s.push(event);
        }
        '⌂' => {
            let items: Vec<Value> = vm.args.iter().map(|a| Value::str(a.clone())).collect();
            s.push(Value::List(Arc::new(items)));
        }
        '⍜' => {
            let (rows, cols) = crate::term::size();
            s.push(Value::List(Arc::new(vec![Value::int(rows), Value::int(cols)])));
        }
        '⌚' => {
            let ms = vm.clock.unwrap_or_else(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0)
            });
            s.push(Value::int(ms));
        }
        '⍟' => {
            let items: Vec<String> = s.stack.iter().map(|v| fmt(v, true)).collect();
            let _ = writeln!(
                vm.err,
                "⍟ strand {} ({}): {}",
                fmt_i64(s.sid),
                s.label,
                items.join(" ")
            );
        }
        _ => unreachable!("op {ch} has no implementation"),
    }
    Ok(())
}

/// The MLANG_CLOCK environment variable pins ⌚ to a fixed millisecond
/// timestamp — the clock, like files and argv, is part of a run's input.
fn clock_env() -> Option<i64> {
    std::env::var("MLANG_CLOCK").ok().and_then(|v| v.parse().ok())
}

/// One HTTP(S) GET for ⍆. Ok(body) on 2xx; Err(Some(status)) when the
/// server answered with an error status; Err(None) for everything else —
/// transport failure, timeout, oversize, or a body that is not UTF-8.
/// The 10-second deadline is absolute: a fetch can never hang a strand
/// forever. Proxies come from the standard HTTPS_PROXY / HTTP_PROXY
/// environment variables; trust roots from the platform store (and
/// SSL_CERT_FILE), so corporate middleboxes work without configuration.
fn fetch_url(url: &str) -> Result<String, Option<u16>> {
    const DEADLINE: std::time::Duration = std::time::Duration::from_secs(10);
    const MAX_BODY: u64 = 16 * 1024 * 1024;
    let mut builder = ureq::AgentBuilder::new().timeout(DEADLINE);
    if let Some(proxy) = ["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"]
        .iter()
        .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
    {
        if let Ok(p) = ureq::Proxy::new(&proxy) {
            builder = builder.proxy(p);
        }
    }
    let response = builder
        .build()
        .get(url)
        .set("User-Agent", "mlang/0.1")
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(code, _) => Some(code),
            ureq::Error::Transport(_) => None,
        })?;
    let mut body = Vec::new();
    use std::io::Read;
    response
        .into_reader()
        .take(MAX_BODY + 1)
        .read_to_end(&mut body)
        .map_err(|_| None)?;
    if body.len() as u64 > MAX_BODY {
        return Err(None);
    }
    String::from_utf8(body).map_err(|_| None)
}

use num_traits::FromPrimitive;
