//! Distributed streams — MLang channels bridged over TCP.
//!
//! `mlang hub` runs a program whose **work channel** (α unless renamed)
//! is exported: values the program sends there go over the wire to
//! connected workers instead of the local queue, and its **results
//! channel** (β) is imported: values workers send back are injected into
//! it. `mlang worker` is the mirror image — work arrives on its imported
//! work channel, and sends to its exported results channel return to the
//! hub. A worker program is therefore just a pump: `[body]⇉αβ`.
//!
//! The end-of-stream convention carries over: the hub program ends its
//! pour with ∅ (⇈ does this automatically); the hub holds that ∅ until
//! every dispatched item has its result, then tells every worker the
//! stream is over (stopping their pumps) and forwards the ∅ onto its own
//! results channel (finishing its drain). On the wire, end-of-stream is
//! the control line `⇅ end`, *not* a `∅` value line — so a pump whose
//! legitimate result is ∅ sends that ∅ as an ordinary value and it is
//! acknowledged like any other. Item k's result is matched to item k
//! because a pump is one-in-one-out in order.
//!
//! Distribution is demand-driven: each worker holds at most CREDIT
//! unacknowledged items, the next item goes to the least-loaded worker,
//! and a worker that disconnects — network failure or a glitch in its
//! pump body — has its outstanding items requeued for the others.
//! Workers may join at any time, including mid-run. A worker whose
//! *machine* vanishes (no FIN, no RST) is caught by an application
//! heartbeat: the hub pings idle workers every PING_EVERY and drops one
//! that has said nothing for WORKER_SILENCE; a worker gives up on a hub
//! silent for HUB_SILENCE. An item charged with MAX_ATTEMPTS worker
//! failures is dropped with a diagnostic rather than requeued forever,
//! so a poison item cannot stall the run.
//!
//! What this trades away is stated in docs/distributed.md: cross-machine
//! interleaving is real timing, so results arrive in nondeterministic
//! order — programs that reduce order-independently (a sum, a max, a
//! sort) keep byte-identical output regardless.
//!
//! The wire is line-per-value UTF-8 (wire.rs) after a one-line hello on
//! each side; lines starting `⇅ ` are control lines. There is no
//! authentication or encryption: run it on a network you trust.

use crate::par::{self, Bus, ExportTap};
use crate::values::Value;
use crate::vm::CompiledProgram;
use crate::wire::{self, LineEnd};
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{BufReader, ErrorKind, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

/// Unacknowledged work items per worker: 2 keeps a worker busy while its
/// previous result travels, without hoarding items a new joiner could take.
const CREDIT: usize = 2;

/// Worker failures an item may be charged with before it is dropped.
const MAX_ATTEMPTS: u32 = 3;

/// The peer must complete the hello within this; a port scanner or a
/// wedged client must not hold a thread forever.
const HELLO_TIMEOUT: Duration = Duration::from_secs(10);
/// The hub pings a worker that has been silent this long.
const PING_EVERY: Duration = Duration::from_secs(5);
/// A worker silent this long (no result, no pong) is dropped.
const WORKER_SILENCE: Duration = Duration::from_secs(30);
/// A hub silent this long (no work, no ping, no end) is lost.
const HUB_SILENCE: Duration = Duration::from_secs(60);

const HELLO_HUB: &str = "⇓ mlang-hub 2";
const HELLO_WORKER: &str = "⇓ mlang-worker 2";

/// Control lines. A value line never starts with `⇅ ` (wire.rs would
/// refuse it), so the prefix alone tells them apart.
const CTL: &str = "⇅ ";
const END: &str = "⇅ end";
const PING: &str = "⇅ ping";
const PONG: &str = "⇅ pong";

/// Which glyphs are bridged. Defaults: work α, results β.
#[derive(Clone, Copy)]
pub struct NetOpts {
    pub work: char,
    pub results: char,
}

impl Default for NetOpts {
    fn default() -> Self {
        NetOpts { work: 'α', results: 'β' }
    }
}

/// A stderr diagnostic that cannot panic: `eprintln!` panics on a closed
/// stderr, and a panic inside a lock scope would poison the hub state.
/// Some diagnostics are written with the state lock held on purpose —
/// "worker N finished" must land before the notify that lets the main
/// thread exit, or it would never appear at all.
fn diag(msg: &str) {
    let _ = writeln!(std::io::stderr(), "{msg}");
}

fn fatal(msg: &str) -> ! {
    diag(&format!("✗ net: {msg}"));
    std::process::exit(1);
}

/// Spawn the write half of a connection: lines from an mpsc queue. Each
/// line goes out as one `write_all` of payload *and* newline — a process
/// dying between the two would otherwise hand the peer a line that is
/// truncated yet still parseable. (TcpStream is unbuffered, so nothing
/// waits for a flush; work items are chunky and latency wins.)
fn spawn_writer(stream: TcpStream) -> Sender<String> {
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        let mut stream = stream;
        for mut line in rx {
            line.push('\n');
            if stream.write_all(line.as_bytes()).is_err() {
                return; // peer gone; the read half reports it
            }
        }
    });
    tx
}

