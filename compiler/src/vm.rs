//! The native MLang virtual machine — semantics identical to the reference
//! implementation, including the deterministic scheduler (SLICE = 8) and
//! every diagnostic message. Same program + same input ⇒ same bytes out.

use crate::forms::Program;
use crate::lex::{lex_strand, LoadError};
use crate::values::{fmt, fmt_i64, truthy, type_name, val_eq, Instr, Op, Pos, Value};
use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{BufRead, Write};
use std::sync::Arc;

const SLICE: usize = 8;

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
}

impl Strand {
    pub(crate) fn new(sid: i64, label: String, code: Arc<Vec<Instr>>, locals: Vec<(char, Value)>) -> Self {
        Strand {
            sid,
            label,
            frames: vec![cf(code)],
            stack: Vec::new(),
            locals,
            status: Status::Run,
            block: None,
            glitch: None,
            glitch_chain: Vec::new(),
            calls: Vec::new(),
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
                    match y.to_u32() {
                        Some(e) => Value::from_big(x.pow(e)),
                        None => return glitch("^ exponent too large", pos),
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
    /// The program's physical source lines, for report excerpts. Empty
    /// when the source is unavailable (payloads from older toolchains).
    src_lines: Vec<String>,
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

/// Library code carries its positions in high row bands so a report can
/// name the source it points into: std.ml rows live at +STD_ROWS, ui.ml
/// rows at +UI_ROWS, json.ml rows at +JSON_ROWS. Program rows are
/// untouched.
pub const STD_ROWS: u32 = 1_000_000;
pub const UI_ROWS: u32 = 2_000_000;
pub const JSON_ROWS: u32 = 3_000_000;

/// Split a position into (source label, display row, col).
fn pos_origin(pos: Pos) -> (&'static str, u32, u32) {
    match pos.0 {
        r if r >= JSON_ROWS => ("json.ml ", r - JSON_ROWS, pos.1),
        r if r >= UI_ROWS => ("ui.ml ", r - UI_ROWS, pos.1),
        r if r >= STD_ROWS => ("std.ml ", r - STD_ROWS, pos.1),
        r => ("", r, pos.1),
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
    let mut out = String::new();
    if let Some(x) = excerpt(source, pos) {
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
        let (src, row, col) = pos_origin(pos);
        format!("{src}{row}:{col}")
    }
}

/// Render a two-line source excerpt for a position: the (windowed) line
/// and a caret marking the exact glyph. `lines` are the program's physical
/// source lines; library positions resolve against the bundled sources.
/// Returns None when the position is unlocatable (e.g. a payload built by
/// an older toolchain, or eval'd code no longer at hand).
pub fn excerpt(lines: &[String], pos: Pos) -> Option<String> {
    if pos == (0, 0) {
        return None;
    }
    let (src, row, col) = pos_origin(pos);
    let line: String = match src {
        "" => lines.get(row.checked_sub(1)? as usize)?.clone(),
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
            src_lines: Vec::new(),
            chan_sites: HashMap::new(),
            gui: None,
            force_headless: false,
        }
    }

    /// Replay-mode ⎆: read one request frame from this VM's own stdin.
    fn read_request_frame(&mut self) -> Result<Option<(String, String, String)>, String> {
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
        let detail = fault_detail(&self.src_lines, *pos, &s.glitch_chain, s.stack_view());
        let _ = write!(self.err, "{detail}");
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
            if let Some(x) = excerpt(&self.src_lines, pos) {
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
        let _ = write!(self.err, "{}", channel_census(&self.chan_sites, &waited));
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
        self.src_lines = prog.source.clone();
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
        if self.failed {
            1
        } else {
            0
        }
    }
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
            let (phase, cond, body) = match &s.frames[fi] {
                Frame::While { phase, cond, body } => (*phase, cond.clone(), body.clone()),
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
                let flag = s.pop_any((0, 0))?;
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
            let (mode, awaiting, i, len, f) = match &s.frames[fi] {
                Frame::Iter { mode, awaiting, i, items, f, .. } => {
                    (*mode, *awaiting, *i, items.len(), f.clone())
                }
                _ => unreachable!(),
            };
            if awaiting {
                if let Frame::Iter { awaiting, .. } = &mut s.frames[fi] {
                    *awaiting = false;
                }
                match mode {
                    IterMode::Map => {
                        let v = s.pop_any((0, 0))?;
                        if let Frame::Iter { out, .. } = &mut s.frames[fi] {
                            out.push(v);
                        }
                    }
                    IterMode::Filter => {
                        let flag = s.pop_any((0, 0))?;
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
                // Record the call so a fault inside the definition can
                // name it and its call site.
                s.calls.retain(|&(_, _, depth)| depth <= s.frames.len());
                s.frames.push(cf(q));
                s.calls.push((*c, pos, s.frames.len()));
            } else {
                s.push(v);
            }
            Ok(())
        }
        Op::LMark => {
            s.push(Value::Mark);
            Ok(())
        }
        Op::LBuild => {
            let mut items = Vec::new();
            loop {
                match s.stack.pop() {
                    Some(Value::Mark) => break,
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
            s.frames.push(cf(q));
        }
        '?' => {
            let e = s.pop_any(pos)?;
            let t = s.pop_any(pos)?;
            let c = s.pop_any(pos)?;
            let pick = if truthy(&c) { t } else { e };
            if let Value::Quot(q) = pick {
                s.frames.push(cf(q));
            } else {
                s.push(pick);
            }
        }
        '⟳' => {
            let body = s.pop_quot(pos, "⟳")?;
            let cond = s.pop_quot(pos, "⟳")?;
            s.frames.push(Frame::While { cond, body, phase: 0 });
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
            s.frames.push(Frame::Iter { items, i: 0, f, mode, out: Vec::new(), awaiting: false });
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
            });
        }
        '⍸' => {
            let n = s.pop_i64(pos, "⍸")?;
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
            let accepted = if let Some(bridge) = &vm.http {
                let bridge = bridge.clone();
                Some(bridge.accept())
            } else if let Some(bus) = &vm.bus {
                let bus = bus.clone();
                match bus.read_request() {
                    Ok(r) => r,
                    Err(bad) => return glitch(format!("⎆ bad request frame «{bad}»"), pos),
                }
            } else {
                match vm.read_request_frame() {
                    Ok(Some((method, path, body))) => {
                        let id = vm.next_request_id;
                        vm.next_request_id += 1;
                        vm.open_requests.insert(id);
                        Some((id, method, path, body))
                    }
                    Ok(None) => None,
                    Err(bad) => return glitch(format!("⎆ bad request frame «{bad}»"), pos),
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
