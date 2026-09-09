//! The distributed runtime: a hub and its workers as real processes.
//!
//! The prime-finder example reduces order-independently (a sum and a
//! max), so however results interleave on the wire, the hub's stdout is
//! byte-exact — including when a worker dies mid-run and its items are
//! requeued on the survivors.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

fn mlang() -> &'static str {
    env!("CARGO_BIN_EXE_mlang")
}

fn example(name: &str) -> String {
    format!("{}/../examples/{name}", env!("CARGO_MANIFEST_DIR"))
}

/// A scratch program file unique to this test *and* this process, so
/// parallel test runs (and stale files from earlier ones) cannot collide.
fn scratch(test: &str, name: &str, source: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mlang-net-test-{}-{test}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, source).unwrap();
    path
}

struct HubHandle {
    child: Child,
    addr: String,
    stderr_lines: std::io::Lines<BufReader<std::process::ChildStderr>>,
}

/// Start a hub on an OS-assigned port and read its stderr up to the
/// listening line, so the workers know where to go.
fn start_hub(args: &[&str]) -> HubHandle {
    let mut child = Command::new(mlang())
        .arg("hub")
        .args(["--listen", "127.0.0.1:0"])
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stderr = child.stderr.take().unwrap();
    let mut stderr_lines = BufReader::new(stderr).lines();
    let addr = loop {
        let line = stderr_lines
            .next()
            .expect("hub stderr ended before the listening line")
            .unwrap();
        if let Some(rest) = line.strip_prefix("⇅ hub listening on ") {
            break rest.split_whitespace().next().unwrap().to_string();
        }
    };
    HubHandle { child, addr, stderr_lines }
}

impl HubHandle {
    /// Read hub stderr until a line containing `needle` goes by.
    fn await_line(&mut self, needle: &str) -> String {
        loop {
            let line = self
                .stderr_lines
                .next()
                .unwrap_or_else(|| panic!("hub stderr ended before {needle:?}"))
                .unwrap();
            if line.contains(needle) {
                return line;
            }
        }
    }