/// Split a socket into a buffered read half plus a writer thread.
fn split(stream: TcpStream) -> Option<(BufReader<TcpStream>, Sender<String>)> {
    let _ = stream.set_nodelay(true);
    let read_half = stream.try_clone().ok()?;
    Some((BufReader::new(read_half), spawn_writer(stream)))
}

/// One line off the wire, or the reason there is none.
fn next_line(r: &mut BufReader<TcpStream>) -> Result<String, LineEnd> {
    wire::read_line(r, wire::MAX_LINE)
}

fn is_timeout(e: &LineEnd) -> bool {
    matches!(e, LineEnd::Io(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut))
}

// ── the hub ────────────────────────────────────────────────────────────

/// A work item and how many workers have died holding it at the head of
/// their queue — the position a pump is working on.
struct Item {
    v: Value,
    failures: u32,
}

struct WorkerLink {
    tx: Sender<String>,
    /// Items dispatched and not yet answered, oldest first. A pump is
    /// one-in-one-out in order, so each arriving result pops the front.
    outstanding: VecDeque<Item>,
    /// Last time anything (result or pong) arrived from this worker.
    last_seen: Instant,
    /// Last ping sent, so an idle worker gets one per PING_EVERY.
    last_ping: Instant,
}

struct HubState {
    pending: VecDeque<Item>,
    workers: HashMap<u64, WorkerLink>,
    next_worker_id: u64,
    joined_ever: usize,
    /// The program has sent ∅ on the work channel: the stream is complete.
    eos: bool,
    /// End broadcast to workers and ∅ forwarded to the results channel.
    finished: bool,
}

struct Hub {
    state: Mutex<HubState>,
    cv: Condvar,
    bus: OnceLock<Arc<Bus>>,
    opts: NetOpts,
}

impl Hub {
    fn new(opts: NetOpts) -> Hub {
        Hub {
            state: Mutex::new(HubState {
                pending: VecDeque::new(),
                workers: HashMap::new(),
                next_worker_id: 1,
                joined_ever: 0,
                eos: false,
                finished: false,
            }),
            cv: Condvar::new(),
            bus: OnceLock::new(),
            opts,
        }
    }

    /// Take the state lock. A poisoned lock (a thread panicked inside a
    /// scope) is recovered rather than propagated: the state itself is
    /// always left consistent between statements.
    fn lock(&self) -> MutexGuard<'_, HubState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The export tap: the program sent a value on the work channel.
    /// One stream per run: work after the ∅ is not silently lost — it is
    /// refused with a diagnostic (docs/distributed.md says so).
    fn offer(&self, v: Value) {
        let mut st = self.lock();
        if st.eos {
            drop(st);
            diag("⇅ work sent after end-of-stream — ignored");
            return;
        }
        if matches!(v, Value::Nil) {
            st.eos = true;
        } else {
            st.pending.push_back(Item { v, failures: 0 });
            self.assign(&mut st);
        }
        self.maybe_finish(&mut st);
    }

    /// Hand pending items to the least-loaded workers with free credit.
    fn assign(&self, st: &mut HubState) {
        while !st.pending.is_empty() {
            let target = st
                .workers
                .iter()
                .filter(|(_, w)| w.outstanding.len() < CREDIT)
                .min_by_key(|(id, w)| (w.outstanding.len(), **id))
                .map(|(id, _)| *id);
            let Some(id) = target else { return };
            let item = st.pending.pop_front().unwrap();
            let line = match wire::render(&item.v) {
                Ok(l) => l,
                Err(e) => fatal(&format!("{e} (channel {})", self.opts.work)),
            };
            let w = st.workers.get_mut(&id).unwrap();
            if w.tx.send(line).is_ok() {
                w.outstanding.push_back(item);
            } else {
                // Writer already gone — requeue and drop the worker now;
                // its reader thread's removal will find it already done.
                st.pending.push_front(item);
                self.remove_worker(st, id, "lost");
            }
        }
    }

