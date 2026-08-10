"""The MLang virtual machine.

Execution model:
  * Each strand (grid column) is an independent machine with its own stack,
    locals, and frame stack. Values are immutable; strands share nothing.
  * Strands communicate only over named channels (unbounded FIFOs): sends
    never block, receives block until a value arrives. No shared memory
    means no data races, by construction.
  * A deterministic round-robin scheduler gives each strand SLICE
    instructions per turn. Identical inputs always produce identical runs.
  * A runtime fault is a "glitch". It unwinds to the nearest ⍥ handler in
    its strand; uncaught, it kills only that strand. If every remaining
    strand is blocked, the scheduler reports a deadlock with exact
    coordinates instead of hanging.
"""

import sys
from collections import deque

from .errors import BlockSignal, LoadError, MGlitch, YieldSignal
from .forms import parse_source
from .lex import lex_strand
from .sigils import ARG2_OPS, ARG_OPS, OPS, RESERVED
from .values import MARK, NIL, Quot, fmt, truthy, type_name

SLICE = 8  # instructions per strand per scheduler turn (fixed → deterministic)

RUN, BLOCKED, DONE, DEAD = "run", "blocked", "done", "dead"


def glitch(msg, pos=None):
    raise MGlitch(msg, pos)


# ── frames ─────────────────────────────────────────────────────────────
class CF:
    """A plain code frame: a strip of instructions and a pointer into it."""

    __slots__ = ("code", "ip")

    def __init__(self, code):
        self.code = code
        self.ip = 0

    def step(self, vm, strand):
        if self.ip >= len(self.code):
            strand.frames.pop()
            return
        instr = self.code[self.ip]
        try:
            execute(vm, strand, instr)  # on BlockSignal the ip stays put
        except YieldSignal:
            self.ip += 1  # yield completed — resume at the next instruction
            raise
        self.ip += 1


class WhileF:
    __slots__ = ("cond", "body", "phase")

    def __init__(self, cond, body):
        self.cond = cond
        self.body = body
        self.phase = 0  # 0 → run cond, 1 → inspect flag

    def step(self, vm, strand):
        if self.phase == 0:
            self.phase = 1
            strand.frames.append(CF(self.cond.code))
        else:
            self.phase = 0
            if truthy(strand.pop(None)):
                strand.frames.append(CF(self.body.code))
            else:
                strand.frames.pop()


class RepeatF:
    __slots__ = ("left", "body")

    def __init__(self, n, body):
        self.left = n
        self.body = body

    def step(self, vm, strand):
        if self.left <= 0:
            strand.frames.pop()
        else:
            self.left -= 1
            strand.frames.append(CF(self.body.code))


class IterF:
    """Drives ∵ map, ∀ each, ⌿ filter, ⍀ fold over a list or string."""

    __slots__ = ("items", "i", "f", "mode", "out", "awaiting")

    def __init__(self, items, f, mode):
        self.items = items
        self.i = 0
        self.f = f
        self.mode = mode  # 'map' | 'each' | 'filter' | 'fold'
        self.out = []
        self.awaiting = False

    def step(self, vm, strand):
        if self.awaiting:
            self.awaiting = False
            if self.mode == "map":
                self.out.append(strand.pop(None))
            elif self.mode == "filter":
                if truthy(strand.pop(None)):
                    self.out.append(self.items[self.i - 1])
        if self.i >= len(self.items):
            strand.frames.pop()
            if self.mode in ("map", "filter"):
                strand.push(tuple(self.out))
            return
        strand.push(self.items[self.i])
        self.i += 1
        self.awaiting = True
        strand.frames.append(CF(self.f.code))


class DrainF:
    """Drives ⇟: collects from a channel until ∅, then pushes the list."""

    __slots__ = ("chan", "out", "pos")

    def __init__(self, chan, pos):
        self.chan = chan
        self.out = []
        self.pos = pos

    def step(self, vm, strand):
        q = vm.channels.setdefault(self.chan, deque())
        if not q:
            raise BlockSignal("chan", self.chan, self.pos)
        v = q.popleft()
        if v is NIL:
            strand.frames.pop()
            strand.push(tuple(self.out))
        else:
            self.out.append(v)


