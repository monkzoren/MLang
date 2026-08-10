"""Source forms: rain (vertical, canonical) and flat (horizontal, transpose).

Rain form — the Matrix screen. The file starts with a line containing ⇓.
Every grid column is one strand, executed top→bottom; all strands run
concurrently. A row whose first non-space character is ⇊ divides the file:
rows above it are the boot section (run to completion, left-to-right, before
the rain starts — the place for ≔ definitions), rows below are the strands.

Flat form — the transpose, easy for linear text generation. One line is one
strand, executed left→right. A line containing only ⇊ divides boot lines
(above) from strand lines (below). A line starting with ⋮ continues the
previous strand.

The two forms are semantically identical: rain is the transpose of flat.
"""

from .errors import LoadError
from .lex import Cell

GUTTER = 2  # blank columns between strands when rendering rain


class Program:
    def __init__(self, boot_cells, strands, axis):
        self.boot_cells = boot_cells      # list[Cell] | None
        self.strands = strands            # list[(label, list[Cell])]
        self.axis = axis                  # 'row' | 'col'


def parse_source(text):
    if "\t" in text:
        row = text[: text.index("\t")].count("\n") + 1
        raise LoadError("tab characters break the grid — use spaces", (row, 1))
    lines = text.splitlines()
    if lines and lines[0].strip() == "⇓":
        return _parse_rain(lines)
    return _parse_flat(lines)


# ── flat form ──────────────────────────────────────────────────────────
def _line_cells(line, row, start_col=0):
    return [Cell(ch, row, c + 1) for c, ch in enumerate(line) if c >= start_col]


def _parse_flat(lines):
    sections = [[], []]  # boot strands, main strands (each: list of cell-lists)
    section = 1  # with no ⇊ divider, everything is a main strand
    if any(ln.strip() == "⇊" for ln in lines):
        section = 0
    for i, line in enumerate(lines):
        row = i + 1
        stripped = line.strip()
        if not stripped:
            continue
        if stripped == "⇊":
            if section == 1 and sections[0]:
                raise LoadError("second ⇊ divider", (row, 1))
            section = 1
            continue
        if stripped.startswith("⋮"):
            bucket = sections[section]
            if not bucket:
                raise LoadError("⋮ continuation with nothing to continue", (row, 1))
            start = line.index("⋮") + 1
            bucket[-1].append(Cell(" ", row, start))  # keep number runs apart
            bucket[-1].extend(_line_cells(line, row, start_col=start))
            continue
        sections[section].append(_line_cells(line, row))
    boot = None
    if sections[0]:
        boot = []
        for cells in sections[0]:
            if boot:
                boot.append(Cell(" ", cells[0].row, 0))
            boot.extend(cells)
    strands = [
        (f"row {cells[0].row}", cells) for cells in sections[1] if cells
    ]
    return Program(boot, strands, "row")


# ── rain form ──────────────────────────────────────────────────────────
def _parse_rain(lines):
    rows = lines[1:]  # after the ⇓ sigil line
    row0 = 2  # 1-based row number of the first grid row in the file
    divider = None
    for i, ln in enumerate(rows):
        s = ln.strip()
        if s and s[0] == "⇊":
            divider = i
            break
    pre_rows = range(0, divider) if divider is not None else range(0)
    main_start = divider + 1 if divider is not None else 0
    main_rows = range(main_start, len(rows))
    width = max((len(ln) for ln in rows), default=0)

    def cell(r, c):
        ch = rows[r][c] if c < len(rows[r]) else " "
        return Cell(ch, r + row0, c + 1)

    boot = None
    if divider is not None:
        boot = []
        for c in range(width):
            col = [cell(r, c) for r in pre_rows]
            if any(x.ch != " " for x in col):
                if boot:
                    boot.append(Cell(" ", row0, c + 1))
                boot.extend(col)
        if not boot:
            boot = None

    strands = []
    for c in range(width):
        col = [cell(r, c) for r in main_rows]
        if any(x.ch != " " for x in col):
            strands.append((f"col {c + 1}", col))
    return Program(boot, strands, "col")


# ── renderers ──────────────────────────────────────────────────────────
def _strand_strings(prog):
    boot = None
    if prog.boot_cells:
        boot = "".join(c.ch for c in prog.boot_cells).strip()
    return boot, ["".join(c.ch for c in cells).rstrip() for _, cells in prog.strands]


def to_rain(text):
    """Render flat source as the canonical vertical rain grid."""
    prog = parse_source(text)
    if prog.axis == "col":
        raise LoadError("already in rain form")
    boot, strands = _strand_strings(prog)
    out = ["⇓"]
    if boot:
        out.extend(ch for ch in boot)
        out.append("⇊")
    height = max((len(s) for s in strands), default=0)
    for r in range(height):
        row = []
        for s in strands:
            row.append(s[r] if r < len(s) else " ")
            row.append(" " * GUTTER)
        out.append("".join(row).rstrip())
    return "\n".join(out) + "\n"


def to_flat(text):
    """Render rain source as flat lines (one strand per line)."""
    prog = parse_source(text)
    if prog.axis == "row":
        raise LoadError("already in flat form")
    boot, strands = _strand_strings(prog)
    out = []
    if boot:
        out.append(" ".join(boot.split()))
        out.append("⇊")
    out.extend(strands)
    return "\n".join(out) + "\n"
