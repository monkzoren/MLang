//! The distributed runtime: a hub and its workers as real processes.
//!
//! The prime-finder example reduces order-independently (a sum and a
//! max), so however results interleave on the wire, the hub's stdout is
//! byte-exact — including when a worker dies mid-run and its items are
//! requeued on the survivors.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

fn mlang() -> &'static str {
    env!("CARGO_BIN_EXE_mlang")
}

fn example(name: &str) -> String {
    format!("{}/../examples/{name}", env!("CARGO_MANIFEST_DIR"))
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
    fn finish(mut self) -> (Option<i32>, String, String) {
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
    let dir = std::env::temp_dir().join("mlang-net-test");
    std::fs::create_dir_all(&dir).unwrap();
    let poison = dir.join("poison-worker.ml");
    {
        let mut f = std::fs::File::create(&poison).unwrap();
        writeln!(f, "[«boom»↯]⇉αβ").unwrap();
    }

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
    let dir = std::env::temp_dir().join("mlang-net-test");
    std::fs::create_dir_all(&dir).unwrap();
    let empty_hub = dir.join("empty-hub.ml");
    {
        let mut f = std::fs::File::create(&empty_hub).unwrap();
        writeln!(f, "⟨⟩⇈α").unwrap();
        writeln!(f, "⇟β#⍞").unwrap();
    }
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
