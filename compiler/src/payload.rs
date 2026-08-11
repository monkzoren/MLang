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
//! runs the embedded program instead of behaving as a compiler. Because the
//! payload rides inside the exact runtime it was built with, version
//! mismatches are impossible by construction.
//!
//! (The layout line above is a diagram, not code — kept out of doctests.)

use crate::values::{Instr, Op, Value};
use crate::vm::CompiledProgram;
use num_bigint::BigInt;
use std::rc::Rc;

pub const MAGIC: &[u8; 8] = b"MLANGBIN";
const FORMAT_VERSION: u32 = 1;

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
    w.0
}

// ── reader ─────────────────────────────────────────────────────────────
struct R<'a> {
    buf: &'a [u8],
    i: usize,
}

type PResult<T> = Result<T, String>;

impl<'a> R<'a> {
    fn take(&mut self, n: usize) -> PResult<&'a [u8]> {
        if self.i + n > self.buf.len() {
            return Err("truncated payload".into());
        }
        let s = &self.buf[self.i..self.i + n];
        self.i += n;
        Ok(s)
    }
    fn u8(&mut self) -> PResult<u8> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> PResult<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> PResult<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn bytes(&mut self) -> PResult<&'a [u8]> {
        let n = self.u64()? as usize;
        self.take(n)
    }
    fn string(&mut self) -> PResult<String> {
        String::from_utf8(self.bytes()?.to_vec()).map_err(|_| "bad utf-8".into())
    }
    fn ch(&mut self) -> PResult<char> {
        char::from_u32(self.u32()?).ok_or_else(|| "bad char".into())
    }

    fn value(&mut self) -> PResult<Value> {
        Ok(match self.u8()? {
            0 => Value::Nil,
            1 => Value::from_big(BigInt::from_signed_bytes_le(self.bytes()?)),
            2 => Value::Float(f64::from_bits(self.u64()?)),
            3 => Value::Str(Rc::new(self.string()?)),
            4 => {
                let n = self.u64()? as usize;
                let mut items = Vec::with_capacity(n);
                for _ in 0..n {
                    items.push(self.value()?);
                }
                Value::List(Rc::new(items))
            }
            5 => Value::Quot(Rc::new(self.code()?)),
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
        let n = self.u64()? as usize;
        let mut code = Vec::with_capacity(n);
        for _ in 0..n {
            code.push(self.instr()?);
        }
        Ok(code)
    }
}

pub fn deserialize(buf: &[u8]) -> PResult<CompiledProgram> {
    let mut r = R { buf, i: 0 };
    let version = r.u32()?;
    if version != FORMAT_VERSION {
        return Err(format!("payload format v{version}, runtime speaks v{FORMAT_VERSION}"));
    }
    let boot = r.code()?;
    let n = r.u64()? as usize;
    let mut strands = Vec::with_capacity(n);
    for _ in 0..n {
        let label = r.string()?;
        let code = r.code()?;
        strands.push((label, code));
    }
    Ok(CompiledProgram { boot, strands })
}

// ── native binary embedding ────────────────────────────────────────────
/// Extract a payload from an executable image, if present.
pub fn extract(image: &[u8]) -> Option<PResult<CompiledProgram>> {
    if image.len() < 16 || &image[image.len() - 8..] != MAGIC {
        return None;
    }
    let len_bytes: [u8; 8] = image[image.len() - 16..image.len() - 8].try_into().unwrap();
    let plen = u64::from_le_bytes(len_bytes);
    if plen.saturating_add(16) > image.len() as u64 {
        return Some(Err("corrupt payload footer".into()));
    }
    let start = image.len() - 16 - plen as usize;
    Some(deserialize(&image[start..image.len() - 16]))
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
