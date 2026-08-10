"""MLang runtime values.

Every value is immutable: integers (arbitrary precision), floats, strings,
lists (tuples), quotations (code blocks), and the nil sentinel ∅. Immutability
plus channel-only communication is what makes MLang data-race-free.
"""


class _Nil:
    __slots__ = ()

    def __repr__(self):
        return "∅"


NIL = _Nil()


class _Mark:
    """Internal stack marker used while building a ⟨ ⟩ list literal."""

    __slots__ = ()

    def __repr__(self):
        return "⟨"


MARK = _Mark()


class Quot:
    """A quotation: deferred code pushed on the stack by [ ... ]."""

    __slots__ = ("code",)

    def __init__(self, code):
        self.code = code  # list[Instr]

    def __repr__(self):
        return "[⋯]"


def truthy(v):
    if v is NIL:
        return False
    if isinstance(v, Quot):
        return True
    if isinstance(v, (int, float, str, tuple)):
        return bool(v)
    return True


def _num_str(x):
    if isinstance(x, float) and x == int(x) and abs(x) < 1e16:
        s = repr(x)
    else:
        s = repr(x) if isinstance(x, float) else str(x)
    return s.replace("-", "¯")


def fmt(v, quote=False):
    """Render a value. quote=True renders strings as «...» (debug style)."""
    if v is NIL:
        return "∅"
    if isinstance(v, bool):  # bools never escape ops, but be safe
        return "1" if v else "0"
    if isinstance(v, (int, float)):
        return _num_str(v)
    if isinstance(v, str):
        return f"«{v}»" if quote else v
    if isinstance(v, tuple):
        return "⟨" + " ".join(fmt(x, quote=True) for x in v) + "⟩"
    if isinstance(v, Quot):
        return "[⋯]"
    if v is MARK:
        return "⟨"
    return repr(v)


def type_name(v):
    if v is NIL:
        return "∅"
    if isinstance(v, bool) or isinstance(v, int):
        return "int"
    if isinstance(v, float):
        return "float"
    if isinstance(v, str):
        return "str"
    if isinstance(v, tuple):
        return "list"
    if isinstance(v, Quot):
        return "quot"
    return type(v).__name__