    /// Forget a worker, requeueing whatever it still owed. Idempotent —
    /// the reader thread and a failed assign can both get here.
    ///
    /// The failure is charged to the *head* item only: a pump works in
    /// order, so that is the item it was on when it died; the rest were
    /// merely queued. An item that has now killed MAX_ATTEMPTS workers
    /// is dropped, not requeued — otherwise a poison item would kill
    /// every worker in turn and the hub would wait forever.
    fn remove_worker(&self, st: &mut HubState, id: u64, how: &str) {
        let Some(w) = st.workers.remove(&id) else { return };
        let mut owed = w.outstanding;
        let mut dropped = None;
        if let Some(head) = owed.front_mut() {
            head.failures += 1;
            if head.failures >= MAX_ATTEMPTS {
                dropped = owed.pop_front();
            }
        }
        let n = owed.len();
        if n == 0 {
            diag(&format!("⇅ worker {id} {how}"));
        } else {
            diag(&format!(
                "⇅ worker {id} {how} — {n} item{} requeued",
                if n == 1 { "" } else { "s" }
            ));
        }
        if let Some(item) = dropped {
            let shown = wire::render(&item.v)
                .unwrap_or_else(|_| crate::values::fmt(&item.v, false));
            diag(&format!(
                "⇅ item dropped after {MAX_ATTEMPTS} worker failures: {shown}"
            ));
        }
        for item in owed.into_iter().rev() {
            st.pending.push_front(item);
        }
        self.cv.notify_all(); // the after-run linger waits on this
    }

    /// All work dispatched and every result home: end the stream — tell
    /// each worker (their pumps stop) and put the held ∅ onto the results
    /// channel (the program's drain finishes) — and let the import close.
    fn maybe_finish(&self, st: &mut HubState) {
        if st.finished
            || !st.eos
            || !st.pending.is_empty()
            || st.workers.values().any(|w| !w.outstanding.is_empty())
        {
            return;
        }
        st.finished = true;
        for w in st.workers.values() {
            let _ = w.tx.send(END.into());
        }
        let bus = self.bus.get().expect("bus attached before strands run");
        bus.send(self.opts.results, Value::Nil);
        bus.close_import(self.opts.results);
    }

    /// The heartbeat: ping every worker that has been quiet for a while.
    /// Silence past WORKER_SILENCE is caught by the reader's timeout.
    fn ping_idle(&self) {
        let mut st = self.lock();
        let now = Instant::now();
        for w in st.workers.values_mut() {
            if now.duration_since(w.last_seen) >= PING_EVERY
                && now.duration_since(w.last_ping) >= PING_EVERY
            {
                w.last_ping = now;
                let _ = w.tx.send(PING.into());
            }
        }
    }
}

