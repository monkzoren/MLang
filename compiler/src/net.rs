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
//! The end-of-stream convention carries over unchanged: the hub program
//! ends its pour with ∅ (⇈ does this automatically); the hub holds that
//! ∅ until every dispatched item has its result, then forwards it to all
//! workers (stopping their pumps) and onto its own results channel
//! (finishing its drain). Item k's result is matched to item k because a
//! pump is one-in-one-out in order.
//!
//! Distribution is demand-driven: each worker holds at most CREDIT
//! unacknowledged items, the next item goes to the least-loaded worker,
//! and a worker that disconnects — network failure or a glitch in its
//! pump body — has its outstanding items requeued for the others.
//! Workers may join at any time, including mid-run.
//!
//! What this trades away is stated in docs/distributed.md: cross-machine
//! interleaving is real timing, so results arrive in nondeterministic
//! order — programs that reduce order-independently (a sum, a max, a
//! sort) keep byte-identical output regardless.
//!
//! The wire is line-per-value UTF-8 (wire.rs) after a one-line hello on
//! each side. There is no authentication or encryption: run it on a
//! network you trust.

use crate::par::{self, Bus, ExportTap};
use crate::values::Value;
use crate::vm::CompiledProgram;
use crate::wire;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Condvar, Mutex, OnceLock};

/// Unacknowledged work items per worker: 2 keeps a worker busy while its
/// previous result travels, without hoarding items a new joiner could take.
const CREDIT: usize = 2;

const HELLO_HUB: &str = "⇓ mlang-hub 1";
const HELLO_WORKER: &str = "⇓ mlang-worker 1";

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

fn fatal(msg: &str) -> ! {
    eprintln!("✗ net: {msg}");
    std::process::exit(1);
}

/// Spawn the write half of a connection: lines from an mpsc queue, each
/// flushed immediately (work items are chunky; latency wins).
fn spawn_writer(stream: TcpStream) -> Sender<String> {
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        let mut stream = stream;
        for line in rx {
            if writeln!(stream, "{line}").and_then(|_| stream.flush()).is_err() {
                return; // peer gone; the read half reports it
            }
        }
    });
    tx
}

// ── the hub ────────────────────────────────────────────────────────────

struct WorkerLink {
    tx: Sender<String>,
    /// Items dispatched and not yet answered, oldest first. A pump is
    /// one-in-one-out in order, so each arriving result pops the front.
    outstanding: VecDeque<Value>,
}

struct HubState {
    pending: VecDeque<Value>,
    workers: HashMap<u64, WorkerLink>,
    next_worker_id: u64,
    joined_ever: usize,
    /// The program has sent ∅ on the work channel: the stream is complete.
    eos: bool,
    /// ∅ broadcast to workers and forwarded to the results channel.
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

    /// The export tap: the program sent a value on the work channel.
    fn offer(&self, v: Value) {
        let mut st = self.state.lock().unwrap();
        if matches!(v, Value::Nil) {
            st.eos = true;
        } else {
            st.pending.push_back(v);
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
            let v = st.pending.pop_front().unwrap();
            let line = match wire::render(&v) {
                Ok(l) => l,
                Err(e) => fatal(&format!("{e} (channel {})", self.opts.work)),
            };
            let w = st.workers.get_mut(&id).unwrap();
            if w.tx.send(line).is_ok() {
                w.outstanding.push_back(v);
            } else {
                // Writer already gone — requeue and drop the worker now;
                // its reader thread's removal will find it already done.
                st.pending.push_front(v);
                self.remove_worker(st, id, "lost");
            }
        }
    }

    /// Forget a worker, requeueing whatever it still owed. Idempotent —
    /// the reader thread and a failed assign can both get here.
    fn remove_worker(&self, st: &mut HubState, id: u64, how: &str) {
        let Some(w) = st.workers.remove(&id) else { return };
        if w.outstanding.is_empty() {
            eprintln!("⇅ worker {id} {how}");
        } else {
            let n = w.outstanding.len();
            eprintln!(
                "⇅ worker {id} {how} — {n} item{} requeued",
                if n == 1 { "" } else { "s" }
            );
            for v in w.outstanding.into_iter().rev() {
                st.pending.push_front(v);
            }
        }
        self.cv.notify_all(); // the after-run linger waits on this
    }

    /// All work dispatched and every result home: forward the held ∅ —
    /// to each worker (their pumps stop) and onto the results channel
    /// (the program's drain finishes) — and let the import close.
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
            let _ = w.tx.send("∅".into());
        }
        let bus = self.bus.get().expect("bus attached before strands run");
        bus.send(self.opts.results, Value::Nil);
        bus.close_import(self.opts.results);
    }
}

