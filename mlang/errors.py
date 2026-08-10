"""Error types for the MLang weave."""


class LoadError(Exception):
    """The source text could not be woven into a program."""

    def __init__(self, msg, pos=None):
        super().__init__(msg)
        self.msg = msg
        self.pos = pos  # (row, col), 1-based, in the original source file


class MGlitch(Exception):
    """A runtime fault inside a strand. Carries an arbitrary MLang value."""

    def __init__(self, value, pos=None):
        super().__init__(str(value))
        self.value = value
        self.pos = pos


class BlockSignal(Exception):
    """Raised by an op that cannot proceed yet (channel empty, join pending).

    The instruction pointer is NOT advanced, so the op re-executes when the
    strand is next scheduled.
    """

    def __init__(self, kind, key, pos=None):
        super().__init__(f"blocked on {kind} {key}")
        self.kind = kind  # 'chan' | 'strand'
        self.key = key
        self.pos = pos


class YieldSignal(Exception):
    """Raised by ⌛ to voluntarily end the strand's scheduler slice."""