/// One connected worker, from hello to hangup, on its own thread.
fn hub_serve_worker(hub: Arc<Hub>, stream: TcpStream) {
    let peer = stream
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "?".into());
    let _ = stream.set_read_timeout(Some(HELLO_TIMEOUT));
    let Some((mut reader, tx)) = split(stream) else { return };

    if tx.send(HELLO_HUB.into()).is_err() {
        return;
    }
    match next_line(&mut reader) {
        Ok(l) if l == HELLO_WORKER => {}
        _ => {
            diag(&format!("⇅ {peer} is not an mlang worker — dropped"));
            return;
        }
    }
    // From here on silence is the heartbeat's business.
    let _ = reader.get_ref().set_read_timeout(Some(WORKER_SILENCE));

    let id = {
        let mut st = hub.lock();
        let id = st.next_worker_id;
        st.next_worker_id += 1;
        st.joined_ever += 1;
        if st.finished {
            // The stream already ended — this joiner's only news is that.
            let _ = tx.send(END.into());
        }
        let now = Instant::now();
        st.workers.insert(
            id,
            WorkerLink { tx: tx.clone(), outstanding: VecDeque::new(), last_seen: now, last_ping: now },
        );
        // Logged before the notify: a test (or a script) that waits for
        // this line may rely on the worker already holding its items.
        diag(&format!("⇅ worker {id} joined ({peer})"));
        hub.assign(&mut st);
        hub.cv.notify_all();
        id
    };

    let how: String = loop {
        let line = match next_line(&mut reader) {
            Ok(l) => l,
            Err(LineEnd::Eof) => break "hung up".into(),
            Err(e) if is_timeout(&e) => {
                break format!("silent for {}s — dropped", WORKER_SILENCE.as_secs())
            }
            Err(e) => break format!("lost ({e})"),
        };
        if let Some(ctl) = line.strip_prefix(CTL) {
            match ctl {
                "pong" => {
                    if let Some(w) = hub.lock().workers.get_mut(&id) {
                        w.last_seen = Instant::now();
                    }
                }
                "ping" => {
                    let _ = tx.send(PONG.into());
                }
                _ => diag(&format!("⇅ worker {id} sent an unknown control line «{line}» — ignored")),
            }
            continue;
        }
        let v = match wire::parse(&line) {
            Ok(v) => v,
            Err(e) => break format!("sent a malformed value ({e}) — dropped"),
        };
        let mut st = hub.lock();
        // A result acknowledges the worker's oldest outstanding item.
        // Inject it only if there *is* one: after a requeue (or after
        // the end) an extra value would otherwise duplicate a result.
        let owed = st.workers.get_mut(&id).and_then(|w| {
            w.last_seen = Instant::now();
            w.outstanding.pop_front()
        });
        if owed.is_some() {
            hub.bus.get().expect("bus attached").send(hub.opts.results, v);
        } else {
            diag(&format!("⇅ worker {id} sent an unsolicited result — ignored"));
        }
        hub.assign(&mut st);
        hub.maybe_finish(&mut st);
    };

    let mut st = hub.lock();
    // A hangup after the end is the worker done with its stream; any
    // earlier one is a loss.
    let how = match how.as_str() {
        "hung up" if st.finished => "finished",
        "hung up" => "lost",
        other => other,
    };
    hub.remove_worker(&mut st, id, how);
    hub.assign(&mut st);
    hub.maybe_finish(&mut st);
}

/// `mlang hub` — serve a program across N workers.
pub fn run_hub(
    prog: &CompiledProgram,
    prog_args: Vec<String>,
    listen: &str,
    min_workers: usize,
    opts: NetOpts,
) -> i32 {
    let listener = match TcpListener::bind(listen) {
        Ok(l) => l,
        Err(e) => fatal(&format!("cannot listen on {listen}: {e}")),
    };
    let local = listener
        .local_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| listen.into());
    diag(&format!(
        "⇅ hub listening on {local} (work {} → workers → results {})",
        opts.work, opts.results
    ));

    let hub = Arc::new(Hub::new(opts));
    let tap_hub = hub.clone();
    let mut exports: HashMap<char, ExportTap> = HashMap::new();
    exports.insert(opts.work, Box::new(move |v| tap_hub.offer(v)));
    let imports: HashSet<char> = [opts.results].into();
    let bus = Arc::new(Bus::with_net(
        prog.strands.len(),
        prog_args,
        None,
        prog.source.clone(),
        par::channel_census(prog),
        exports,
        imports,
    ));
    hub.bus.set(bus.clone()).ok().expect("bus set once");

    let accept_hub = hub.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let hub = accept_hub.clone();
                    std::thread::spawn(move || hub_serve_worker(hub, stream));
                }
                // Out of descriptors, or a transient network fault: do
                // not spin on it.
                Err(_) => std::thread::sleep(Duration::from_millis(100)),
            }
        }
    });

    let beat_hub = hub.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(1));
        beat_hub.ping_idle();
    });

    if min_workers > 0 {
        let mut st = hub.lock();
        if st.joined_ever < min_workers {
            diag(&format!(
                "⇅ waiting for {min_workers} worker{}…",
                if min_workers == 1 { "" } else { "s" }
            ));
            while st.joined_ever < min_workers {
                st = hub.cv.wait(st).unwrap_or_else(|e| e.into_inner());
            }
        }
    }

    let code = par::run_with_bus(bus, prog);

    // A worker hangs up only after taking its end off the wire, so
    // linger until every one has — exiting sooner could tear the socket
    // down with that line still in a writer's queue. A worker that never
    // hangs up (wedged, or joined after the end) is abandoned after 2s.
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut st = hub.lock();
    while st.finished && !st.workers.is_empty() {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        st = hub
            .cv
            .wait_timeout(st, left)
            .unwrap_or_else(|e| e.into_inner())
            .0;
    }
    drop(st);
    code
}

// ── the worker ─────────────────────────────────────────────────────────

