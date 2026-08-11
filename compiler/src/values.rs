//! MLang runtime values — immutable, cheaply clonable via Rc.

use num_bigint::BigInt;
use num_traits::ToPrimitive;
use std::rc::Rc;

/// 1-based (row, col) in the original source file; (0, 0) means unknown.
pub type Pos = (u32, u32);

#[derive(Clone, Debug)]
pub struct Instr {
    pub op: Op,
    pub pos: Pos,
}

#[derive(Clone, Debug)]
pub enum Op {
    Push(Value),
    Name(char),
    LMark,
    LBuild,
    /// Builtin op char plus up to two argument sigils ('\0' when absent):
    /// one for ≔ ⇒ ↥ ↧ ⇂ ⇈ ⇟, two (src, dst) for ⇉.
    B(char, char, char),
}

/// Integers keep a canonical form: `Int` whenever the value fits in i64,
/// `Big` only beyond that. Both are the single language-level "int" type;
/// the split is invisible to programs (⍙, =, ⍕ agree across it).
#[derive(Clone, Debug)]
pub enum Value {
    Int(i64),
    Big(Rc<BigInt>),
    Float(f64),
    Str(Rc<String>),
    List(Rc<Vec<Value>>),
    Quot(Rc<Vec<Instr>>),
    Nil,
    Mark,
}

impl Value {
    pub fn int(i: i64) -> Value {
        Value::Int(i)
    }
    pub fn from_big(b: BigInt) -> Value {
        match b.to_i64() {
            Some(i) => Value::Int(i),
            None => Value::Big(Rc::new(b)),
        }
    }
    pub fn str(s: impl Into<String>) -> Value {
        Value::Str(Rc::new(s.into()))
    }
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(i) => Some(*i as f64),
            Value::Big(b) => Some(b.to_f64().unwrap_or(f64::INFINITY)),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }
    pub fn is_num(&self) -> bool {
        matches!(self, Value::Int(_) | Value::Big(_) | Value::Float(_))
    }
}

pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Nil => false,
        Value::Int(i) => *i != 0,
        Value::Big(_) => true, // canonical form: zero always fits i64
        Value::Float(f) => *f != 0.0,
        Value::Str(s) => !s.is_empty(),
        Value::List(l) => !l.is_empty(),
        Value::Quot(_) | Value::Mark => true,
    }
}

/// Deep equality: numbers compare numerically across int/float, lists deeply,
/// quotations by identity (as in the reference implementation).
pub fn val_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Nil, Value::Nil) => true,
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Big(x), Value::Big(y)) => x == y,
        // canonical form makes a small Big impossible, so Int ≠ Big always
        (Value::Int(_), Value::Big(_)) | (Value::Big(_), Value::Int(_)) => false,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Int(_) | Value::Big(_), Value::Float(y)) => a.as_f64() == Some(*y),
        (Value::Float(x), Value::Int(_) | Value::Big(_)) => Some(*x) == b.as_f64(),
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::List(x), Value::List(y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(p, q)| val_eq(p, q))
        }
        (Value::Quot(x), Value::Quot(y)) => Rc::ptr_eq(x, y),
        _ => false,
    }
}

pub fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Nil => "∅",
        Value::Int(_) | Value::Big(_) => "int",
        Value::Float(_) => "float",
        Value::Str(_) => "str",
        Value::List(_) => "list",
        Value::Quot(_) => "quot",
        Value::Mark => "mark",
    }
}

pub fn fmt_i64(n: i64) -> String {
    n.to_string().replace('-', "¯")
}

fn fmt_f64(x: f64) -> String {
    if x.is_nan() {
        return "nan".into();
    }
    if x.is_infinite() {
        return if x < 0.0 { "¯inf".into() } else { "inf".into() };
    }
    let s = format!("{x}");
    let s = if !s.contains('.') && !s.contains('e') {
        format!("{s}.0")
    } else {
        s
    };
    s.replace('-', "¯")
}

/// Render a value. `quote` renders strings as «...» (debug/list style).
pub fn fmt(v: &Value, quote: bool) -> String {
    match v {
        Value::Nil => "∅".into(),
        Value::Int(i) => fmt_i64(*i),
        Value::Big(b) => b.to_string().replace('-', "¯"),
        Value::Float(f) => fmt_f64(*f),
        Value::Str(s) => {
            if quote {
                format!("«{s}»")
            } else {
                s.to_string()
            }
        }
        Value::List(l) => {
            let inner: Vec<String> = l.iter().map(|x| fmt(x, true)).collect();
            format!("⟨{}⟩", inner.join(" "))
        }
        Value::Quot(_) => "[⋯]".into(),
        Value::Mark => "⟨".into(),
    }
}