class PumpF:
    """Drives ⇉: recv from src, run f on each value, send result to dst.

    On ∅ the end-marker is forwarded to dst and the pump stops.
    """

    __slots__ = ("src", "dst", "f", "phase", "pos")

    def __init__(self, src, dst, f, pos):
        self.src = src
        self.dst = dst
        self.f = f
        self.phase = 0  # 0 → receive next value, 1 → f finished, send result
        self.pos = pos

    def step(self, vm, strand):
        if self.phase == 0:
            q = vm.channels.setdefault(self.src, deque())
            if not q:
                raise BlockSignal("chan", self.src, self.pos)
            v = q.popleft()
            if v is NIL:
                vm.channels.setdefault(self.dst, deque()).append(NIL)
                strand.frames.pop()
                return
            self.phase = 1
            strand.push(v)
            strand.frames.append(CF(self.f.code))
        else:
            self.phase = 0
            v = strand.pop(self.pos, "the pump body's result")
            vm.channels.setdefault(self.dst, deque()).append(v)


class TryF:
    """Armed by ⍥. Catches glitches raised anywhere above it."""

    __slots__ = ("handler", "depth")

    def __init__(self, handler, depth):
        self.handler = handler
        self.depth = depth

    def step(self, vm, strand):
        strand.frames.pop()  # body finished cleanly — disarm


# ── strand ─────────────────────────────────────────────────────────────
class Strand:
    def __init__(self, sid, label, code, locals_=None):
        self.sid = sid
        self.label = label
        self.frames = [CF(code)]
        self.stack = []
        self.locals = dict(locals_) if locals_ else {}
        self.status = RUN
        self.block = None    # the BlockSignal we are parked on
        self.glitch = None   # the MGlitch that killed us

    def push(self, v):
        self.stack.append(v)

    def pop(self, pos, what="a value"):
        if not self.stack:
            glitch(f"stack underflow — needed {what}", pos)
        return self.stack.pop()

    def pop_num(self, pos, op):
        v = self.pop(pos, "a number")
        if not isinstance(v, (int, float)) or isinstance(v, bool):
            glitch(f"{op} expects numbers, got {type_name(v)}", pos)
        return v

    def pop_quot(self, pos, op):
        v = self.pop(pos, "a quotation")
        if not isinstance(v, Quot):
            glitch(f"{op} expects a [quotation], got {type_name(v)}", pos)
        return v

    def pop_seq(self, pos, op):
        v = self.pop(pos, "a list or string")
        if isinstance(v, str):
            return tuple(v), True
        if isinstance(v, tuple):
            return v, False
        glitch(f"{op} expects a list or string, got {type_name(v)}", pos)

    def catch(self, g):
        """Unwind to the nearest TryF. Returns True if the glitch was caught."""
        while self.frames:
            top = self.frames[-1]
            if isinstance(top, TryF):
                del self.stack[top.depth :]
                self.frames.pop()
                self.push(g.value)
                self.frames.append(CF(top.handler.code))
                return True
            self.frames.pop()
        return False

    def run_slice(self, vm):
        executed = 0
        while executed < SLICE:
            if not self.frames:
                self.status = DONE
                break
            try:
                self.frames[-1].step(vm, self)
                executed += 1
            except BlockSignal as b:
                self.status = BLOCKED
                self.block = b
                break
            except YieldSignal:
                executed += 1
                break
            except MGlitch as g:
                executed += 1
                if not self.catch(g):
                    self.status = DEAD
                    self.glitch = g
                    break
        return executed


# ── instruction dispatch ───────────────────────────────────────────────
def execute(vm, strand, instr):
    op, arg, pos = instr
    if op == "push":
        strand.push(arg)
    elif op == "name":
        if arg in strand.locals:
            v = strand.locals[arg]
        elif arg in vm.globals:
            v = vm.globals[arg]
        else:
            glitch(f"undefined sigil '{arg}'", pos)
            return
        if isinstance(v, Quot):
            strand.frames.append(CF(v.code))
        else:
            strand.push(v)
    elif op == "lmark":
        strand.push(MARK)
    elif op == "lbuild":
        items = []
        while strand.stack and strand.stack[-1] is not MARK:
            items.append(strand.stack.pop())
        if not strand.stack:
            glitch("⟩ without matching ⟨", pos)
        strand.stack.pop()  # the mark
        strand.push(tuple(reversed(items)))
    else:
        BUILTIN[op](vm, strand, arg, pos)


