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

/// A network tap on an exported channel: `mlang hub`/`mlang worker`
/// (net.rs) register one per bridged channel, and sends to that channel
/// go to the tap — that is, onto the wire — instead of the local queue.
pub(crate) type ExportTap = Box<dyn Fn(Value) + Send + Sync>;

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
    /// Channels fed by the network (net.rs) whose stream has not ended.
    /// A wait on one is always satisfiable — the wire may yet deliver —
    /// so the deadlock verdict stays sound with a remote peer attached.
    open_imports: HashSet<char>,
}

/// Could this wait complete right now? A notified-but-not-yet-rescheduled
/// thread still sits in `waiting`, so the deadlock verdict must check the
/// condition itself, not the parked set alone — same reasoning as the
/// sequential engine's try-unblock-then-check.
fn wait_satisfiable(st: &State, w: &WaitOn) -> bool {
    match w {
        WaitOn::Chan(c) => {
            st.open_imports.contains(c)
                || st.chans.get(c).map(|q| !q.is_empty()).unwrap_or(false)
        }
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
    /// Channels bridged outward by net.rs: a send goes to the tap, not
    /// the local queue. Empty except under `mlang hub` / `mlang worker`.
    exports: HashMap<char, ExportTap>,
    /// Replay-mode web state (⎆/⍅ without a live listener): the request
    /// counter and the ids still awaiting a response, shared by all strands.
    replay_web: Mutex<(i64, HashSet<i64>)>,
    /// Live web mode: the listener, shared by every strand's VM.
    pub http: Option<std::sync::Arc<crate::http::HttpBridge>>,
    /// The program's source lines and channel census, so parallel-mode
    /// fault reports carry the same excerpts, call chains, and orphaned-
    /// channel warnings as the sequential engine.
    source: Vec<String>,
    chan_sites: HashMap<char, (usize, usize)>,
}

/// Take a lock, recovering from poisoning. A strand thread that panics
/// (contained in `drive`) may have held one of these; the state itself is
/// consistent between statements, so continuing is right — the panic is
/// reported and treated as that strand's death, not as the run's.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn wait<'a, T>(cv: &Condvar, g: MutexGuard<'a, T>) -> MutexGuard<'a, T> {
    cv.wait(g).unwrap_or_else(|e| e.into_inner())
}

impl Bus {
    fn new(
        main_count: usize,
        args: Vec<String>,
        http: Option<std::sync::Arc<crate::http::HttpBridge>>,
        source: Vec<String>,
        chan_sites: HashMap<char, (usize, usize)>,
    ) -> Bus {
        Bus::with_net(
            main_count,
            args,
            http,
            source,
            chan_sites,
            HashMap::new(),
            HashSet::new(),
        )
    }

    /// A Bus with network bridging: sends to an exported channel go to
    /// its tap, and imported channels stay deadlock-exempt until the
    /// wire delivers their ∅ (close_import).
    pub(crate) fn with_net(
        main_count: usize,
        args: Vec<String>,
        http: Option<std::sync::Arc<crate::http::HttpBridge>>,
        source: Vec<String>,
        chan_sites: HashMap<char, (usize, usize)>,
        exports: HashMap<char, ExportTap>,
        imports: HashSet<char>,
    ) -> Bus {
        Bus {
            state: Mutex::new(State {
                chans: HashMap::new(),
                globals: HashMap::new(),
                live: 0,
                waiting: HashMap::new(),
                done: HashSet::new(),
                next_spawn_sid: main_count as i64,
                open_imports: imports,
            }),
            cv: Condvar::new(),
            stdout: Mutex::new(std::io::stdout()),
            stderr: Mutex::new(std::io::stderr()),
            stdin: Mutex::new(std::io::BufReader::new(std::io::stdin())),
            fail: AtomicBool::new(false),
            main_count,
            source,
            chan_sites,
            args,
            exports,
            replay_web: Mutex::new((1, HashSet::new())),
            http,
        }
    }

    // ── the web bridge (replay mode) ───────────────────────────────────

    /// Read one ▷ request frame from the shared stdin, holding its lock
    /// for the whole frame so concurrent accepts cannot interleave bytes.
    pub fn read_request(&self) -> Result<Option<crate::http::Request>, String> {
        let mut stdin = lock(&self.stdin);
        { let _ = lock(&self.stdout).flush(); }
        let mut next = move || {
            let buf = stdin.fill_buf().ok()?;
            if buf.is_empty() {
                return None;
            }
            let b = buf[0];
            stdin.consume(1);
            Some(b)
        };
        match crate::http::read_framed(&mut next)? {
            None => Ok(None),
            Some((method, path, body)) => {
                let mut web = lock(&self.replay_web);
                let id = web.0;
                web.0 += 1;
                web.1.insert(id);
                Ok(Some((id, method, path, body)))
            }
        }
    }