    /// Wait for the hub to exit; returns (exit code, stdout, remaining stderr).
    fn finish(self) -> (Option<i32>, String, String) {
        let mut rest = String::new();
        let drain = std::thread::spawn(move || {
            let mut s = String::new();
            for line in self.stderr_lines {
                let Ok(line) = line else { break };
                s.push_str(&line);
                s.push('\n');
            }
            s
        });
        let out = self.child.wait_with_output().unwrap();
        rest.push_str(&drain.join().unwrap());
        (
            out.status.code(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            rest,
        )
    }
}

fn start_worker(addr: &str, file: &str) -> Child {
    Command::new(mlang())
        .args(["worker", "--connect", addr, file])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

#[test]
fn two_workers_compute_the_primes_byte_exact() {
    let hub_prog = example("net-primes-hub.ml");
    let worker_prog = example("net-primes-worker.ml");
    let hub = start_hub(&["--workers", "2", &hub_prog, "5000", "250"]);
    let addr = hub.addr.clone();
    let mut workers: Vec<Child> = (0..2).map(|_| start_worker(&addr, &worker_prog)).collect();

    let (code, stdout, stderr) = hub.finish();
    assert_eq!(code, Some(0), "hub failed; stderr: {stderr}");
    assert_eq!(stdout, "π(<5000) = 669\nlargest: 4999\n");
    for w in &mut workers {
        assert_eq!(w.wait().unwrap().code(), Some(0));
    }
}

#[test]
fn a_glitching_worker_is_requeued_onto_the_survivor() {
    // Worker 1's pump body glitches on its first item and the worker
    // dies without forwarding ∅ — let-it-crash, over a socket. The hub
    // must requeue its unanswered items on worker 2 and still produce
    // the byte-exact result.
    let poison = scratch("glitch", "poison-worker.ml", "[«boom»↯]⇉αβ\n");

    let hub_prog = example("net-primes-hub.ml");
    let worker_prog = example("net-primes-worker.ml");
    let mut hub = start_hub(&["--workers", "2", &hub_prog, "5000", "500"]);
    let addr = hub.addr.clone();

    // Join the poison worker first so it is deterministically worker 1.
    let mut w1 = start_worker(&addr, poison.to_str().unwrap());
    hub.await_line("worker 1 joined");
    let mut w2 = start_worker(&addr, &worker_prog);

    let (code, stdout, stderr) = hub.finish();
    assert_eq!(code, Some(0), "hub failed; stderr: {stderr}");
    assert_eq!(stdout, "π(<5000) = 669\nlargest: 4999\n");
    assert!(
        stderr.contains("requeued"),
        "expected a requeue in hub stderr: {stderr}"
    );
    assert_eq!(w1.wait().unwrap().code(), Some(1), "the glitch is exit 1");
    assert_eq!(w2.wait().unwrap().code(), Some(0));
}

#[test]
fn an_empty_stream_ends_every_worker_cleanly() {
    // Zero work items: the ∅ is the whole stream. The hub must forward
    // it to the joined worker (whose pump stops at once) and to its own
    // drain, and both processes must exit 0.
    let empty_hub = scratch("empty", "empty-hub.ml", "⟨⟩⇈α\n⇟β#⍞\n");
    let worker_prog = example("net-primes-worker.ml");

    let mut hub = start_hub(&["--workers", "1", empty_hub.to_str().unwrap()]);
    let addr = hub.addr.clone();
    let mut w = start_worker(&addr, &worker_prog);
    hub.await_line("worker 1 joined");

    let (code, stdout, stderr) = hub.finish();
    assert_eq!(code, Some(0), "hub failed; stderr: {stderr}");
    assert_eq!(stdout, "0\n");
    assert_eq!(w.wait().unwrap().code(), Some(0), "worker should take its ∅");
}

#[test]
fn a_nil_result_is_a_value_not_the_end() {
    // The pump's body answers every item with ∅. On the wire that is a
    // value line, distinct from the `⇅ end` control line, so each ∅
    // acknowledges its item: the hub receives all three, its program
    // finishes, and both processes exit 0. (Before end-of-stream was
    // explicit, the worker swallowed these as its ∅ forward and the hub
    // waited forever.)
    let hub_prog = scratch("nil", "nil-hub.ml", "⟨1 2 3⟩⇈α\n↧β⌫↧β⌫↧β⌫«done»⍞\n");
    let worker_prog = scratch("nil", "nil-worker.ml", "[⌫∅]⇉αβ\n");

    let mut hub = start_hub(&["--workers", "1", hub_prog.to_str().unwrap()]);
    let addr = hub.addr.clone();
    let mut w = start_worker(&addr, worker_prog.to_str().unwrap());
    hub.await_line("worker 1 joined");

    let (code, stdout, stderr) = hub.finish();
    assert_eq!(code, Some(0), "hub failed; stderr: {stderr}");
    assert_eq!(stdout, "done\n");
    assert!(stderr.contains("worker 1 finished"), "hub stderr: {stderr}");
    assert_eq!(w.wait().unwrap().code(), Some(0), "worker should take its end");
}

#[test]
fn a_worker_killed_mid_item_is_requeued() {
    // SIGKILL, not a glitch: the worker process vanishes with items in
    // flight and no chance to say anything. The kernel closes its socket,
    // the hub requeues what it owed, and the survivor produces the exact
    // total. (100 items of real work, so worker 1 cannot have finished
    // them all before the kill lands.)
    let hub_prog = example("net-primes-hub.ml");
    let worker_prog = example("net-primes-worker.ml");
    let mut hub = start_hub(&["--workers", "1", &hub_prog, "100000", "1000"]);
    let addr = hub.addr.clone();

    let mut w1 = start_worker(&addr, &worker_prog);
    hub.await_line("worker 1 joined");
    // With --workers 1 the hub starts pouring only once this worker has
    // joined, so give the deal a moment to land before the kill — the
    // point is items in flight, not an empty-handed worker.
    std::thread::sleep(std::time::Duration::from_millis(400));
    w1.kill().unwrap();
    let mut w2 = start_worker(&addr, &worker_prog);

    let (code, stdout, stderr) = hub.finish();
    assert_eq!(code, Some(0), "hub failed; stderr: {stderr}");
    assert_eq!(stdout, "π(<100000) = 9592\nlargest: 99991\n");
    assert!(stderr.contains("worker 1 lost"), "hub stderr: {stderr}");
    assert!(stderr.contains("requeued"), "hub stderr: {stderr}");
    assert!(w1.wait().unwrap().code().is_none(), "w1 should have died by signal");
    assert_eq!(w2.wait().unwrap().code(), Some(0));
}

#[test]
fn a_poison_item_is_dropped_after_three_failures() {
    // Item 0 glitches every worker's pump. Each death is charged to the
    // item at the head of that worker's queue — item 0 every time — and
    // on the third the hub drops it with a diagnostic instead of
    // requeueing it. The remaining items still reach the next worker,
    // the end still propagates, and the hub terminates.
    let hub_prog = scratch("poison", "poison-hub.ml", "⟨0 1 2⟩⇈α\n⇟β#⍞\n");
    let worker_prog = scratch("poison", "poison-worker.ml", "[∂0=[«boom»↯][]?]⇉αβ\n");
    let worker_path = worker_prog.to_str().unwrap();

    let mut hub = start_hub(&["--workers", "1", hub_prog.to_str().unwrap()]);
    let addr = hub.addr.clone();
    let mut w1 = start_worker(&addr, worker_path);
    hub.await_line("worker 1 lost");
    let mut w2 = start_worker(&addr, worker_path);
    hub.await_line("worker 2 lost");
    let mut w3 = start_worker(&addr, worker_path);
    let dropped = hub.await_line("item dropped");
    assert_eq!(dropped, "⇅ item dropped after 3 worker failures: 0");
    let mut w4 = start_worker(&addr, worker_path);

    let (code, stdout, stderr) = hub.finish();
    assert_eq!(code, Some(0), "hub failed; stderr: {stderr}");
    assert_eq!(stdout, "2\n", "items 1 and 2 still answered; stderr: {stderr}");
    for w in [&mut w1, &mut w2, &mut w3] {
        assert_eq!(w.wait().unwrap().code(), Some(1), "the glitch is exit 1");
    }
    assert_eq!(w4.wait().unwrap().code(), Some(0));
}
