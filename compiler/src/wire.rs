//! The wire codec — MLang values as single lines of UTF-8 text.
//!
//! `mlang hub` and `mlang worker` bridge channels over TCP with a
//! line-per-value protocol. The rendering is the language's own value
//! syntax (`∅`, `¯5`, `2.5`, `«text»`, `⟨1 «a» ⟨2⟩⟩`), so the stream is
//! readable with netcat and any value round-trips exactly. Newlines
//! inside strings travel as `⏎`, the same convention source literals
//! use — and with the same limitation: a string cannot contain a
//! literal `⏎` or `»` glyph. Rather than emit a line that would parse
//! back as a *different* string, `render` refuses such a value, so
//! everything that renders round-trips exactly (`render_is_inverse`).
//!
//! Quotations do not cross the wire: they are code, and their equality
//! is identity (values.rs), which serialization cannot preserve.
//!
//! Two limits protect the peer: a nesting-depth cap (a line of 100 000
//! `⟨` must not overflow anyone's stack — the codec is iterative, and
//! the cap keeps the resulting value within what the rest of the
//! runtime handles recursively) and, in `read_line`, a byte cap on
//! incoming lines — a hostile or confused peer cannot make the process
//! allocate without bound.

use crate::values::{fmt_i64, Value};
use num_bigint::BigInt;
use std::io::{BufRead, ErrorKind};
use std::sync::Arc;

/// Deepest list nesting accepted in either direction. The codec itself
/// is iterative, but a value this deep is then dropped, compared and
/// printed by values.rs, which walks lists recursively: a 2 MiB thread
/// (the default for the hub's reader threads and every strand) survives
/// a drop about 7 000 deep, so 1 000 leaves an order of magnitude of
/// headroom and is still far beyond any real program's data.
pub const MAX_DEPTH: usize = 1_000;

/// Longest incoming line, in bytes, before the peer is declared broken.
pub const MAX_LINE: usize = 64 * 1024 * 1024;

/// Render a value as one line (no trailing newline). Errors name the
/// unsendable type: quotations and list marks stay in-process, as do
/// strings containing the two glyphs the line syntax reserves.
///
/// Iterative, with an explicit stack of open lists: a value nested
/// MAX_DEPTH deep must not exhaust a thread's stack, and the depth cap
/// then bounds memory rather than recursion.
pub fn render(v: &Value) -> Result<String, String> {
    let mut out = String::new();
    // Each open list and the index of the next item to render.
    let mut open: Vec<(&Vec<Value>, usize)> = Vec::new();
    let mut cur = v;
    loop {
        match cur {
            Value::Nil => out.push('∅'),
            Value::Int(i) => out.push_str(&fmt_i64(*i)),
            Value::Big(b) => out.push_str(&b.to_string().replace('-', "¯")),
            Value::Float(f) => out.push_str(&render_float(*f)),
            Value::Str(s) => {
                // `»` closes the literal and `⏎` decodes as a newline: a
                // string holding either would come back changed.
                if s.contains('»') || s.contains('⏎') {
                    return Err("cannot send a string containing » or ⏎ over the network".into());
                }
                out.push('«');
                for c in s.chars() {
                    out.push(if c == '\n' { '⏎' } else { c });
                }
                out.push('»');
            }
            Value::List(items) => {
                if open.len() >= MAX_DEPTH {
                    return Err(format!("cannot send a list nested deeper than {MAX_DEPTH}"));
                }
                out.push('⟨');
                open.push((items, 0));
            }
            Value::Quot(_) => return Err("cannot send a quotation over the network".into()),
            Value::Mark => return Err("cannot send an unfinished list over the network".into()),
        }
        // Advance to the next item, closing lists that are exhausted.
        loop {
            let Some((items, idx)) = open.last_mut() else { return Ok(out) };
            if *idx < items.len() {
                if *idx > 0 {
                    out.push(' ');
                }
                cur = &items[*idx];
                *idx += 1;
                break;
            }
            out.push('⟩');
            open.pop();
        }
    }
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

/// Parse one rendered line back into a value. Iterative for the same
/// reason `render` is: a line of 100 000 `⟨` is refused at MAX_DEPTH,
/// never allowed near the stack.
pub fn parse(line: &str) -> Result<Value, String> {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    // The lists opened and not yet closed, innermost last.
    let mut open: Vec<Vec<Value>> = Vec::new();
    loop {
        skip_spaces(&chars, &mut i);
        let v = match chars.get(i) {
            None if open.is_empty() => return Err("empty value".into()),
            None => return Err("unclosed ⟨ list".into()),
            Some('⟨') => {
                if open.len() >= MAX_DEPTH {
                    return Err(format!("list nested deeper than {MAX_DEPTH}"));
                }
                i += 1;
                open.push(Vec::new());
                continue;
            }
            Some('⟩') => {
                i += 1;
                match open.pop() {
                    Some(items) => Value::List(Arc::new(items)),
                    None => return Err("unexpected ⟩".into()),
                }
            }
            Some(_) => parse_atom(&chars, &mut i)?,
        };
        match open.last_mut() {
            Some(items) => items.push(v),
            None => {
                skip_spaces(&chars, &mut i);
                if i != chars.len() {
                    return Err(format!("trailing content after value: {line}"));
                }
                return Ok(v);
            }
        }
    }
}

fn skip_spaces(chars: &[char], i: &mut usize) {
    while chars.get(*i) == Some(&' ') {
        *i += 1;
    }
}

/// One non-list value at position `i`.
fn parse_atom(chars: &[char], i: &mut usize) -> Result<Value, String> {
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

// ── framing ────────────────────────────────────────────────────────────

/// Why `read_line` produced no line.
#[derive(Debug)]
pub enum LineEnd {
    /// Clean end of stream: the last byte read was a `\n`.
    Eof,
    /// The peer stopped mid-line — a process dying with a value half
    /// written. Whatever arrived must not be taken for a value.
    Truncated,
    /// The line exceeded the byte cap without a `\n`.
    TooLong,
    /// The bytes were not UTF-8.
    NotUtf8,
    /// The socket failed — including a read timeout (`WouldBlock` /
    /// `TimedOut`), which the callers treat as a silent peer.
    Io(std::io::Error),
}

impl PartialEq for LineEnd {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (LineEnd::Eof, LineEnd::Eof)
            | (LineEnd::Truncated, LineEnd::Truncated)
            | (LineEnd::TooLong, LineEnd::TooLong)
            | (LineEnd::NotUtf8, LineEnd::NotUtf8) => true,
            (LineEnd::Io(a), LineEnd::Io(b)) => a.kind() == b.kind(),
            _ => false,
        }
    }
}