def _numeric(op):
    def wrap(fn):
        def impl(vm, s, arg, pos):
            b = s.pop_num(pos, op)
            a = s.pop_num(pos, op)
            s.push(fn(a, b, pos))
        return impl
    return wrap


def _cmp(op, fn):
    def impl(vm, s, arg, pos):
        b = s.pop(pos)
        a = s.pop(pos)
        num = lambda v: isinstance(v, (int, float)) and not isinstance(v, bool)
        if not ((num(a) and num(b)) or (isinstance(a, str) and isinstance(b, str))):
            glitch(f"{op} compares two numbers or two strings, got "
                   f"{type_name(a)} {type_name(b)}", pos)
        s.push(1 if fn(a, b) else 0)
    return impl


def _build_builtins():
    B = {}

    # stack
    def dup(vm, s, a, p):
        v = s.pop(p)
        s.push(v); s.push(v)
    def swap(vm, s, a, p):
        b, x = s.pop(p), s.pop(p)
        s.push(b); s.push(x)
    def drop(vm, s, a, p):
        s.pop(p)
    def over(vm, s, a, p):
        b, x = s.pop(p), s.pop(p)
        s.push(x); s.push(b); s.push(x)
    def rot(vm, s, a, p):
        c, b, x = s.pop(p), s.pop(p), s.pop(p)
        s.push(b); s.push(c); s.push(x)
    def depth(vm, s, a, p):
        s.push(len(s.stack))
    B["∂"], B["⇅"], B["⌫"], B["⊚"], B["⥀"], B["≢"] = dup, swap, drop, over, rot, depth

    # arithmetic
    @_numeric("+")
    def add(a, b, p): return a + b
    @_numeric("-")
    def sub(a, b, p): return a - b
    @_numeric("×")
    def mul(a, b, p): return a * b
    @_numeric("÷")
    def div(a, b, p):
        if b == 0:
            glitch("÷ by zero", p)
        if isinstance(a, int) and isinstance(b, int):
            return a // b if a % b == 0 else a / b
        return a / b
    @_numeric("%")
    def mod(a, b, p):
        if b == 0:
            glitch("% by zero", p)
        return a % b
    @_numeric("^")
    def pw(a, b, p): return a ** b
    B["+"], B["-"], B["×"], B["÷"], B["%"], B["^"] = add, sub, mul, div, mod, pw

    def sqrt(vm, s, a, p):
        v = s.pop_num(p, "√")
        if v < 0:
            glitch("√ of a negative number", p)
        s.push(v ** 0.5)
    def floor(vm, s, a, p):
        import math
        s.push(math.floor(s.pop_num(p, "⌊")))
    def ceil(vm, s, a, p):
        import math
        s.push(math.ceil(s.pop_num(p, "⌈")))
    def neg(vm, s, a, p):
        s.push(-s.pop_num(p, "±"))
    B["√"], B["⌊"], B["⌈"], B["±"] = sqrt, floor, ceil, neg

    # comparison / logic
    def eq(vm, s, a, p):
        b, x = s.pop(p), s.pop(p)
        s.push(1 if x == b else 0)
    def ne(vm, s, a, p):
        b, x = s.pop(p), s.pop(p)
        s.push(0 if x == b else 1)
    B["="], B["≠"] = eq, ne
    B["<"] = _cmp("<", lambda a, b: a < b)
    B["≤"] = _cmp("≤", lambda a, b: a <= b)
    B[">"] = _cmp(">", lambda a, b: a > b)
    B["≥"] = _cmp("≥", lambda a, b: a >= b)

    def and_(vm, s, a, p):
        b, x = s.pop(p), s.pop(p)
        s.push(1 if truthy(x) and truthy(b) else 0)
    def or_(vm, s, a, p):
        b, x = s.pop(p), s.pop(p)
        s.push(1 if truthy(x) or truthy(b) else 0)
    def not_(vm, s, a, p):
        s.push(0 if truthy(s.pop(p)) else 1)
    def xor(vm, s, a, p):
        b, x = s.pop(p), s.pop(p)
        s.push(1 if truthy(x) != truthy(b) else 0)
    B["∧"], B["∨"], B["¬"], B["⊻"] = and_, or_, not_, xor

    # control
    def apply(vm, s, a, p):
        q = s.pop_quot(p, "!")
        s.frames.append(CF(q.code))
    def if_(vm, s, a, p):
        e, t = s.pop(p), s.pop(p)
        c = s.pop(p)
        pick = t if truthy(c) else e
        if isinstance(pick, Quot):
            s.frames.append(CF(pick.code))
        else:
            s.push(pick)
    def while_(vm, s, a, p):
        body = s.pop_quot(p, "⟳")
        cond = s.pop_quot(p, "⟳")
        s.frames.append(WhileF(cond, body))
    def repeat(vm, s, a, p):
        body = s.pop_quot(p, "⍣")
        n = s.pop_num(p, "⍣")
        s.frames.append(RepeatF(int(n), body))
    B["!"], B["?"], B["⟳"], B["⍣"] = apply, if_, while_, repeat

    # iteration
    def _iter(mode):
        def impl(vm, s, a, p):
            f = s.pop_quot(p, mode)
            items, _ = s.pop_seq(p, mode)
            s.frames.append(IterF(items, f, mode))
        return impl
    B["∵"], B["∀"], B["⌿"] = _iter("map"), _iter("each"), _iter("filter")
    def fold(vm, s, a, p):
        f = s.pop_quot(p, "⍀")
        acc = s.pop(p, "a fold seed")
        items, _ = s.pop_seq(p, "⍀")
        s.push(acc)
        s.frames.append(IterF(items, f, "fold"))
    B["⍀"] = fold
    def range_(vm, s, a, p):
        n = s.pop_num(p, "⍸")
        s.push(tuple(range(int(n))))
    B["⍸"] = range_

    # sequences
    def length(vm, s, a, p):
        v = s.pop(p, "a list or string")
        if not isinstance(v, (str, tuple)):
            glitch(f"# expects a list or string, got {type_name(v)}", p)
        s.push(len(v))
    def concat(vm, s, a, p):
        b, x = s.pop(p), s.pop(p)
        if isinstance(x, str) and isinstance(b, str):
            s.push(x + b)
        elif isinstance(x, tuple) and isinstance(b, tuple):
            s.push(x + b)
        else:
            glitch(f"⧺ joins two strings or two lists, got "
                   f"{type_name(x)} {type_name(b)}", p)
    def index(vm, s, a, p):
        i = s.pop_num(p, "@")
        v = s.pop(p, "a list or string")
        if not isinstance(v, (str, tuple)):
            glitch(f"@ expects a list or string, got {type_name(v)}", p)
        i = int(i)
        if i < 0 or i >= len(v):
            glitch(f"@ index {fmt(i)} out of bounds (length {len(v)})", p)
        s.push(v[i])
    def slice_(vm, s, a, p):
        j = int(s.pop_num(p, "⊂"))
        i = int(s.pop_num(p, "⊂"))
        v = s.pop(p, "a list or string")
        if not isinstance(v, (str, tuple)):
            glitch(f"⊂ expects a list or string, got {type_name(v)}", p)
        s.push(v[max(i, 0) : max(j, 0)])
    def split(vm, s, a, p):
        sep = s.pop(p, "a separator string")
        v = s.pop(p, "a string")
        if not (isinstance(v, str) and isinstance(sep, str)):
            glitch(f"⊆ expects string sep-string, got "
                   f"{type_name(v)} {type_name(sep)}", p)
        s.push(tuple(v) if sep == "" else tuple(v.split(sep)))
    def join(vm, s, a, p):
        sep = s.pop(p, "a separator string")
        items, _ = s.pop_seq(p, "⊇")
        if not isinstance(sep, str):
            glitch(f"⊇ expects a string separator, got {type_name(sep)}", p)
        s.push(sep.join(fmt(x) for x in items))
    def to_str(vm, s, a, p):
        s.push(fmt(s.pop(p)))
    def parse_num(vm, s, a, p):
        v = s.pop(p, "a string")
        if not isinstance(v, str):
            glitch(f"⍎ expects a string, got {type_name(v)}", p)
        t = v.strip().replace("¯", "-")
        try:
            s.push(float(t) if ("." in t or "e" in t or "E" in t) else int(t))
        except ValueError:
            glitch(f"⍎ cannot parse «{v}» as a number", p)
    def codepoint(vm, s, a, p):
        v = s.pop(p, "a 1-char string")
        if not (isinstance(v, str) and len(v) == 1):
            glitch("⌗ expects a 1-character string", p)
        s.push(ord(v))
    def char(vm, s, a, p):
        n = int(s.pop_num(p, "⍘"))
        if n < 0 or n > 0x10FFFF:
            glitch(f"⍘ code point {fmt(n)} out of range", p)
        s.push(chr(n))
    B["#"], B["⧺"], B["@"], B["⊂"] = length, concat, index, slice_
    B["⊆"], B["⊇"], B["⍕"], B["⍎"] = split, join, to_str, parse_num
    B["⌗"], B["⍘"] = codepoint, char

    # bindings
    def define(vm, s, a, p):
        if a in vm.globals:
            glitch(f"sigil '{a}' is already defined", p)
        vm.globals[a] = s.pop(p, "a value to bind")
    def store(vm, s, a, p):
        s.locals[a] = s.pop(p, "a value to store")
    B["≔"], B["⇒"] = define, store

    # strands & channels
    def send(vm, s, a, p):
        vm.channels.setdefault(a, deque()).append(s.pop(p, "a value to send"))
    def recv(vm, s, a, p):
        ch = vm.channels.setdefault(a, deque())
        if not ch:
            raise BlockSignal("chan", a, p)
        s.push(ch.popleft())
    def try_recv(vm, s, a, p):
        ch = vm.channels.setdefault(a, deque())
        if ch:
            s.push(ch.popleft())
            s.push(1)
        else:
            s.push(0)
    def pour(vm, s, a, p):
        items, _ = s.pop_seq(p, "⇈")
        q = vm.channels.setdefault(a, deque())
        q.extend(items)
        q.append(NIL)
    def drain(vm, s, a, p):
        s.frames.append(DrainF(a, p))
    def pump(vm, s, a, p):
        f = s.pop_quot(p, "⇉")
        src, dst = a
        s.frames.append(PumpF(src, dst, f, p))
    def spawn(vm, s, a, p):
        q = s.pop_quot(p, "⚡")
        child = vm.spawn(q, s)
        s.push(child.sid)
    def join_(vm, s, a, p):
        # Peek, don't pop: a blocked op re-executes, so the stack must be
        # untouched until the join can actually complete.
        if not s.stack:
            glitch("stack underflow — ⋈ needs a strand id", p)
        sid = s.stack[-1]
        if not isinstance(sid, (int, float)) or isinstance(sid, bool):
            glitch(f"⋈ expects a strand id, got {type_name(sid)}", p)
        target = vm.by_sid.get(int(sid))
        if target is None:
            glitch(f"⋈ no strand with id {fmt(int(sid))}", p)
        if target.status not in (DONE, DEAD):
            raise BlockSignal("strand", int(sid), p)
        s.stack.pop()
    def strand_id(vm, s, a, p):
        s.push(s.sid)
    def strand_count(vm, s, a, p):
        s.push(vm.main_count)
    def yield_(vm, s, a, p):
        raise YieldSignal()
    B["↥"], B["↧"], B["⇂"], B["⚡"] = send, recv, try_recv, spawn
    B["⋈"], B["⍳"], B["≣"], B["⌛"] = join_, strand_id, strand_count, yield_
    B["⇈"], B["⇟"], B["⇉"] = pour, drain, pump

    # glitches
    def try_(vm, s, a, p):
        handler = s.pop_quot(p, "⍥")
        body = s.pop_quot(p, "⍥")
        s.frames.append(TryF(handler, len(s.stack)))
        s.frames.append(CF(body.code))
    def raise_(vm, s, a, p):
        raise MGlitch(s.pop(p, "a value to raise"), p)
    B["⍥"], B["↯"] = try_, raise_

    # i/o
    def println(vm, s, a, p):
        vm.out.write(fmt(s.pop(p)) + "\n")
    def print_(vm, s, a, p):
        vm.out.write(fmt(s.pop(p)))
    def readline(vm, s, a, p):
        line = vm.stdin.readline()
        s.push(NIL if line == "" else line.rstrip("\n"))
    def debug(vm, s, a, p):
        items = " ".join(fmt(v, quote=True) for v in s.stack)
        vm.err.write(f"⍟ strand {fmt(s.sid)} ({s.label}): {items}\n")
    B["⍞"], B["⊸"], B["⌨"], B["⍟"] = println, print_, readline, debug

    missing = (set(OPS) | set(ARG_OPS) | set(ARG2_OPS)) - set(B)
    assert not missing, f"ops without implementations: {missing}"
    return B