/// `mlang worker` — join a hub and lend it this machine.
pub fn run_worker(
    prog: &CompiledProgram,
    prog_args: Vec<String>,
    connect: &str,
    opts: NetOpts,
) -> i32 {
    let stream = match TcpStream::connect(connect) {
        Ok(s) => s,
        Err(e) => fatal(&format!("cannot reach a hub at {connect}: {e}")),
    };
    let _ = stream.set_read_timeout(Some(HELLO_TIMEOUT));
    let Some((mut reader, tx)) = split(stream) else {
        fatal("cannot split the hub connection");
    };
    if tx.send(HELLO_WORKER.into()).is_err() {
        fatal(&format!("connection to hub at {connect} lost"));
    }

    // Telling a ∅ *result* from the pump's end-of-stream forward: the
    // pump is one-in-one-out in order, so when it forwards the ∅ it has
    // already sent one result per item received — `sent == received`
    // with the end delivered. Any other ∅ reaching the tap is a value.
    let received = Arc::new(AtomicUsize::new(0));
    let sent = Arc::new(AtomicUsize::new(0));
    let end_received = Arc::new(AtomicBool::new(false));

    // Results leave through the export tap. The socket closing is the
    // hub's signal that this worker is done, so the forwarded ∅ stays
    // local.
    let result_tx = tx.clone();
    let connect_owned = connect.to_string();
    let (tap_received, tap_sent, tap_end) = (received.clone(), sent.clone(), end_received.clone());
    let mut exports: HashMap<char, ExportTap> = HashMap::new();
    exports.insert(
        opts.results,
        Box::new(move |v| {
            if matches!(v, Value::Nil)
                && tap_end.load(Ordering::SeqCst)
                && tap_sent.load(Ordering::SeqCst) == tap_received.load(Ordering::SeqCst)
            {
                return;
            }
            let line = match wire::render(&v) {
                Ok(l) => l,
                Err(e) => fatal(&format!("{e} (channel results)")),
            };
            tap_sent.fetch_add(1, Ordering::SeqCst);
            if result_tx.send(line).is_err() {
                fatal(&format!("connection to hub at {connect_owned} lost"));
            }
        }),
    );
    let imports: HashSet<char> = [opts.work].into();
    let bus = Arc::new(Bus::with_net(
        prog.strands.len(),
        prog_args,
        None,
        prog.source.clone(),
        par::channel_census(prog),
        exports,
        imports,
    ));

    let reader_bus = bus.clone();
    let work_chan = opts.work;
    let connect_owned = connect.to_string();
    std::thread::spawn(move || {
        match next_line(&mut reader) {
            Ok(l) if l == HELLO_HUB => {}
            Ok(_) | Err(LineEnd::Eof) => fatal(&format!("{connect_owned} is not an mlang hub")),
            Err(e) if is_timeout(&e) => {
                fatal(&format!("{connect_owned} did not answer the hello in time"))
            }
            Err(e) => fatal(&format!("connection to hub at {connect_owned} lost ({e})")),
        }
        // From here on the hub pings when idle, so silence means loss.
        let _ = reader.get_ref().set_read_timeout(Some(HUB_SILENCE));
        diag(&format!("⇅ joined hub at {connect_owned}"));
        loop {
            let line = match next_line(&mut reader) {
                Ok(l) => l,
                Err(e) if is_timeout(&e) => fatal(&format!(
                    "hub at {connect_owned} silent for {}s — giving up",
                    HUB_SILENCE.as_secs()
                )),
                Err(e) => fatal(&format!("connection to hub at {connect_owned} lost ({e})")),
            };
            if let Some(ctl) = line.strip_prefix(CTL) {
                match ctl {
                    "ping" => {
                        if tx.send(PONG.into()).is_err() {
                            fatal(&format!("connection to hub at {connect_owned} lost"));
                        }
                    }
                    "pong" => {}
                    "end" => {
                        // End of the hub's stream: deliver the ∅ and let
                        // the import close — waits on it are provable
                        // again. The flag goes up first so the tap can
                        // recognise the pump's forward.
                        end_received.store(true, Ordering::SeqCst);
                        reader_bus.send(work_chan, Value::Nil);
                        reader_bus.close_import(work_chan);
                        return;
                    }
                    _ => diag(&format!("⇅ hub sent an unknown control line «{line}» — ignored")),
                }
                continue;
            }
            match wire::parse(&line) {
                Ok(v) => {
                    received.fetch_add(1, Ordering::SeqCst);
                    reader_bus.send(work_chan, v);
                }
                Err(e) => fatal(&format!("malformed value from hub: {e}")),
            }
        }
    });

    par::run_with_bus(bus, prog)
}