/// One connected worker, from hello to hangup, on its own thread.
fn hub_serve_worker(hub: Arc<Hub>, stream: TcpStream) {
    let _ = stream.set_nodelay(true);
    let peer = stream
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "?".into());
    let Ok(read_half) = stream.try_clone() else { return };
    let mut lines = BufReader::new(read_half).lines();

    let tx = spawn_writer(stream);
    if tx.send(HELLO_HUB.into()).is_err() {
        return;
    }
    match lines.next() {
        Some(Ok(l)) if l == HELLO_WORKER => {}
        _ => {
            eprintln!("⇅ {peer} is not an mlang worker — dropped");
            return;
        }
    }

    let id = {
        let mut st = hub.state.lock().unwrap();
        let id = st.next_worker_id;
        st.next_worker_id += 1;
        st.joined_ever += 1;
        if st.finished {
            // The stream already ended — this joiner's only work is its ∅.
            let _ = tx.send("∅".into());
        }
        st.workers.insert(id, WorkerLink { tx, outstanding: VecDeque::new() });
        eprintln!("⇅ worker {id} joined ({peer})");
        hub.assign(&mut st);
        hub.cv.notify_all();
        id
    };

    for line in lines {
        let Ok(line) = line else { break };
        let v = match wire::parse(&line) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("⇅ worker {id} sent a malformed value ({e}) — dropped");
                break;
            }
        };
        // Inject the result before the bookkeeping so the value is
        // visible the moment the last acknowledgment can finish the run.
        hub.bus.get().expect("bus attached").send(hub.opts.results, v);
        let mut st = hub.state.lock().unwrap();
        if let Some(w) = st.workers.get_mut(&id) {
            w.outstanding.pop_front();
        }
        hub.assign(&mut st);
        hub.maybe_finish(&mut st);
    }

    let mut st = hub.state.lock().unwrap();
    let done = st.finished;
    hub.remove_worker(&mut st, id, if done { "finished" } else { "lost" });
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
    eprintln!(
        "⇅ hub listening on {local} (work {} → workers → results {})",
        opts.work, opts.results
    );

    let hub = Arc::new(Hub::new(opts));
    let tap_hub = hub.clone();
    let mut exports: HashMap<char, ExportTap> = HashMap::new();
    exports.insert(opts.work, Box::new(move |v| tap_hub.offer(v)));
    let imports: HashSet<char> = [opts.results].into();
    let bus = Arc::new(Bus::with_net(prog.strands.len(), prog_args, exports, imports));
    hub.bus.set(bus.clone()).ok().expect("bus set once");

    let accept_hub = hub.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let hub = accept_hub.clone();
            std::thread::spawn(move || hub_serve_worker(hub, stream));
        }
    });

    if min_workers > 0 {
        let mut st = hub.state.lock().unwrap();
        if st.joined_ever < min_workers {
            eprintln!(
                "⇅ waiting for {min_workers} worker{}…",
                if min_workers == 1 { "" } else { "s" }
            );
            while st.joined_ever < min_workers {
                st = hub.cv.wait(st).unwrap();
            }
        }
    }

    let code = par::run_with_bus(bus, prog);

    // A worker hangs up only after taking its ∅ off the wire, so linger
    // until every one has — exiting sooner could tear the socket down
    // with that ∅ still in a writer's queue. A worker that never hangs
    // up (wedged, or joined after the end) is abandoned after 2s.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let mut st = hub.state.lock().unwrap();
    while st.finished && !st.workers.is_empty() {
        let left = deadline.saturating_duration_since(std::time::Instant::now());
        if left.is_zero() {
            break;
        }
        st = hub.cv.wait_timeout(st, left).unwrap().0;
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
    let _ = stream.set_nodelay(true);
    let Ok(read_half) = stream.try_clone() else {
        fatal("cannot split the hub connection");
    };
    let tx = spawn_writer(stream);
    if tx.send(HELLO_WORKER.into()).is_err() {
        fatal(&format!("connection to hub at {connect} lost"));
    }

    // Results leave through the export tap. The pump forwards the ∅ it
    // receives from the hub; the socket closing is the hub's signal that
    // this worker is done, so the ∅ itself stays local.
    let result_tx = tx.clone();
    let connect_owned = connect.to_string();
    let mut exports: HashMap<char, ExportTap> = HashMap::new();
    exports.insert(
        opts.results,
        Box::new(move |v| {
            if matches!(v, Value::Nil) {
                return;
            }
            let line = match wire::render(&v) {
                Ok(l) => l,
                Err(e) => fatal(&format!("{e} (channel results)")),
            };
            if result_tx.send(line).is_err() {
                fatal(&format!("connection to hub at {connect_owned} lost"));
            }
        }),
    );
    let imports: HashSet<char> = [opts.work].into();
    let bus = Arc::new(Bus::with_net(prog.strands.len(), prog_args, exports, imports));

    let reader_bus = bus.clone();
    let work_chan = opts.work;
    let connect_owned = connect.to_string();
    std::thread::spawn(move || {
        let mut lines = BufReader::new(read_half).lines();
        match lines.next() {
            Some(Ok(l)) if l == HELLO_HUB => {}
            _ => fatal(&format!("{connect_owned} is not an mlang hub")),
        }
        eprintln!("⇅ joined hub at {connect_owned}");
        for line in lines {
            let Ok(line) = line else {
                fatal(&format!("connection to hub at {connect_owned} lost"));
            };
            match wire::parse(&line) {
                Ok(Value::Nil) => {
                    // End of the hub's stream: deliver the ∅ and let the
                    // import close — waits on it are provable again.
                    reader_bus.send(work_chan, Value::Nil);
                    reader_bus.close_import(work_chan);
                    return;
                }
                Ok(v) => reader_bus.send(work_chan, v),
                Err(e) => fatal(&format!("malformed value from hub: {e}")),
            }
        }
        fatal(&format!("connection to hub at {connect_owned} lost"));
    });

    par::run_with_bus(bus, prog)
}
