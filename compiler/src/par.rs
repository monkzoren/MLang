//! The parallel scheduler — MLang's strands on OS threads.
//!
//! The language's default engine (vm.rs) is deterministic: strands step
//! round-robin on one thread and identical input produces identical bytes,
//! which the conformance corpus pins. This module is the opt-in alternative
//! (`mlang run --parallel`, or MLANG_PAR=1 for welded binaries): every
//! strand runs on its own OS thread, sharing only what the language itself
//! shares — channels and single-assignment globals — through the Bus.
//!
//! The contract in parallel mode:
//!   * each strand's own execution order is unchanged;
//!   * channels stay FIFO per sender, and a receive still blocks (parking
//!     the thread) until a value arrives;
//!   * output is atomic per line, but the interleaving of output from
//!     *different* strands — like the order ⇂ observes, ⚡ id assignment
//!     across racing spawners, and glitch-report ordering — follows real
//!     thread timing and is not reproducible run to run.
//! Programs whose channels have one sender and one receiver and that print
//! from a single strand (the Mandelbrot explorer, the ⇈/⇉/⇟ pipelines)
//! produce byte-identical output in both modes; the deterministic engine
//! remains the language's semantic ground truth.

use crate::values::{fmt, fmt_i64, Instr, Pos, Value};
use crate::vm::{run_burst, CompiledProgram, Status, Strand, VM};
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

fn coords(pos: Pos) -> String {
    if pos == (0, 0) {
        "?".into()
    } else {
        format!("{}:{}", pos.0, pos.1)
    }
}

enum WaitOn {
    Chan(char),
    Strand(i64),
}

struct State {
    chans: HashMap<char, VecDeque<Value>>,
    globals: HashMap<char, Value>,
    /// Strand threads that have not finished (running or parked).
    live: usize,
    /// Parked strands: sid → (label, what it awaits, where).
    waiting: HashMap<i64, (String, WaitOn, Pos)>,
    /// Finished strand ids, for ⋈.
    done: HashSet<i64>,
    next_spawn_sid: i64,
}

/// Could this wait complete right now? A notified-but-not-yet-rescheduled
/// thread still sits in `waiting`, so the deadlock verdict must check the
/// condition itself, not the parked set alone — same reasoning as the
/// sequential engine's try-unblock-then-check.
fn wait_satisfiable(st: &State, w: &WaitOn) -> bool {
    match w {
        WaitOn::Chan(c) => st.chans.get(c).map(|q| !q.is_empty()).unwrap_or(false),
        WaitOn::Strand(id) => *id == -1 || st.done.contains(id),
    }
}

pub struct Bus {
    state: Mutex<State>,
    cv: Condvar,
    stdout: Mutex<std::io::Stdout>,
    stderr: Mutex<std::io::Stderr>,
    stdin: Mutex<std::io::BufReader<std::io::Stdin>>,
    fail: AtomicBool,
    main_count: usize,
    args: Vec<String>,
}

impl Bus {
    fn new(main_count: usize, args: Vec<String>) -> Bus {
        Bus {
            state: Mutex::new(State {
                chans: HashMap::new(),
                globals: HashMap::new(),
                live: 0,
                waiting: HashMap::new(),
                done: HashSet::new(),
                next_spawn_sid: main_count as i64,
            }),
            cv: Condvar::new(),
            stdout: Mutex::new(std::io::stdout()),
            stderr: Mutex::new(std::io::stderr()),
            stdin: Mutex::new(std::io::BufReader::new(std::io::stdin())),
            fail: AtomicBool::new(false),
            main_count,
            args,
        }
    }

    // ── channels ───────────────────────────────────────────────────────

    pub fn send(&self, c: char, v: Value) {
        let mut st = self.state.lock().unwrap();
        st.chans.entry(c).or_default().push_back(v);
        self.cv.notify_all();
    }

    pub fn try_recv(&self, c: char) -> Option<Value> {
        self.state.lock().unwrap().chans.entry(c).or_default().pop_front()
    }

    /// Blocking receive: parks the thread until a value arrives. If parking
    /// would leave every live strand parked, that is the program's deadlock —
    /// report it and end the run, exactly as the sequential engine would.
    pub fn recv(&self, c: char, sid: i64, label: &str, pos: Pos) -> Value {
        let mut st = self.state.lock().unwrap();
        loop {
            if let Some(v) = st.chans.entry(c).or_default().pop_front() {
                return v;
            }
            st.waiting
                .insert(sid, (label.to_string(), WaitOn::Chan(c), pos));
            self.maybe_deadlock(&st);
            st = self.cv.wait(st).unwrap();
            st.waiting.remove(&sid);
        }
    }

