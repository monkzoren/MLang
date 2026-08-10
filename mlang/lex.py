"""The MLang lexer/weaver: turns a strand's cell stream into instructions.

A strand arrives as a sequence of Cells — (char, row, col) — produced either
from one grid column (rain form) or one source line (flat form). The lexer is
identical for both; only the comment axis differs (※ comments run to the end
of the physical line in flat form, to the end of the column in rain form).
"""

from collections import namedtuple

from .errors import LoadError
from .sigils import ARG_OPS, DIGITS, OPS, RESERVED
from .values import NIL, Quot

Cell = namedtuple("Cell", "ch row col")
Instr = namedtuple("Instr", "op arg pos")


class Lexer:
    def __init__(self, cells, axis):
        self.cells = list(cells)
        self.i = 0
        self.axis = axis  # 'row' (flat form) or 'col' (rain form)

    # ── helpers ────────────────────────────────────────────────────────
    def _pos(self, cell):
        return (cell.row, cell.col)

    def _err(self, msg, cell=None):
        raise LoadError(msg, self._pos(cell) if cell else None)

    def _same_line(self, a, b):
        return a.row == b.row if self.axis == "row" else a.col == b.col

    def _skip_comment(self, start):
        while self.i < len(self.cells) and self._same_line(self.cells[self.i], start):
            self.i += 1

    def _skip_blank(self):
        while self.i < len(self.cells):
            c = self.cells[self.i]
            if c.ch == " ":
                self.i += 1
            elif c.ch == "※":
                self.i += 1
                self._skip_comment(c)
            else:
                return

    # ── token parsers ──────────────────────────────────────────────────
    def _number(self, cell):
        s = ""
        if self.cells[self.i].ch == "¯":
            s = "-"
            self.i += 1
            if self.i >= len(self.cells) or self.cells[self.i].ch not in DIGITS:
                self._err("lone ¯ — negatives are written like ¯5", cell)
        dots = 0
        while self.i < len(self.cells) and (
            self.cells[self.i].ch in DIGITS or self.cells[self.i].ch == "."
        ):
            ch = self.cells[self.i].ch
            if ch == ".":
                dots += 1
                if dots > 1:
                    self._err("number has two . points", self.cells[self.i])
            s += ch
            self.i += 1
        val = float(s) if dots else int(s)
        return Instr("push", val, self._pos(cell))

    def _string(self, cell):
        self.i += 1
        buf = []
        while self.i < len(self.cells) and self.cells[self.i].ch != "»":
            ch = self.cells[self.i].ch
            buf.append("\n" if ch == "⏎" else ch)
            self.i += 1
        if self.i >= len(self.cells):
            self._err("unterminated « string", cell)
        self.i += 1
        return Instr("push", "".join(buf), self._pos(cell))

    def _arg_char(self, op_cell, op_ch):
        self._skip_blank()
        if self.i >= len(self.cells):
            self._err(f"{op_ch} needs a sigil after it", op_cell)
        name_cell = self.cells[self.i]
        self.i += 1
        if name_cell.ch in RESERVED:
            self._err(
                f"'{name_cell.ch}' is reserved and cannot follow {op_ch}", name_cell
            )
        return name_cell.ch

    # ── main loop ──────────────────────────────────────────────────────
    def parse(self, until=None, open_cell=None):
        code = []
        while True:
            self._skip_blank()
            if self.i >= len(self.cells):
                if until:
                    self._err(f"unclosed [ quotation", open_cell)
                return code
            cell = self.cells[self.i]
            ch = cell.ch
            if until and ch == until:
                self.i += 1
                return code
            if ch in DIGITS or ch == "¯" or (
                ch == "."
                and self.i + 1 < len(self.cells)
                and self.cells[self.i + 1].ch in DIGITS
            ):
                code.append(self._number(cell))
            elif ch == "«":
                code.append(self._string(cell))
            elif ch == "»":
                self._err("» without matching «", cell)
            elif ch == "[":
                self.i += 1
                inner = self.parse(until="]", open_cell=cell)
                code.append(Instr("push", Quot(inner), self._pos(cell)))
            elif ch == "]":
                self._err("] without matching [", cell)
            elif ch == "⟨":
                self.i += 1
                code.append(Instr("lmark", None, self._pos(cell)))
            elif ch == "⟩":
                self.i += 1
                code.append(Instr("lbuild", None, self._pos(cell)))
            elif ch == "∅":
                self.i += 1
                code.append(Instr("push", NIL, self._pos(cell)))
            elif ch in ARG_OPS:
                self.i += 1
                arg = self._arg_char(cell, ch)
                code.append(Instr(ch, arg, self._pos(cell)))
            elif ch in OPS:
                self.i += 1
                code.append(Instr(ch, None, self._pos(cell)))
            elif ch == ".":
                self._err("stray . — floats are written like 1.5 or .5", cell)
            else:
                # Any other Unicode character is a user sigil reference.
                self.i += 1
                code.append(Instr("name", ch, self._pos(cell)))


def lex_strand(cells, axis):
    return Lexer(cells, axis).parse()
