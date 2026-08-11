//! The wire codec — MLang values as single lines of UTF-8 text.
//!
//! `mlang hub` and `mlang worker` bridge channels over TCP with a
//! line-per-value protocol. The rendering is the language's own value
//! syntax (`∅`, `¯5`, `2.5`, `«text»`, `⟨1 «a» ⟨2⟩⟩`), so the stream is
//! readable with netcat and any value round-trips exactly. Newlines
//! inside strings travel as `⏎`, the same convention source literals
//! use — and with the same limitation: a string cannot contain a
//! literal `⏎` or `»` glyph.
//!
//! Quotations do not cross the wire: they are code, and their equality
//! is identity (values.rs), which serialization cannot preserve.

use crate::values::{fmt_i64, Value};
use num_bigint::BigInt;
use std::sync::Arc;

/// Render a value as one line (no trailing newline). Errors name the
/// unsendable type: quotations and list marks stay in-process.
pub fn render(v: &Value) -> Result<String, String> {
    let mut s = String::new();
    write_value(v, &mut s)?;
    Ok(s)
}

fn write_value(v: &Value, out: &mut String) -> Result<(), String> {
    match v {
        Value::Nil => out.push('∅'),
        Value::Int(i) => out.push_str(&fmt_i64(*i)),
        Value::Big(b) => out.push_str(&b.to_string().replace('-', "¯")),
        Value::Float(f) => out.push_str(&render_float(*f)),
        Value::Str(s) => {
            out.push('«');
            for c in s.chars() {
                out.push(if c == '\n' { '⏎' } else { c });
            }
            out.push('»');
        }
        Value::List(items) => {
            out.push('⟨');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                write_value(item, out)?;
            }
            out.push('⟩');
        }
        Value::Quot(_) => return Err("cannot send a quotation over the network".into()),
        Value::Mark => return Err("cannot send an unfinished list over the network".into()),
    }
    Ok(())
}

/// Floats render like ⍕ does, except always with a distinguishing mark
/// (`.`, `e`, `inf`, `nan`) so the parser can tell them from ints.
fn render_float(x: f64) -> String {
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

/// Parse one rendered line back into a value.
pub fn parse(line: &str) -> Result<Value, String> {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    skip_spaces(&chars, &mut i);
    let v = parse_value(&chars, &mut i)?;
    skip_spaces(&chars, &mut i);
    if i != chars.len() {
        return Err(format!("trailing content after value: {line}"));
    }
    Ok(v)
}

fn skip_spaces(chars: &[char], i: &mut usize) {
    while chars.get(*i) == Some(&' ') {
        *i += 1;
    }
}

fn parse_value(chars: &[char], i: &mut usize) -> Result<Value, String> {
    match chars.get(*i) {
        None => Err("empty value".into()),
        Some('∅') => {
            *i += 1;
            Ok(Value::Nil)
        }
        Some('«') => {
            *i += 1;
            let mut s = String::new();
            loop {
                match chars.get(*i) {
                    None => return Err("unclosed « string".into()),
                    Some('»') => {
                        *i += 1;
                        return Ok(Value::str(s));
                    }
                    Some('⏎') => {
                        s.push('\n');
                        *i += 1;
                    }
                    Some(c) => {
                        s.push(*c);
                        *i += 1;
                    }
                }
            }
        }
        Some('⟨') => {
            *i += 1;
            let mut items = Vec::new();
            loop {
                skip_spaces(chars, i);
                match chars.get(*i) {
                    None => return Err("unclosed ⟨ list".into()),
                    Some('⟩') => {
                        *i += 1;
                        return Ok(Value::List(Arc::new(items)));
                    }
                    Some(_) => items.push(parse_value(chars, i)?),
                }
            }
        }
        Some(c) if *c == '¯' || c.is_ascii_digit() || *c == '.' || *c == 'n' || *c == 'i' => {
            parse_number(chars, i)
        }
        Some(c) => Err(format!("unexpected glyph {c}")),
    }
}

fn parse_number(chars: &[char], i: &mut usize) -> Result<Value, String> {
    let start = *i;
    while let Some(c) = chars.get(*i) {
        if c.is_ascii_digit()
            || matches!(c, '¯' | '.' | 'e' | 'i' | 'n' | 'f' | 'a' | '+' | '-')
        {
            *i += 1;
        } else {
            break;
        }
    }
    let raw: String = chars[start..*i].iter().collect();
    let ascii = raw.replace('¯', "-");
    let is_float = ascii.contains('.')
        || ascii.contains('e')
        || ascii.contains("inf")
        || ascii.contains("nan");
    if is_float {
        ascii
            .parse::<f64>()
            .map(Value::Float)
            .map_err(|_| format!("malformed number {raw}"))
    } else {
        ascii
            .parse::<BigInt>()
            .map(Value::from_big)
            .map_err(|_| format!("malformed number {raw}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::val_eq;

    fn round_trip(v: Value) {
        let line = render(&v).unwrap();
        assert!(!line.contains('\n'), "rendering must be one line: {line}");
        let back = parse(&line).unwrap();
        assert!(val_eq(&v, &back), "round trip changed {line}");
    }

    #[test]
    fn scalars_round_trip() {
        round_trip(Value::Nil);
        round_trip(Value::int(0));
        round_trip(Value::int(-42));
        round_trip(Value::int(i64::MAX));
        round_trip(Value::from_big("123456789012345678901234567890".parse().unwrap()));
        round_trip(Value::from_big(
            "-123456789012345678901234567890".parse().unwrap(),
        ));
        round_trip(Value::Float(2.5));
        round_trip(Value::Float(-0.125));
        round_trip(Value::Float(1e300));
        round_trip(Value::Float(f64::INFINITY));
        round_trip(Value::Float(f64::NEG_INFINITY));
        round_trip(Value::Float(3.0));
    }

    #[test]
    fn nan_round_trips_as_nan() {
        let back = parse(&render(&Value::Float(f64::NAN)).unwrap()).unwrap();
        match back {
            Value::Float(f) => assert!(f.is_nan()),
            _ => panic!("nan came back as a non-float"),
        }
    }

    #[test]
    fn strings_round_trip() {
        round_trip(Value::str(""));
        round_trip(Value::str("hello world"));
        round_trip(Value::str("line one\nline two\n"));
        round_trip(Value::str("glyphs: ⟨⟩ ∂ × ¯5 ∅"));
    }

    #[test]
    fn lists_round_trip() {
        round_trip(Value::List(Arc::new(vec![])));
        round_trip(Value::List(Arc::new(vec![
            Value::int(1),
            Value::str("a b"),
            Value::List(Arc::new(vec![Value::int(2), Value::Nil])),
            Value::Float(0.5),
        ])));
    }

    #[test]
    fn quotations_are_refused() {
        let q = Value::Quot(Arc::new(vec![]));
        assert!(render(&q).is_err());
        assert!(render(&Value::List(Arc::new(vec![Value::int(1), q]))).is_err());
    }

    #[test]
    fn malformed_lines_are_refused() {
        for bad in ["", "⟨1 2", "«open", "1 2", "abc", "¯¯3", "1.2.3"] {
            assert!(parse(bad).is_err(), "parsed: {bad}");
        }
    }
}