    /// Retire a replay request id; false when it was never open.
    pub fn close_request(&self, id: i64) -> bool {
        lock(&self.replay_web).1.remove(&id)
    }

    // ── channels ───────────────────────────────────────────────────────

    pub fn send(&self, c: char, v: Value) {
        // Exported channels leave the process here. The tap runs without
        // the state lock held: taps take their own locks (net.rs) and may
        // re-enter send() for a different channel.
        if let Some(tap) = self.exports.get(&c) {
            tap(v);
            return;
        }
        let mut st = lock(&self.state);
        st.chans.entry(c).or_default().push_back(v);
        self.cv.notify_all();
    }

    /// The network's end of an imported channel has closed (its ∅ is
    /// delivered): waits on it are no longer exempt from the deadlock
    /// verdict, and the verdict is re-checked in case every remaining
    /// strand was already parked.
    pub(crate) fn close_import(&self, c: char) {
        let mut st = lock(&self.state);
        st.open_imports.remove(&c);
        self.maybe_deadlock(&st);
    }

    pub fn try_recv(&self, c: char) -> Option<Value> {
        lock(&self.state).chans.entry(c).or_default().pop_front()
    }

    /// Blocking receive: parks the thread until a value arrives. If parking
    /// would leave every live strand parked, that is the program's deadlock —
    /// report it and end the run, exactly as the sequential engine would.
    pub fn recv(&self, c: char, sid: i64, label: &str, pos: Pos) -> Value {
        let mut st = lock(&self.state);
        loop {
            if let Some(v) = st.chans.entry(c).or_default().pop_front() {
                return v;
            }
            st.waiting
                .insert(sid, (label.to_string(), WaitOn::Chan(c), pos));
            self.maybe_deadlock(&st);
            st = wait(&self.cv, st);
            st.waiting.remove(&sid);
        }
    }

    // ── globals (single-assignment) ────────────────────────────────────

    pub fn global_get(&self, c: char) -> Option<Value> {
        lock(&self.state).globals.get(&c).cloned()
    }

    /// False if the sigil was already defined (the ≔ rebind glitch).
    pub fn global_define(&self, c: char, v: Value) -> bool {
        let mut st = lock(&self.state);
        if st.globals.contains_key(&c) {
            return false;
        }
        st.globals.insert(c, v);
        true
    }

    // ── strands ────────────────────────────────────────────────────────

    pub fn knows_strand(&self, sid: i64) -> bool {
        sid == -1 || (sid >= 0 && sid < lock(&self.state).next_spawn_sid)
    }

    /// Park until strand `sid` has finished (normally or by glitch).
    pub fn join_wait(&self, sid: i64, my_sid: i64, label: &str, pos: Pos) {
        let mut st = lock(&self.state);
        loop {
            if sid == -1 || st.done.contains(&sid) {
                return;
            }
            st.waiting
                .insert(my_sid, (label.to_string(), WaitOn::Strand(sid), pos));
            self.maybe_deadlock(&st);
            st = wait(&self.cv, st);
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
            let mut st = lock(&self.state);
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
        let mut st = lock(&self.state);
        st.live -= 1;
        st.done.insert(sid);
        self.cv.notify_all();
    }

    fn add_live(&self, n: usize) {
        lock(&self.state).live += n;
    }

    fn wait_quiescent(&self) {
        let mut st = lock(&self.state);
        while st.live > 0 {
            st = wait(&self.cv, st);
        }
    }

    // ── i/o ────────────────────────────────────────────────────────────

    pub fn read_line(&self, line: &mut String) -> usize {
        let mut stdin = lock(&self.stdin);
        { let _ = lock(&self.stdout).flush(); }
        stdin.read_line(line).unwrap_or(0)
    }

    /// One byte for the ⌥ event parser; None at end of input.
    pub fn read_byte(&self) -> Option<u8> {
        let mut stdin = lock(&self.stdin);
        { let _ = lock(&self.stdout).flush(); }
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
            let _ = lock(&self.stderr).write_all(bytes);
        } else {
            let _ = lock(&self.stdout).write_all(bytes);
        }
    }

