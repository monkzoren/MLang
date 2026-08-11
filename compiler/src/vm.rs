//! The native MLang virtual machine — semantics identical to the reference
//! implementation, including the deterministic scheduler (SLICE = 8) and
//! every diagnostic message. Same program + same input ⇒ same bytes out.

use crate::forms::Program;
use crate::lex::{lex_strand, LoadError};
use crate::values::{fmt, fmt_i64, truthy, type_name, val_eq, Instr, Op, Pos, Value};
use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};
use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, Write};
use std::rc::Rc;

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
        code: Rc<Vec<Instr>>,
        ip: usize,
    },
    While {
        cond: Rc<Vec<Instr>>,
        body: Rc<Vec<Instr>>,
        phase: u8,
    },
    Repeat {
        left: i64,
        body: Rc<Vec<Instr>>,
    },
    Iter {
        items: Rc<Vec<Value>>,
        i: usize,
        f: Rc<Vec<Instr>>,
        mode: IterMode,
        out: Vec<Value>,
        awaiting: bool,
    },
    Try {
        handler: Rc<Vec<Instr>>,
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
        f: Rc<Vec<Instr>>,
        phase: u8,
        pos: Pos,
    },
}

fn cf(code: Rc<Vec<Instr>>) -> Frame {
    Frame::CF { code, ip: 0 }
}

pub struct Strand {
    pub sid: i64,
    pub label: String,
    frames: Vec<Frame>,
    stack: Vec<Value>,
    locals: HashMap<char, Value>,
    pub status: Status,
    pub block: Option<(BlockOn, Pos)>,
    pub glitch: Option<(Value, Pos)>,
}

impl Strand {
    fn new(sid: i64, label: String, code: Rc<Vec<Instr>>, locals: HashMap<char, Value>) -> Self {
        Strand {
            sid,
            label,
            frames: vec![cf(code)],
            stack: Vec::new(),
            locals,
            status: Status::Run,
            block: None,
            glitch: None,
        }
    }

    fn placeholder() -> Self {
        Strand::new(i64::MIN, String::new(), Rc::new(Vec::new()), HashMap::new())
    }

    fn push(&mut self, v: Value) {
        self.stack.push(v);
    }

    fn pop(&mut self, pos: Pos, what: &str) -> R<Value> {
        match self.stack.pop() {
            Some(v) => Ok(v),
            None => glitch(format!("stack underflow — needed {what}"), pos),
        }
    }

    fn pop_any(&mut self, pos: Pos) -> R<Value> {
        self.pop(pos, "a value")
    }

    fn pop_num(&mut self, pos: Pos, op: &str) -> R<Value> {
        let v = self.pop(pos, "a number")?;
        if !v.is_num() {
            return glitch(format!("{op} expects numbers, got {}", type_name(&v)), pos);
        }
        Ok(v)
    }

    fn pop_i64(&mut self, pos: Pos, op: &str) -> R<i64> {
        let v = self.pop_num(pos, op)?;
        Ok(match v {
            Value::Int(i) => i.to_i64().unwrap_or(i64::MAX),
            Value::Float(f) => f as i64,
            _ => unreachable!(),
        })
    }

    fn pop_quot(&mut self, pos: Pos, op: &str) -> R<Rc<Vec<Instr>>> {
        let v = self.pop(pos, "a quotation")?;
        match v {
            Value::Quot(q) => Ok(q),
            _ => glitch(
                format!("{op} expects a [quotation], got {}", type_name(&v)),
                pos,
            ),
        }
    }

    /// A list, or a string exploded into 1-char strings.
    fn pop_seq(&mut self, pos: Pos, op: &str) -> R<Rc<Vec<Value>>> {
        let v = self.pop(pos, "a list or string")?;
        match v {
            Value::Str(s) => Ok(Rc::new(
                s.chars().map(|c| Value::str(c.to_string())).collect(),
            )),
            Value::List(l) => Ok(l),
            _ => glitch(
                format!("{op} expects a list or string, got {}", type_name(&v)),
                pos,
            ),
        }
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
fn both_ints<'a>(a: &'a Value, b: &'a Value) -> Option<(&'a BigInt, &'a BigInt)> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Some((x, y)),
        _ => None,
    }
}