BUILTIN = _build_builtins()


# ── the machine ────────────────────────────────────────────────────────
class VM:
    def __init__(self, stdin=None, stdout=None, stderr=None):
        self.stdin = stdin if stdin is not None else sys.stdin
        self.out = stdout if stdout is not None else sys.stdout
        self.err = stderr if stderr is not None else sys.stderr
        self.globals = {}
        self.channels = {}
        self.strands = []       # strands active in the scheduler
        self.by_sid = {}
        self.main_count = 0
        self.next_spawn_sid = 0
        self.failed = False

    # strand management
    def _register(self, strand):
        self.by_sid[strand.sid] = strand
        self.strands.append(strand)

    def spawn(self, quot, parent):
        sid = self.next_spawn_sid
        self.next_spawn_sid += 1
        child = Strand(sid, f"⚡ of strand {fmt(parent.sid)}", quot.code,
                       locals_=parent.locals)
        self._register(child)
        return child

    # reporting
    def _coords(self, pos):
        return f"{pos[0]}:{pos[1]}" if pos else "?"

    def report_glitch(self, strand):
        g = strand.glitch
        self.failed = True
        self.err.write(
            f"✗ glitch in strand {fmt(strand.sid)} ({strand.label}) "
            f"at {self._coords(g.pos)}: {fmt(g.value)}\n"
        )

    def report_deadlock(self, blocked):
        self.failed = True
        self.err.write("✗ deadlock — every remaining strand is blocked:\n")
        for s in blocked:
            b = s.block
            what = f"channel {b.key}" if b.kind == "chan" else f"strand {fmt(b.key)}"
            self.err.write(
                f"  strand {fmt(s.sid)} ({s.label}) waiting on {what} "
                f"at {self._coords(b.pos)}\n"
            )

    # scheduling
    def _unblock(self, s):
        b = s.block
        if b.kind == "chan":
            ch = self.channels.get(b.key)
            if ch:
                s.status = RUN
                s.block = None
        else:
            t = self.by_sid.get(b.key)
            if t is not None and t.status in (DONE, DEAD):
                s.status = RUN
                s.block = None

    def run_scheduler(self):
        while True:
            progressed = 0
            for s in list(self.strands):
                if s.status == BLOCKED:
                    self._unblock(s)
                if s.status == RUN:
                    progressed += s.run_slice(self)
                    if s.status == DEAD:
                        self.report_glitch(s)
            live = [s for s in self.strands if s.status in (RUN, BLOCKED)]
            if not live:
                return
            if progressed == 0:
                blocked = [s for s in live if s.status == BLOCKED]
                if blocked and len(blocked) == len(live):
                    self.report_deadlock(blocked)
                    return

    # program entry
    def run_program(self, prog):
        woven = []
        for label, cells in prog.strands:
            code = lex_strand(cells, prog.axis)
            if code:  # comment-only lines/columns are not strands
                woven.append((label, code))
        main = [Strand(i, label, code) for i, (label, code) in enumerate(woven)]
        self.main_count = len(main)
        self.next_spawn_sid = len(main)

        if prog.boot_cells:
            boot_code = lex_strand(prog.boot_cells, prog.axis)
            boot = Strand(-1, "boot", boot_code)
            self._register(boot)
            self.run_scheduler()
            if boot.status == DEAD or self.failed:
                return 1
        for s in main:
            self._register(s)
        self.run_scheduler()
        return 1 if self.failed else 0


def run_text(text, stdin_text=""):
    """Run MLang source. Returns (exit_code, stdout, stderr)."""
    import io

    out, err = io.StringIO(), io.StringIO()
    vm = VM(stdin=io.StringIO(stdin_text), stdout=out, stderr=err)
    try:
        prog = parse_source(text)
        code = vm.run_program(prog)
    except LoadError as e:
        loc = f" at {e.pos[0]}:{e.pos[1]}" if e.pos else ""
        err.write(f"✗ weave error{loc}: {e.msg}\n")
        code = 2
    return code, out.getvalue(), err.getvalue()