    // ── globals (single-assignment) ────────────────────────────────────

    pub fn global_get(&self, c: char) -> Option<Value> {
        self.state.lock().unwrap().globals.get(&c).cloned()
    }

    /// False if the sigil was already defined (the ≔ rebind glitch).
    pub fn global_define(&self, c: char, v: Value) -> bool {
        let mut st = self.state.lock().unwrap();
        if st.globals.contains_key(&c) {
            return false;
        }
        st.globals.insert(c, v);
        true
    }

    // ── strands ────────────────────────────────────────────────────────

    pub fn knows_strand(&self, sid: i64) -> bool {
        sid == -1 || (sid >= 0 && sid < self.state.lock().unwrap().next_spawn_sid)
    }

    /// Park until strand `sid` has finished (normally or by glitch).
    pub fn join_wait(&self, sid: i64, my_sid: i64, label: &str, pos: Pos) {
        let mut st = self.state.lock().unwrap();
        loop {
            if sid == -1 || st.done.contains(&sid) {
                return;
            }
            st.waiting
                .insert(my_sid, (label.to_string(), WaitOn::Strand(sid), pos));
            self.maybe_deadlock(&st);
            st = self.cv.wait(st).unwrap();
            st.waiting.remove(&my_sid);
        }
    }

    /// ⚡ — start a quotation as a new strand on its own thread.
    pub fn spawn(
        self: Arc<Self>,
        label: String,
        code: Arc<Vec<Instr>>,
        locals: Vec<(char, Value)>,
    ) -> i64 {
        let sid = {
            let mut st = self.state.lock().unwrap();
            let sid = st.next_spawn_sid;
            st.next_spawn_sid += 1;
            st.live += 1;
            sid
        };
        let bus = self.clone();
        std::thread::spawn(move || drive(bus, sid, label, code, locals));
        sid
    }

    fn finish(&self, sid: i64) {
        let mut st = self.state.lock().unwrap();
        st.live -= 1;
        st.done.insert(sid);
        self.cv.notify_all();
    }

    fn add_live(&self, n: usize) {
        self.state.lock().unwrap().live += n;
    }

    fn wait_quiescent(&self) {
        let mut st = self.state.lock().unwrap();
        while st.live > 0 {
            st = self.cv.wait(st).unwrap();
        }
    }

    // ── i/o ────────────────────────────────────────────────────────────

    pub fn read_line(&self, line: &mut String) -> usize {
        let mut stdin = self.stdin.lock().unwrap();
        { let _ = self.stdout.lock().unwrap().flush(); }
        stdin.read_line(line).unwrap_or(0)
    }

    /// One byte for the ⌥ event parser; None at end of input.
    pub fn read_byte(&self) -> Option<u8> {
        let mut stdin = self.stdin.lock().unwrap();
        { let _ = self.stdout.lock().unwrap().flush(); }
        let buf = stdin.fill_buf().ok()?;
        if buf.is_empty() {
            return None;
        }
        let b = buf[0];
        stdin.consume(1);
        Some(b)
    }

    fn write_stream(&self, err: bool, bytes: &[u8]) {
        if err {
            let _ = self.stderr.lock().unwrap().write_all(bytes);
        } else {
            let _ = self.stdout.lock().unwrap().write_all(bytes);
        }
    }

    fn flush_streams(&self) {
        let _ = self.stdout.lock().unwrap().flush();
        let _ = self.stderr.lock().unwrap().flush();
    }

    // ── failure ────────────────────────────────────────────────────────

    fn set_failed(&self) {
        self.fail.store(true, Ordering::Relaxed);
    }

    fn failed(&self) -> bool {
        self.fail.load(Ordering::Relaxed)
    }

    /// Fire the deadlock report only when every live strand is parked AND
    /// none of their waits can complete — a parked entry whose channel has
    /// a value (or whose joinee is done) is a thread mid-wakeup, not stuck.
    fn maybe_deadlock(&self, st: &MutexGuard<State>) {
        if st.live == 0 || st.waiting.len() < st.live {
            return;
        }
        if st.waiting.values().any(|(_, w, _)| wait_satisfiable(st, w)) {
            return;
        }
        self.report_deadlock(st);
    }