fn arith(op: char, a: &Value, b: &Value, pos: Pos) -> R<Value> {
    if let Some((x, y)) = both_ints(a, b) {
        return Ok(match op {
            '+' => Value::Int(Rc::new(x + y)),
            '-' => Value::Int(Rc::new(x - y)),
            '×' => Value::Int(Rc::new(x * y)),
            '÷' => {
                if y.is_zero() {
                    return glitch("÷ by zero", pos);
                }
                if (x % y).is_zero() {
                    Value::Int(Rc::new(x / y))
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
                Value::Int(Rc::new(r))
            }
            '^' => {
                if y.is_negative() {
                    Value::Float(a.as_f64().unwrap().powf(b.as_f64().unwrap()))
                } else {
                    match y.to_u32() {
                        Some(e) => Value::Int(Rc::new(x.pow(e))),
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
    if let Some((x, y)) = both_ints(a, b) {
        return x.cmp(y);
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
}

fn coords(pos: Pos) -> String {
    if pos == (0, 0) {
        "?".into()
    } else {
        format!("{}:{}", pos.0, pos.1)
    }
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
        }
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
            };
            let _ = writeln!(
                self.err,
                "  strand {} ({}) waiting on {} at {}",
                fmt_i64(s.sid),
                s.label,
                what,
                coords(pos)
            );
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
        };
        if free {
            self.strands[i].status = Status::Run;
            self.strands[i].block = None;
        }
    }

    fn run_slice(&mut self, idx: usize) -> usize {
        let mut s = std::mem::replace(&mut self.strands[idx], Strand::placeholder());
        let mut executed = 0;
        while executed < SLICE {
            if s.frames.is_empty() {
                s.status = Status::Done;
                break;
            }
            match step(self, &mut s) {
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
                    if !s.catch(v.clone()) {
                        s.status = Status::Dead;
                        s.glitch = Some((v, pos));
                        break;
                    }
                }
            }
        }
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
                    self.report_deadlock(&blocked);
                    return;
                }
            }
        }
    }

    pub fn run_compiled(&mut self, prog: &CompiledProgram) -> i32 {
        self.main_count = prog.strands.len();
        self.next_spawn_sid = prog.strands.len() as i64;

        // The boot strand always runs: the standard library first, then the
        // program's own boot section (both already woven in at compile time).
        let boot = Strand::new(
            -1,
            "boot".into(),
            Rc::new(prog.boot.clone()),
            HashMap::new(),
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
                Rc::new(code.clone()),
                HashMap::new(),
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
    let mut boot = std_code();
    if let Some(cells) = &prog.boot_cells {
        boot.extend(lex_strand(cells.clone(), prog.axis)?);
    }
    Ok(CompiledProgram { boot, strands })
}

/// Compile MLang source text (rain or flat form).
pub fn compile_text(text: &str) -> Result<CompiledProgram, LoadError> {
    compile(&crate::forms::parse_source(text)?)
}

pub const STD_SOURCE: &str = include_str!("../../std/std.ml");

/// The standard library, lexed. Infallible: std.ml is verified by CI.
fn std_code() -> Vec<Instr> {
    let prog = crate::forms::parse_source(STD_SOURCE).expect("std.ml parses");
    let mut code = Vec::new();
    for (_, cells) in prog.strands {
        code.extend(lex_strand(cells, prog.axis).expect("std.ml lexes"));
    }
    code
}

// ── frame stepping ─────────────────────────────────────────────────────
fn step(vm: &mut VM, s: &mut Strand) -> R<()> {
    let fi = s.frames.len() - 1;
    match &s.frames[fi] {
        Frame::CF { .. } => {
            let (code, ip) = match &s.frames[fi] {
                Frame::CF { code, ip } => (code.clone(), *ip),
                _ => unreachable!(),
            };
            if ip >= code.len() {
                s.frames.pop();
                return Ok(());
            }
            let instr = code[ip].clone();
            match execute(vm, s, &instr) {
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
                        s.push(Value::List(Rc::new(out)));
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
            let q = vm.channels.entry(chan).or_default();
            let Some(v) = q.pop_front() else {
                return Err(Sig::Block(BlockOn::Chan(chan), pos));
            };
            if matches!(v, Value::Nil) {
                let frame = s.frames.pop().unwrap();
                if let Frame::Drain { out, .. } = frame {
                    s.push(Value::List(Rc::new(out)));
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
                let q = vm.channels.entry(src).or_default();
                let Some(v) = q.pop_front() else {
                    return Err(Sig::Block(BlockOn::Chan(src), pos));
                };
                if matches!(v, Value::Nil) {
                    vm.channels.entry(dst).or_default().push_back(Value::Nil);
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
                vm.channels.entry(dst).or_default().push_back(v);
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
            let v = if let Some(v) = s.locals.get(c) {
                v.clone()
            } else if let Some(v) = vm.globals.get(c) {
                v.clone()
            } else {
                return glitch(format!("undefined sigil '{c}'"), pos);
            };
            if let Value::Quot(q) = v {
                s.frames.push(cf(q));
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
            s.push(Value::List(Rc::new(items)));
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
                Value::Int(i) => s.push(Value::Int(i)),
                Value::Float(f) => {
                    let r = if ch == '⌊' { f.floor() } else { f.ceil() };
                    let big = BigInt::from_f64(r).unwrap_or_default();
                    s.push(Value::Int(Rc::new(big)));
                }
                _ => unreachable!(),
            }
        }
        '±' => {
            let v = s.pop_num(pos, "±")?;
            match v {
                Value::Int(i) => s.push(Value::Int(Rc::new(-&*i))),
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
            s.push(Value::List(Rc::new(items)));
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
                    s.push(Value::List(Rc::new(v)));
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
                    s.push(Value::List(Rc::new(l[i..j].to_vec())));
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
                    s.push(Value::List(Rc::new(parts)));
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
                t.parse::<BigInt>().ok().map(|b| Value::Int(Rc::new(b)))
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
                    s.push(Value::List(Rc::new(l.iter().rev().cloned().collect())))
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
                        s.push(Value::List(Rc::new(v2)));
                    } else if strs {
                        let mut v2 = l.as_ref().clone();
                        v2.sort_by(|a, b| match (a, b) {
                            (Value::Str(x), Value::Str(y)) => x.cmp(y),
                            _ => unreachable!(),
                        });
                        s.push(Value::List(Rc::new(v2)));
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
            if vm.globals.contains_key(&arg) {
                return glitch(format!("sigil '{arg}' is already defined"), pos);
            }
            let v = s.pop(pos, "a value to bind")?;
            vm.globals.insert(arg, v);
        }
        '⇒' => {
            let v = s.pop(pos, "a value to store")?;
            s.locals.insert(arg, v);
        }
        // ── strands & channels ──
        '↥' => {
            let v = s.pop(pos, "a value to send")?;
            vm.channels.entry(arg).or_default().push_back(v);
        }
        '↧' => {
            let ch_q = vm.channels.entry(arg).or_default();
            match ch_q.pop_front() {
                Some(v) => s.push(v),
                None => return Err(Sig::Block(BlockOn::Chan(arg), pos)),
            }
        }
        '⇂' => {
            let ch_q = vm.channels.entry(arg).or_default();
            match ch_q.pop_front() {
                Some(v) => {
                    s.push(v);
                    s.push(Value::int(1));
                }
                None => s.push(Value::int(0)),
            }
        }
        '⇈' => {
            let items = s.pop_seq(pos, "⇈")?;
            let q = vm.channels.entry(arg).or_default();
            q.extend(items.iter().cloned());
            q.push_back(Value::Nil);
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
            let sid = vm.next_spawn_sid;
            vm.next_spawn_sid += 1;
            let child = Strand::new(
                sid,
                format!("⚡ of strand {}", fmt_i64(s.sid)),
                q,
                s.locals.clone(),
            );
            vm.register(child);
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
            let Some(&t) = vm.by_sid.get(&sid) else {
                return glitch(format!("⋈ no strand with id {}", fmt_i64(sid)), pos);
            };
            if !matches!(vm.strands[t].status, Status::Done | Status::Dead) {
                return Err(Sig::Block(BlockOn::Strand(sid), pos));
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
        // ── i/o ──
        '⍞' => {
            let v = s.pop_any(pos)?;
            let _ = writeln!(vm.out, "{}", fmt(&v, false));
        }
        '⊸' => {
            let v = s.pop_any(pos)?;
            let _ = write!(vm.out, "{}", fmt(&v, false));
        }
        '⌨' => {
            let mut line = String::new();
            match vm.stdin.read_line(&mut line) {
                Ok(0) => s.push(Value::Nil),
                Ok(_) => {
                    if line.ends_with('\n') {
                        line.pop();
                        if line.ends_with('\r') {
                            line.pop();
                        }
                    }
                    s.push(Value::str(line));
                }
                Err(_) => s.push(Value::Nil),
            }
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

use num_traits::FromPrimitive;
