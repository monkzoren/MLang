//! The MLang lexer — a faithful port of the reference implementation.

use crate::values::{Instr, Op, Pos, Value};
use std::rc::Rc;

#[derive(Clone, Copy, Debug)]
pub struct Cell {
    pub ch: char,
    pub row: u32,
    pub col: u32,
}

#[derive(Debug)]
pub struct LoadError {
    pub msg: String,
    pub pos: Option<Pos>,
}

impl LoadError {
    pub fn new(msg: impl Into<String>, pos: Option<Pos>) -> Self {
        LoadError { msg: msg.into(), pos }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Axis {
    Row,
    Col,
}

pub const OP_CHARS: &str = "∂⇅⌫⊚⥀≢+-×÷%^√⌊⌈±=≠<≤>≥∧∨¬⊻!?⟳⍣∵∀⌿⍀⍸#⧺@⊂⊆⊇⍕⍎⌗⍘⚡⋈⍳≣⌛⍥↯⍞⊸⌨⍟⍙⌽⍋∈⍷⍇⍈⌂";
pub const ARG_OP_CHARS: &str = "≔⇒↥↧⇂⇈⇟";
pub const ARG2_OP_CHARS: &str = "⇉";
const STRUCTURAL: &str = "«»⟨⟩[]⏎¯.※⋮⇓⇊∅ \t";

pub fn is_op(c: char) -> bool {
    OP_CHARS.contains(c)
}
pub fn is_arg_op(c: char) -> bool {
    ARG_OP_CHARS.contains(c)
}
pub fn is_arg2_op(c: char) -> bool {
    ARG2_OP_CHARS.contains(c)
}
pub fn is_reserved(c: char) -> bool {
    is_op(c) || is_arg_op(c) || is_arg2_op(c) || STRUCTURAL.contains(c) || c.is_ascii_digit()
}

struct Lexer {
    cells: Vec<Cell>,
    i: usize,
    axis: Axis,
}

type LResult<T> = Result<T, LoadError>;

impl Lexer {
    fn pos(&self, c: &Cell) -> Pos {
        (c.row, c.col)
    }

    fn err<T>(&self, msg: impl Into<String>, cell: &Cell) -> LResult<T> {
        Err(LoadError::new(msg, Some(self.pos(cell))))
    }

    fn same_line(&self, a: &Cell, b: &Cell) -> bool {
        match self.axis {
            Axis::Row => a.row == b.row,
            Axis::Col => a.col == b.col,
        }
    }

    fn skip_comment(&mut self, start: Cell) {
        while self.i < self.cells.len() && self.same_line(&self.cells[self.i], &start) {
            self.i += 1;
        }
    }

    fn skip_blank(&mut self) {
        while self.i < self.cells.len() {
            let c = self.cells[self.i];
            if c.ch == ' ' {
                self.i += 1;
            } else if c.ch == '※' {
                self.i += 1;
                self.skip_comment(c);
            } else {
                return;
            }
        }
    }

    fn number(&mut self, cell: Cell) -> LResult<Instr> {
        let mut s = String::new();
        if self.cells[self.i].ch == '¯' {
            s.push('-');
            self.i += 1;
            if self.i >= self.cells.len() || !self.cells[self.i].ch.is_ascii_digit() {
                return self.err("lone ¯ — negatives are written like ¯5", &cell);
            }
        }
        let mut dots = 0;
        while self.i < self.cells.len() {
            let ch = self.cells[self.i].ch;
            if ch.is_ascii_digit() || ch == '.' {
                if ch == '.' {
                    dots += 1;
                    if dots > 1 {
                        let bad = self.cells[self.i];
                        return self.err("number has two . points", &bad);
                    }
                }
                s.push(ch);
                self.i += 1;
            } else {
                break;
            }
        }
        let val = if dots > 0 {
            Value::Float(s.parse::<f64>().unwrap())
        } else {
            match s.parse::<i64>() {
                Ok(i) => Value::Int(i),
                Err(_) => Value::from_big(s.parse().unwrap()),
            }
        };
        Ok(Instr { op: Op::Push(val), pos: self.pos(&cell) })
    }

    fn string(&mut self, cell: Cell) -> LResult<Instr> {
        self.i += 1;
        let mut buf = String::new();
        while self.i < self.cells.len() && self.cells[self.i].ch != '»' {
            let ch = self.cells[self.i].ch;
            buf.push(if ch == '⏎' { '\n' } else { ch });
            self.i += 1;
        }
        if self.i >= self.cells.len() {
            return self.err("unterminated « string", &cell);
        }
        self.i += 1;
        Ok(Instr { op: Op::Push(Value::str(buf)), pos: self.pos(&cell) })
    }

    fn arg_char(&mut self, op_cell: Cell, op_ch: char) -> LResult<char> {
        self.skip_blank();
        if self.i >= self.cells.len() {
            return self.err(format!("{op_ch} needs a sigil after it"), &op_cell);
        }
        let name_cell = self.cells[self.i];
        self.i += 1;
        if is_reserved(name_cell.ch) {
            return self.err(
                format!("'{}' is reserved and cannot follow {}", name_cell.ch, op_ch),
                &name_cell,
            );
        }
        Ok(name_cell.ch)
    }

    fn parse(&mut self, until: Option<char>, open_cell: Option<Cell>) -> LResult<Vec<Instr>> {
        let mut code = Vec::new();
        loop {
            self.skip_blank();
            if self.i >= self.cells.len() {
                if until.is_some() {
                    return self.err("unclosed [ quotation", &open_cell.unwrap());
                }
                return Ok(code);
            }
            let cell = self.cells[self.i];
            let ch = cell.ch;
            if until == Some(ch) {
                self.i += 1;
                return Ok(code);
            }
            let next_is_digit = self.i + 1 < self.cells.len()
                && self.cells[self.i + 1].ch.is_ascii_digit();
            if ch.is_ascii_digit() || ch == '¯' || (ch == '.' && next_is_digit) {
                let instr = self.number(cell)?;
                code.push(instr);
            } else if ch == '«' {
                let instr = self.string(cell)?;
                code.push(instr);
            } else if ch == '»' {
                return self.err("» without matching «", &cell);
            } else if ch == '[' {
                self.i += 1;
                let inner = self.parse(Some(']'), Some(cell))?;
                code.push(Instr {
                    op: Op::Push(Value::Quot(Rc::new(inner))),
                    pos: self.pos(&cell),
                });
            } else if ch == ']' {
                return self.err("] without matching [", &cell);
            } else if ch == '⟨' {
                self.i += 1;
                code.push(Instr { op: Op::LMark, pos: self.pos(&cell) });
            } else if ch == '⟩' {
                self.i += 1;
                code.push(Instr { op: Op::LBuild, pos: self.pos(&cell) });
            } else if ch == '∅' {
                self.i += 1;
                code.push(Instr { op: Op::Push(Value::Nil), pos: self.pos(&cell) });
            } else if is_arg_op(ch) {
                self.i += 1;
                let arg = self.arg_char(cell, ch)?;
                code.push(Instr { op: Op::B(ch, arg, '\0'), pos: self.pos(&cell) });
            } else if is_arg2_op(ch) {
                self.i += 1;
                let a = self.arg_char(cell, ch)?;
                let b = self.arg_char(cell, ch)?;
                code.push(Instr { op: Op::B(ch, a, b), pos: self.pos(&cell) });
            } else if is_op(ch) {
                self.i += 1;
                code.push(Instr { op: Op::B(ch, '\0', '\0'), pos: self.pos(&cell) });
            } else if ch == '.' {
                return self.err("stray . — floats are written like 1.5 or .5", &cell);
            } else if ch == '⇓' {
                return self.err(
                    "⇓ marks rain form and must be alone on the first line \
                     of the file (flat form needs no marker)",
                    &cell,
                );
            } else if ch == '⇊' {
                return self.err(
                    "⇊ is the boot divider and must stand alone on \
                     its own line (or row, in rain form)",
                    &cell,
                );
            } else if ch == '⋮' {
                return self.err("⋮ continues a strand and must start a line", &cell);
            } else if ch == '⏎' {
                return self.err("⏎ is only meaningful inside « » strings", &cell);
            } else {
                self.i += 1;
                code.push(Instr { op: Op::Name(ch), pos: self.pos(&cell) });
            }
        }
    }
}

pub fn lex_strand(cells: Vec<Cell>, axis: Axis) -> LResult<Vec<Instr>> {
    Lexer { cells, i: 0, axis }.parse(None, None)
}