    /// Every live strand is provably stuck: report the wait graph like the
    /// sequential engine and end the run. Never returns.
    fn report_deadlock(&self, st: &MutexGuard<State>) -> ! {
        let mut report = String::from("✗ deadlock — every remaining strand is blocked:\n");
        let mut sids: Vec<i64> = st.waiting.keys().copied().collect();
        sids.sort();
        for sid in sids {
            let (label, what, pos) = &st.waiting[&sid];
            let what = match what {
                WaitOn::Chan(c) => format!("channel {c}"),
                WaitOn::Strand(id) => format!("strand {}", fmt_i64(*id)),
            };
            report.push_str(&format!(
                "  strand {} ({}) waiting on {} at {}\n",
                fmt_i64(sid),
                label,
                what,
                coords(*pos)
            ));
        }
        let _ = self.stdout.lock().unwrap().flush();
        {
            let mut err = self.stderr.lock().unwrap();
            let _ = err.write_all(report.as_bytes());
            let _ = err.flush();
        }
        std::process::exit(1);
    }
}

/// A per-thread stdout/stderr proxy: buffers locally and hands whole lines
/// to the shared stream, so parallel strands never interleave mid-line.
struct SharedWriter {
    bus: Arc<Bus>,
    err: bool,
    buf: Vec<u8>,
}

impl Write for SharedWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.buf.extend_from_slice(bytes);
        if let Some(i) = self.buf.iter().rposition(|&b| b == b'\n') {
            let chunk: Vec<u8> = self.buf.drain(..=i).collect();
            self.bus.write_stream(self.err, &chunk);
        } else if self.buf.len() > 8192 {
            let chunk = std::mem::take(&mut self.buf);
            self.bus.write_stream(self.err, &chunk);
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if !self.buf.is_empty() {
            let chunk = std::mem::take(&mut self.buf);
            self.bus.write_stream(self.err, &chunk);
        }
        Ok(())
    }
}

impl Drop for SharedWriter {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

/// Run one strand to completion on the current thread. Blocking operations
/// park inside the Bus, so Sig::Block never surfaces here; a burst ends
/// only on completion, an uncaught glitch, or ⌛ (which becomes a real
/// thread yield).
fn drive(bus: Arc<Bus>, sid: i64, label: String, code: Arc<Vec<Instr>>, locals: Vec<(char, Value)>) {
    let mut stdin = std::io::empty();
    let mut out = SharedWriter { bus: bus.clone(), err: false, buf: Vec::new() };
    let mut err = SharedWriter { bus: bus.clone(), err: true, buf: Vec::new() };
    {
        let mut vm = VM::new(&mut stdin, &mut out, &mut err);
        vm.bus = Some(bus.clone());
        vm.main_count = bus.main_count;
        vm.args = bus.args.clone();
        let mut s = Strand::new(sid, label, code, locals);
        loop {
            run_burst(&mut vm, &mut s, usize::MAX);
            match s.status {
                Status::Done | Status::Dead => break,
                Status::Run => std::thread::yield_now(), // ⌛
                Status::Blocked => unreachable!("blocking op surfaced in parallel mode"),
            }
        }
        if s.status == Status::Dead {
            let (v, pos) = s.glitch.take().unwrap();
            let _ = writeln!(
                vm.err,
                "✗ glitch in strand {} ({}) at {}: {}",
                fmt_i64(s.sid),
                s.label,
                coords(pos),
                fmt(&v, false)
            );
            bus.set_failed();
        }
    }
    let _ = out.flush();
    let _ = err.flush();
    bus.finish(sid);
}

/// Run a compiled program with one OS thread per strand. Boot (with the
/// standard library woven in) runs first and must fully finish — including
/// anything it spawned — before the main strands start, same as the
/// sequential engine.
pub fn run_parallel(prog: &CompiledProgram, args: Vec<String>) -> i32 {
    let bus = Arc::new(Bus::new(prog.strands.len(), args));
    bus.add_live(1);
    drive(
        bus.clone(),
        -1,
        "boot".into(),
        Arc::new(prog.boot.clone()),
        Vec::new(),
    );
    bus.wait_quiescent();
    if bus.failed() {
        bus.flush_streams();
        return 1;
    }
    bus.add_live(prog.strands.len());
    for (i, (label, code)) in prog.strands.iter().enumerate() {
        let bus = bus.clone();
        let label = label.clone();
        let code = Arc::new(code.clone());
        std::thread::spawn(move || drive(bus, i as i64, label, code, Vec::new()));
    }
    bus.wait_quiescent();
    bus.flush_streams();
    if bus.failed() {
        1
    } else {
        0
    }
}
