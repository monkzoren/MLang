//! The loom over a live socket: a served program is pulled, patched, and
//! keeps answering — on the new code — without restarting.

use mlang::http::HttpBridge;
use mlang::loom::{self, Loom};
use mlang::vm;
use std::io::{Cursor, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

/// A tiny server: answers every request with `3 Q`, where Q is a boot
/// definition; a nil request (end of input) stops it.
const V0: &str = "[∂×]≔Q\n⇊\n1⇒g[g][⎆∂∅=[⌫0⇒g][⟨⇅0@ 200 «text/plain» 3Q⍕⟩⍅ 1]?]⟳\n";

fn http(port: u16, req: &str) -> (u16, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    s.write_all(req.as_bytes()).unwrap();
    let mut buf = String::new();
    s.read_to_string(&mut buf).unwrap();
    let status: u16 = buf.split_whitespace().nth(1).unwrap().parse().unwrap();
    let body = buf.split_once("\r\n\r\n").map(|(_, b)| b.to_string()).unwrap_or_default();
    (status, body)
}

fn get(port: u16, path: &str) -> (u16, String) {
    http(port, &format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n"))
}

fn post(port: u16, path: &str, body: &str) -> (u16, String) {
    http(
        port,
        &format!(
            "POST {path} HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        ),
    )
}

/// Serve V0 on a fresh port with the loom open; returns the port and the
/// server thread (which ends when the bridge is dropped — never, here).
fn serve() -> u16 {
    let bridge = HttpBridge::start(0).unwrap();
    let port = bridge.port;
    let loom = Loom::new(V0);
    bridge.attach_loom(loom.clone());
    let prog = vm::compile_text(V0).unwrap();
    std::thread::spawn(move || {
        let mut stdin = Cursor::new(Vec::new());
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut machine = vm::VM::new(&mut stdin, &mut out, &mut err);
        machine.http = Some(bridge);
        machine.loom = Some(loom);
        machine.run_compiled(&prog);
    });
    port
}

#[test]
fn pull_patch_and_keep_serving() {
    let port = serve();
    assert_eq!(get(port, "/anything"), (200, "9".into()));

    // Pull: the live source, stamped with its version and origin.
    let (status, pulled) = get(port, "/.loom");
    assert_eq!(status, 200);
    let (v, url, text) = loom::unstamp(&pulled);
    assert_eq!((v, text), (Some(0), V0));
    assert_eq!(url, format!("http://127.0.0.1:{port}"));

    // Patch: rebind Q. The stamp carries the base version.
    let edited = pulled.replace("[∂×]≔Q", "[∂×∂×]≔Q");
    let (status, report) = post(port, "/.loom", &edited);
    assert_eq!(status, 200, "{report}");
    assert_eq!(report, "⟡ v1: 1 definition rebound (Q)\n");
    assert_eq!(get(port, "/again"), (200, "81".into()));

    // A second agent, still on v0, edits a different line: merged.
    let (_, pulled_v1) = get(port, "/.loom");
    let stale = pulled.replace("3Q⍕", "«=»3Q⍕⧺");
    let (status, report) = post(port, "/.loom", &stale);
    assert_eq!(status, 200, "{report}");
    assert!(report.starts_with("⟡ v2: 1 strand replaced\n"), "{report}");
    assert_eq!(get(port, "/merged"), (200, "=81".into()));
    let (_, pulled_v2) = get(port, "/.loom");
    assert!(pulled_v2.contains("[∂×∂×]≔Q") && pulled_v2.contains("«=»3Q⍕⧺"));
    assert_ne!(pulled_v1, pulled_v2);

    // A third agent, also on v0, changes Q differently: conflict, and
    // the grid is untouched.
    let (status, report) = post(port, "/.loom", &pulled.replace("[∂×]≔Q", "[∂+]≔Q"));
    assert_eq!(status, 409);
    assert!(report.contains("conflicts with v2"), "{report}");
    assert_eq!(get(port, "/still"), (200, "=81".into()));

    // The log names every version.
    let (_, log) = get(port, "/.loom/log");
    assert_eq!(log, "v0  as started\nv1  1 definition rebound (Q)\nv2  1 strand replaced\n");
    let (status, old) = get(port, "/.loom/v0");
    assert_eq!((status, loom::unstamp(&old).2), (200, V0));

    // A patch that changes boot code, or has no stamp, is refused.
    let (status, _) = post(port, "/.loom", &pulled_v2.replace("[∂×∂×]≔Q", "[∂×∂×]≔Q 1⍞"));
    assert_eq!(status, 422);
    let (status, why) = post(port, "/.loom", V0);
    assert_eq!(status, 400);
    assert!(why.contains("mlang pull"), "{why}");
    // A stampless patch may name its base in the query: V0 against base
    // v0 changes nothing, so it merges onto v2 as a no-op version.
    let (status, report) = post(port, "/.loom?base=0", V0);
    assert_eq!((status, report.as_str()), (200, "⟡ v3: no change\n"));
    assert_eq!(get(port, "/.loom/nope").0, 404);
}

#[test]
fn without_a_loom_the_routes_are_closed() {
    let bridge = HttpBridge::start(0).unwrap();
    let port = bridge.port;
    let prog = vm::compile_text(V0).unwrap();
    std::thread::spawn(move || {
        let mut stdin = Cursor::new(Vec::new());
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let mut machine = vm::VM::new(&mut stdin, &mut out, &mut err);
        machine.http = Some(bridge);
        machine.run_compiled(&prog);
    });
    assert_eq!(get(port, "/.loom").0, 404);
    assert_eq!(get(port, "/x"), (200, "9".into()));
}

#[test]
fn a_welded_source_round_trips_through_the_stamp() {
    let s = loom::stamp(4, "http://127.0.0.1:1", V0);
    assert!(vm::compile_text(&s).is_ok(), "a stamped file is still a valid program");
    let (v, _, text) = loom::unstamp(&s);
    assert_eq!((v, text), (Some(4), V0));
}

#[test]
fn shared_loom_arc_is_the_same_store() {
    let loom = Loom::new("1⍞\n");
    let other: Arc<Loom> = loom.clone();
    other.push("2⍞\n".into(), "test".into());
    assert_eq!(loom.current(), 1);
    assert_eq!(loom.text(1).as_deref(), Some("2⍞\n"));
}