    fn flush_streams(&self) {
        let _ = lock(&self.stdout).flush();
        let _ = lock(&self.stderr).flush();
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
            if let Some(x) = crate::vm::excerpt(&self.source, *pos) {
                report.push_str(&x);
                report.push('\n');
            }
        }
        let waited: Vec<char> = st
            .waiting
            .values()
            .filter_map(|(_, what, _)| match what {
                WaitOn::Chan(c) => Some(*c),
                _ => None,
            })
            .collect();
        report.push_str(&crate::vm::channel_census(&self.chan_sites, &waited));
        let _ = lock(&self.stdout).flush();
        {
            let mut err = lock(&self.stderr);
            let _ = err.write_all(report.as_bytes());
            let _ = err.flush();
        }
        std::process::exit(1);
    }
}

/// A per-thread stdout/stderr proxy: buffers locally and hands whole lines
/// to the shared stream, so parallel strands never interleave mid-line.
/// A partial line is held however long it grows — flushing it early would
/// break "output is atomic per line" (SPEC §4.2) exactly for the long
/// lines where interleaving is most visible. Only an explicit flush (⌨
/// prompts, end of strand) emits an unterminated tail.
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
///
/// A *panic* on the thread (an interpreter bug, not a program fault) is
/// contained here: without that the strand would silently vanish with
/// `live` never decremented, `wait_quiescent` would hang, and deadlock
/// detection would be disabled for the rest of the run. It is reported
/// and treated as the strand's death, through the same finish path.
fn drive(bus: Arc<Bus>, sid: i64, label: String, code: Arc<Vec<Instr>>, locals: Vec<(char, Value)>) {
    let shown = format!("{} ({})", fmt_i64(sid), label);
    let body = std::panic::AssertUnwindSafe(|| {
        let mut stdin = std::io::empty();
        let mut out = SharedWriter { bus: bus.clone(), err: false, buf: Vec::new() };
        let mut err = SharedWriter { bus: bus.clone(), err: true, buf: Vec::new() };
        {
            let mut vm = VM::new(&mut stdin, &mut out, &mut err);
            vm.bus = Some(bus.clone());
            vm.main_count = bus.main_count;
            vm.args = bus.args.clone();
            vm.http = bus.http.clone();
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
                // The whole report is composed first and written once, so
                // its lines never interleave with another strand's output.
                let (v, pos) = s.glitch.take().unwrap();
                let mut report = format!(
                    "✗ glitch in strand {} ({}) at {}: {}\n",
                    fmt_i64(s.sid),
                    s.label,
                    coords(pos),
                    fmt(&v, false)
                );
                report.push_str(&crate::vm::fault_detail(
                    &bus.source, pos, &s.glitch_chain, s.stack_view()));
                let _ = vm.err.flush();
                let _ = vm.err.write_all(report.as_bytes());
                bus.set_failed();
            }
        }
        let _ = out.flush();
        let _ = err.flush();
    });
    if let Err(payload) = std::panic::catch_unwind(body) {
        let what = payload
            .downcast_ref::<String>()
            .map(|s| s.as_str())
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("");
        let line = if what.is_empty() {
            format!("✗ strand {shown} panicked\n")
        } else {
            format!("✗ strand {shown} panicked: {what}\n")
        };
        bus.write_stream(true, line.as_bytes());
        bus.set_failed();
    }
    bus.finish(sid);
}

/// Run a compiled program with one OS thread per strand. Boot (with the
/// standard library woven in) runs first and must fully finish — including
/// anything it spawned — before the main strands start, same as the
/// sequential engine.
pub fn run_parallel(
    prog: &CompiledProgram,
    args: Vec<String>,
    http: Option<Arc<crate::http::HttpBridge>>,
) -> i32 {
    let bus = Arc::new(Bus::new(
        prog.strands.len(),
        args,
        http,
        prog.source.clone(),
        channel_census(prog),
    ));
    run_with_bus(bus, prog)
}

/// The channel census (send/receive site counts per glyph) for a whole
/// program, as parallel-mode fault reports want it.
pub(crate) fn channel_census(prog: &CompiledProgram) -> HashMap<char, (usize, usize)> {
    let mut chan_sites = HashMap::new();
    crate::vm::channel_sites(&prog.boot, &mut chan_sites);
    for (_, code) in &prog.strands {
        crate::vm::channel_sites(code, &mut chan_sites);
    }
    chan_sites
}

/// The strand-startup sequence on an already-configured Bus — net.rs
/// builds a Bus with taps and imports, attaches its threads, then runs
/// the program through here exactly as run_parallel would.
pub(crate) fn run_with_bus(bus: Arc<Bus>, prog: &CompiledProgram) -> i32 {
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