impl std::fmt::Display for LineEnd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LineEnd::Eof => write!(f, "end of stream"),
            LineEnd::Truncated => write!(f, "connection closed mid-line"),
            LineEnd::TooLong => write!(f, "line longer than {MAX_LINE} bytes"),
            LineEnd::NotUtf8 => write!(f, "line is not UTF-8"),
            LineEnd::Io(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                write!(f, "peer silent for too long")
            }
            LineEnd::Io(e) => write!(f, "{e}"),
        }
    }
}

/// Read one `\n`-terminated line (terminator removed). Unlike
/// `BufRead::lines`, a final line *without* its terminator is an error,
/// not a line: on this wire a value is only complete once its newline
/// has arrived, so a truncated line means the peer died mid-write.
/// A line longer than `max` bytes is refused before it is stored.
pub fn read_line<R: BufRead>(r: &mut R, max: usize) -> Result<String, LineEnd> {
    let mut buf: Vec<u8> = Vec::new();
    loop {
        let (done, used) = {
            let avail = match r.fill_buf() {
                Ok(a) => a,
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(LineEnd::Io(e)),
            };
            if avail.is_empty() {
                return Err(if buf.is_empty() { LineEnd::Eof } else { LineEnd::Truncated });
            }
            match avail.iter().position(|&b| b == b'\n') {
                Some(i) => {
                    if buf.len() + i > max {
                        return Err(LineEnd::TooLong);
                    }
                    buf.extend_from_slice(&avail[..i]);
                    (true, i + 1)
                }
                None => {
                    if buf.len() + avail.len() > max {
                        return Err(LineEnd::TooLong);
                    }
                    buf.extend_from_slice(avail);
                    (false, avail.len())
                }
            }
        };
        r.consume(used);
        if done {
            return String::from_utf8(buf).map_err(|_| LineEnd::NotUtf8);
        }
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
        for bad in ["", "⟨1 2", "«open", "1 2", "abc", "¯¯3", "1.2.3", "⟩", "1⟩", "⟨⟩⟩"] {
            assert!(parse(bad).is_err(), "parsed: {bad}");
        }
    }

    #[test]
    fn reserved_glyphs_in_strings_are_refused() {
        // `»` would close the literal early; `⏎` would decode as a
        // newline. Neither can round-trip, so neither may be sent —
        // even nested inside a list.
        assert!(render(&Value::str("a»b")).is_err());
        assert!(render(&Value::str("a⏎b")).is_err());
        assert!(render(&Value::List(Arc::new(vec![Value::int(1), Value::str("»")]))).is_err());
        // `«` inside a string is fine: only `»` terminates.
        round_trip(Value::str("say «hi"));
    }

    #[test]
    fn render_is_inverse() {
        // Every string that renders must parse back identical; the rest
        // must be refused outright, never mangled.
        for s in ["", "\n", "\n\n", "«", "⟨⟩ ∅", " lead and trail ", "»", "⏎", "x\ny", "\r"] {
            let v = Value::str(s);
            match render(&v) {
                Ok(line) => assert!(val_eq(&v, &parse(&line).unwrap()), "changed: {s:?}"),
                Err(_) => assert!(s.contains('»') || s.contains('⏎'), "refused: {s:?}"),
            }
        }
    }

    #[test]
    fn deep_nesting_is_capped_not_overflowed() {
        // 100 000 `⟨` on one line must produce an error, not a stack
        // overflow; a deep-but-bounded list still parses.
        let line = "⟨".repeat(100_000);
        assert!(parse(&line).is_err());
        let closed = format!("{}{}", "⟨".repeat(100_000), "⟩".repeat(100_000));
        assert!(parse(&closed).is_err());
        let ok = format!("{}1{}", "⟨".repeat(MAX_DEPTH - 1), "⟩".repeat(MAX_DEPTH - 1));
        assert!(parse(&ok).is_ok());

        // Rendering is capped the same way — and neither rendering nor
        // parsing a value at the cap needs more than a small stack.
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let mut v = Value::int(1);
                for _ in 0..MAX_DEPTH - 1 {
                    v = Value::List(Arc::new(vec![v]));
                }
                let line = render(&v).unwrap();
                let back = parse(&line).unwrap();
                assert_eq!(render(&back).unwrap(), line);
                // MAX_DEPTH open lists are allowed; one more is not.
                v = Value::List(Arc::new(vec![v]));
                assert!(render(&v).is_ok());
                v = Value::List(Arc::new(vec![v]));
                assert!(render(&v).is_err());
                let line = "⟨".repeat(100_000);
                assert!(parse(&line).is_err());
                // Dropping (or val_eq-ing) a value this deep recurses in
                // values.rs, which is not the codec's concern: leak them.
                std::mem::forget(back);
                std::mem::forget(v);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn huge_ints_round_trip() {
        let digits = "9".repeat(10_000);
        round_trip(Value::from_big(digits.parse().unwrap()));
        round_trip(Value::from_big(format!("-{digits}").parse().unwrap()));
        round_trip(Value::int(i64::MIN));
        round_trip(Value::int(i64::MIN + 1));
        round_trip(Value::from_big(BigInt::from(i64::MAX) + 1));
        round_trip(Value::from_big(BigInt::from(i64::MIN) - 1));
    }

    #[test]
    fn unterminated_final_line_is_not_a_line() {
        use std::io::BufReader;
        let mut r = BufReader::new("⟨1 2⟩\n«hal".as_bytes());
        assert_eq!(read_line(&mut r, MAX_LINE).unwrap(), "⟨1 2⟩");
        assert_eq!(read_line(&mut r, MAX_LINE).unwrap_err(), LineEnd::Truncated);

        let mut r = BufReader::new("∅\n".as_bytes());
        assert_eq!(read_line(&mut r, MAX_LINE).unwrap(), "∅");
        assert_eq!(read_line(&mut r, MAX_LINE).unwrap_err(), LineEnd::Eof);

        // Empty lines are lines; the cap holds across buffer refills.
        let mut r = BufReader::with_capacity(4, "\n123456789\n".as_bytes());
        assert_eq!(read_line(&mut r, 8).unwrap(), "");
        assert_eq!(read_line(&mut r, 8).unwrap_err(), LineEnd::TooLong);

        let mut r = BufReader::new(&[0xffu8, b'\n'][..]);
        assert_eq!(read_line(&mut r, MAX_LINE).unwrap_err(), LineEnd::NotUtf8);
    }
}
