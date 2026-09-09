//! The MLang executable payload format.
//!
//! `mlang build` welds a compiled program into a native binary the same way
//! Go links its runtime into every executable: the output file is a copy of
//! the toolchain's own runtime image with the serialized program appended,
//! followed by a fixed footer:
//!
//! ```text
//! [runtime image][payload bytes][payload_len: u64 LE][b"MLANGBIN"]
//! ```
//!
//! At startup the runtime checks its own file for the footer; if present it
//! runs the embedded program instead of behaving as a compiler. The payload
//! normally rides inside the exact runtime it was built with, but nothing
//! stops someone splicing a payload onto a different runtime (or the file
//! being damaged in transit), so the `FORMAT_VERSION` check at the head of
//! the payload is the defence against version mismatches, and the reader
//! below treats every byte as untrusted: any malformed input yields `Err`,
//! never a panic or an unbounded allocation.
//!
//! (The layout line above is a diagram, not code — kept out of doctests.)

use crate::values::{Instr, Op, Value};
use crate::vm::CompiledProgram;
use num_bigint::BigInt;
use std::sync::Arc;

pub const MAGIC: &[u8; 8] = b"MLANGBIN";
const FORMAT_VERSION: u32 = 2;

// ── writer ─────────────────────────────────────────────────────────────
struct W(Vec<u8>);

impl W {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn bytes(&mut self, b: &[u8]) {
        self.u64(b.len() as u64);
        self.0.extend_from_slice(b);
    }
    fn string(&mut self, s: &str) {
        self.bytes(s.as_bytes());
    }
    fn ch(&mut self, c: char) {
        self.u32(c as u32);
    }

    fn value(&mut self, v: &Value) {
        match v {
            Value::Nil => self.u8(0),
            // Int and Big share the language-level int type and one wire tag.
            Value::Int(i) => {
                self.u8(1);
                self.bytes(&BigInt::from(*i).to_signed_bytes_le());
            }
            Value::Big(b) => {
                self.u8(1);
                self.bytes(&b.to_signed_bytes_le());
            }
            Value::Float(f) => {
                self.u8(2);
                self.u64(f.to_bits());
            }
            Value::Str(s) => {
                self.u8(3);
                self.string(s);
            }
            Value::List(l) => {
                self.u8(4);
                self.u64(l.len() as u64);
                for x in l.iter() {
                    self.value(x);
                }
            }
            Value::Quot(q) => {
                self.u8(5);
                self.code(q);
            }
            Value::Mark => unreachable!("marks never appear in compiled code"),
        }
    }

    fn instr(&mut self, i: &Instr) {
        self.u32(i.pos.0);
        self.u32(i.pos.1);
        match &i.op {
            Op::Push(v) => {
                self.u8(0);
                self.value(v);
            }
            Op::Name(c) => {
                self.u8(1);
                self.ch(*c);
            }
            Op::LMark => self.u8(2),
            Op::LBuild => self.u8(3),
            Op::B(op, a, b) => {
                self.u8(4);
                self.ch(*op);
                self.ch(*a);
                self.ch(*b);
            }
        }
    }

    fn code(&mut self, code: &[Instr]) {
        self.u64(code.len() as u64);
        for i in code {
            self.instr(i);
        }
    }
}

pub fn serialize(prog: &CompiledProgram) -> Vec<u8> {
    let mut w = W(Vec::new());
    w.u32(FORMAT_VERSION);
    w.code(&prog.boot);
    w.u64(prog.strands.len() as u64);
    for (label, code) in &prog.strands {
        w.string(label);
        w.code(code);
    }
    // v2: the program's source lines ride along, so a welded binary's
    // glitch reports can excerpt the offending line.
    w.u64(prog.source.len() as u64);
    for line in &prog.source {
        w.string(line);
    }
    w.0
}

// ── reader ─────────────────────────────────────────────────────────────
//
// Everything in the reader is total: the payload may be truncated, bit
// flipped or hand-crafted, and the only acceptable outcome is `Err`. In
// particular no length read from the payload is ever handed to an allocator
// unchecked — a corrupted `u64` count would otherwise abort the process with
// a capacity overflow long before main() could print "corrupt program
// payload" and exit 2.
struct R<'a> {
    buf: &'a [u8],
    i: usize,
    /// Current nesting of quotations/lists being decoded. The reader is
    /// recursive, so a hand-crafted payload nesting `⟨⟨⟨…` tens of thousands
    /// deep would overflow the stack — an abort, not an `Err`. Real programs
    /// nest as deep as their source does, which is nowhere near the cap.
    depth: u32,
}

const MAX_DEPTH: u32 = 1024;

type PResult<T> = Result<T, String>;

/// A `Vec` capacity hint that can't be weaponised by a bogus count. Every
/// item of every list encodes to at least one byte, so a claimed count of
/// `n` items is only plausible if at least `n` bytes remain; the hint is
/// capped there so the allocation stays proportional to real input.
fn bounded_capacity(claimed: usize, remaining: usize) -> usize {
    claimed.min(remaining)
}

impl<'a> R<'a> {
    fn remaining(&self) -> usize {
        self.buf.len() - self.i
    }

    fn take(&mut self, n: usize) -> PResult<&'a [u8]> {
        // `self.i + n` could wrap for a hostile `n`, which would let the
        // bounds check pass and the slice below panic — hence checked math.
        let end = self.i.checked_add(n).ok_or("truncated payload")?;
        if end > self.buf.len() {
            return Err("truncated payload".into());
        }
        let s = &self.buf[self.i..end];
        self.i = end;
        Ok(s)
    }
    fn u8(&mut self) -> PResult<u8> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> PResult<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn u64(&mut self) -> PResult<u64> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
    }
    /// A length prefix (count of bytes or items). Validated against the
    /// remaining input before anyone allocates on the strength of it: a
    /// count that can't possibly fit is reported as truncation right away.
    /// The u64 → usize narrowing (a concern on 32-bit targets) folds into
    /// the same error instead of silently wrapping.
    fn len(&mut self) -> PResult<usize> {
        let n = usize::try_from(self.u64()?).map_err(|_| "truncated payload")?;
        if n > self.remaining() {
            return Err("truncated payload".into());
        }
        Ok(n)
    }
    fn bytes(&mut self) -> PResult<&'a [u8]> {
        let n = self.len()?;
        self.take(n)
    }
    fn string(&mut self) -> PResult<String> {
        String::from_utf8(self.bytes()?.to_vec()).map_err(|_| "bad utf-8".into())
    }
    fn ch(&mut self) -> PResult<char> {
        char::from_u32(self.u32()?).ok_or_else(|| "bad char".into())
    }
    fn enter(&mut self) -> PResult<()> {
        if self.depth >= MAX_DEPTH {
            return Err("program nested too deeply".into());
        }
        self.depth += 1;
        Ok(())
    }
    fn leave(&mut self) {
        self.depth -= 1;
    }

    fn value(&mut self) -> PResult<Value> {
        self.enter()?;
        let v = self.value_inner();
        self.leave();
        v
    }

    fn value_inner(&mut self) -> PResult<Value> {
        Ok(match self.u8()? {
            0 => Value::Nil,
            1 => Value::from_big(BigInt::from_signed_bytes_le(self.bytes()?)),
            2 => Value::Float(f64::from_bits(self.u64()?)),
            3 => Value::Str(Arc::new(self.string()?)),
            4 => {
                let n = self.len()?;
                let mut items = Vec::with_capacity(bounded_capacity(n, self.remaining()));
                for _ in 0..n {
                    items.push(self.value()?);
                }
                Value::List(Arc::new(items))
            }
            5 => Value::Quot(Arc::new(self.code()?)),
            t => return Err(format!("bad value tag {t}")),
        })
    }

    fn instr(&mut self) -> PResult<Instr> {
        let pos = (self.u32()?, self.u32()?);
        let op = match self.u8()? {
            0 => Op::Push(self.value()?),
            1 => Op::Name(self.ch()?),
            2 => Op::LMark,
            3 => Op::LBuild,
            4 => Op::B(self.ch()?, self.ch()?, self.ch()?),
            t => return Err(format!("bad op tag {t}")),
        };
        Ok(Instr { op, pos })
    }

    fn code(&mut self) -> PResult<Vec<Instr>> {
        self.enter()?;
        let c = self.code_inner();
        self.leave();
        c
    }

    fn code_inner(&mut self) -> PResult<Vec<Instr>> {
        let n = self.len()?;
        let mut code = Vec::with_capacity(bounded_capacity(n, self.remaining()));
        for _ in 0..n {
            code.push(self.instr()?);
        }
        Ok(code)
    }
}

pub fn deserialize(buf: &[u8]) -> PResult<CompiledProgram> {
    let mut r = R { buf, i: 0, depth: 0 };
    let version = r.u32()?;
    if version != FORMAT_VERSION {
        return Err(format!("payload format v{version}, runtime speaks v{FORMAT_VERSION}"));
    }
    let boot = r.code()?;
    let n = r.len()?;
    let mut strands = Vec::with_capacity(bounded_capacity(n, r.remaining()));
    for _ in 0..n {
        let label = r.string()?;
        let code = r.code()?;
        strands.push((label, code));
    }
    let n = r.len()?;
    let mut source = Vec::with_capacity(bounded_capacity(n, r.remaining()));
    for _ in 0..n {
        source.push(r.string()?);
    }
    // Bytes left over after a complete program mean the length footer and
    // the program disagree — that is corruption too, not something to run.
    if r.remaining() != 0 {
        return Err("trailing bytes after program".into());
    }
    Ok(CompiledProgram { boot, strands, source })
}

// ── native binary embedding ────────────────────────────────────────────
/// Extract a payload from an executable image, if present.
pub fn extract(image: &[u8]) -> Option<PResult<CompiledProgram>> {
    if image.len() < 16 || &image[image.len() - 8..] != MAGIC {
        return None;
    }
    let body = &image[..image.len() - 16];
    let mut len_bytes = [0u8; 8];
    len_bytes.copy_from_slice(&image[image.len() - 16..image.len() - 8]);
    let plen = u64::from_le_bytes(len_bytes);
    // The footer's length is as untrusted as the payload: a value larger
    // than the file (or one that doesn't fit a usize) is a corrupt footer,
    // not a reason to index out of bounds.
    let start = match usize::try_from(plen).ok().and_then(|n| body.len().checked_sub(n)) {
        Some(start) => start,
        None => return Some(Err("corrupt payload footer".into())),
    };
    Some(deserialize(&body[start..]))
}

/// The payload embedded in the currently running executable, if any.
pub fn self_payload() -> Option<PResult<CompiledProgram>> {
    let exe = std::env::current_exe().ok()?;
    let image = std::fs::read(exe).ok()?;
    extract(&image)
}

/// Weld a compiled program onto a runtime image, producing a standalone
/// native executable image.
pub fn weld(runtime_image: &[u8], prog: &CompiledProgram) -> Vec<u8> {
    let payload = serialize(prog);
    let mut out = Vec::with_capacity(runtime_image.len() + payload.len() + 16);
    out.extend_from_slice(runtime_image);
    out.extend_from_slice(&payload);
    out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    out.extend_from_slice(MAGIC);
    out
}
