//! The live web bridge, exercised directly over TCP: what reaches ⎆, what
//! is answered 400 by the runtime, and what ⍅ refuses to put on the wire.
//! Each test binds its own OS-chosen port, so they run in parallel.

use mlang::http::{HttpBridge, validate_response};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

fn bridge() -> Arc<HttpBridge> {
    HttpBridge::start(0).expect("bind 127.0.0.1:0")
}

fn connect(b: &HttpBridge) -> TcpStream {
    let s = TcpStream::connect(("127.0.0.1", b.port)).expect("connect");
    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    s
}

/// Send raw bytes and read the whole reply (the bridge always closes).
fn exchange(b: &HttpBridge, raw: &[u8]) -> String {
    let mut s = connect(b);
    s.write_all(raw).unwrap();
    let mut reply = Vec::new();
    let _ = s.read_to_end(&mut reply);
    String::from_utf8_lossy(&reply).into_owned()
}

/// Read a reply on an already-written stream.
fn reply(mut s: TcpStream) -> String {
    let mut reply = Vec::new();
    let _ = s.read_to_end(&mut reply);
    String::from_utf8_lossy(&reply).into_owned()
}

#[test]
fn get_is_queued_and_answered() {
    let b = bridge();
    let mut s = connect(&b);
    s.write_all(b"get /hello?x=1 HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();
    let (id, method, path, body) = b.accept();
    assert_eq!((id, method.as_str(), path.as_str(), body.as_str()), (1, "GET", "/hello?x=1", ""));
    assert!(b.respond(id, 200, "text/plain", "hi"));
    let r = reply(s);
    assert!(r.starts_with("HTTP/1.1 200 OK\r\n"), "{r}");
    assert!(r.contains("Content-Type: text/plain\r\n"), "{r}");
    assert!(r.contains("Content-Length: 2\r\n"), "{r}");
    assert!(r.contains("Connection: close\r\n"), "{r}");
    assert!(!r.contains("Access-Control"), "no CORS header: {r}");
    assert!(r.ends_with("\r\n\r\nhi"), "{r}");
    // answered once; a second answer is a glitch
    assert!(!b.respond(id, 200, "text/plain", "again"));
}

#[test]
fn post_body_reaches_the_program() {
    let b = bridge();
    let mut s = connect(&b);
    // "héllo" is six bytes of UTF-8 and five characters; the value
    // carries the characters
    s.write_all("POST /p HTTP/1.0\r\nContent-Length: 6\r\n\r\nhéllo".as_bytes()).unwrap();
    let (id, method, path, body) = b.accept();
    assert_eq!((method.as_str(), path.as_str(), body.as_str()), ("POST", "/p", "héllo"));
    assert!(b.respond(id, 201, "text/plain", ""));
    assert!(reply(s).starts_with("HTTP/1.1 201 "));
}

#[test]
fn oversize_content_length_is_400() {
    let b = bridge();
    let r = exchange(&b, b"POST / HTTP/1.1\r\nContent-Length: 16777217\r\n\r\n");
    assert!(r.starts_with("HTTP/1.1 400 "), "{r}");
}

#[test]
fn chunked_body_is_decoded() {
    let b = bridge();
    let mut s = connect(&b);
    s.write_all(
        b"POST /c HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n\
          5;ext=1\r\nhello\r\n1\r\n \r\n6\r\nworld!\r\n0\r\nTrailer: x\r\n\r\n",
    )
    .unwrap();
    let (id, method, path, body) = b.accept();
    assert_eq!((method.as_str(), path.as_str(), body.as_str()), ("POST", "/c", "hello world!"));
    assert!(b.respond(id, 200, "text/plain", "ok"));
    assert!(reply(s).starts_with("HTTP/1.1 200 "));
}

#[test]
fn chunked_with_content_length_is_400() {
    let b = bridge();
    let r = exchange(
        &b,
        b"POST / HTTP/1.1\r\nContent-Length: 3\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n",
    );
    assert!(r.starts_with("HTTP/1.1 400 "), "{r}");
}

#[test]
fn invalid_utf8_body_is_400() {
    let b = bridge();
    let r = exchange(&b, b"POST / HTTP/1.1\r\nContent-Length: 2\r\n\r\n\xff\xfe");
    assert!(r.starts_with("HTTP/1.1 400 "), "{r}");
}

#[test]
fn request_line_must_be_http_1x() {
    let b = bridge();
    for raw in [
        &b"GET /\r\n\r\n"[..],
        b"GET / HTTP/2\r\n\r\n",
        b"GET  / HTTP/1.1\r\n\r\n",
        b"GET / HTTP/1.1 extra\r\n\r\n",
        b"GET nopath HTTP/1.1\r\n\r\n",
    ] {
        let r = exchange(&b, raw);
        assert!(r.starts_with("HTTP/1.1 400 "), "{:?} → {r}", String::from_utf8_lossy(raw));
    }
}

#[test]
fn content_length_must_be_plain_digits_and_consistent() {
    let b = bridge();
    let r = exchange(&b, b"POST / HTTP/1.1\r\nContent-Length: +2\r\n\r\nhi");
    assert!(r.starts_with("HTTP/1.1 400 "), "{r}");
    let r = exchange(&b, b"POST / HTTP/1.1\r\nContent-Length: 2\r\nContent-Length: 3\r\n\r\nhi");
    assert!(r.starts_with("HTTP/1.1 400 "), "{r}");
    // two that agree are fine
    let mut s = connect(&b);
    s.write_all(b"POST / HTTP/1.1\r\nContent-Length: 2\r\nContent-Length: 2\r\n\r\nhi").unwrap();
    let (id, _, _, body) = b.accept();
    assert_eq!(body, "hi");
    assert!(b.respond(id, 200, "text/plain", ""));
}

#[test]
fn header_limits_are_400() {
    let b = bridge();
    let long = format!("GET / HTTP/1.1\r\nX: {}\r\n\r\n", "a".repeat(9000));
    let r = exchange(&b, long.as_bytes());
    assert!(r.starts_with("HTTP/1.1 400 "), "long header line: {r}");
    let mut many = String::from("GET / HTTP/1.1\r\n");
    for i in 0..101 {
        many.push_str(&format!("X-{i}: 1\r\n"));
    }
    many.push_str("\r\n");
    let r = exchange(&b, many.as_bytes());
    assert!(r.starts_with("HTTP/1.1 400 "), "too many headers: {r}");
}

#[test]
fn expect_continue_is_answered_before_the_body() {
    let b = bridge();
    let mut s = connect(&b);
    s.write_all(b"POST / HTTP/1.1\r\nContent-Length: 2\r\nExpect: 100-continue\r\n\r\n").unwrap();
    let mut interim = [0u8; 25];
    s.read_exact(&mut interim).unwrap();
    assert_eq!(&interim, b"HTTP/1.1 100 Continue\r\n\r\n");
    s.write_all(b"ok").unwrap();
    let (id, _, _, body) = b.accept();
    assert_eq!(body, "ok");
    assert!(b.respond(id, 204, "text/plain", ""));
    assert!(reply(s).starts_with("HTTP/1.1 204 "));
}

#[test]
fn slow_trickle_is_cut_off_by_the_total_deadline() {
    // A 300 ms budget; each byte arrives well inside any per-read idle
    // window, but the whole request never does.
    let b = HttpBridge::start_with_deadline(0, Duration::from_millis(300)).unwrap();
    let mut s = connect(&b);
    let started = std::time::Instant::now();
    let raw = b"GET / HTTP/1.1\r\nX: 1\r\n\r\n";
    let mut sent_all = true;
    for byte in raw {
        if s.write_all(&[*byte]).is_err() {
            sent_all = false;
            break;
        }
        std::thread::sleep(Duration::from_millis(40));
    }
    let r = reply(s);
    assert!(started.elapsed() < Duration::from_secs(3));
    assert!(r.starts_with("HTTP/1.1 400 "), "sent_all={sent_all}: {r:?}");
}

#[test]
fn respond_refuses_header_injection() {
    let b = bridge();
    let s = connect(&b);
    let mut s2 = s.try_clone().unwrap();
    s2.write_all(b"GET / HTTP/1.1\r\n\r\n").unwrap();
    let (id, _, _, _) = b.accept();
    assert!(!b.respond(id, 200, "text/plain\r\nSet-Cookie: x=1", "hi"));
    let r = reply(s);
    assert!(r.starts_with("HTTP/1.1 500 "), "{r}");
    assert!(!r.contains("Set-Cookie"), "{r}");
}

#[test]
fn respond_refuses_an_impossible_status() {
    let b = bridge();
    let mut s = connect(&b);
    s.write_all(b"GET / HTTP/1.1\r\n\r\n").unwrap();
    let (id, _, _, _) = b.accept();
    assert!(!b.respond(id, 99999, "text/plain", "hi"));
    assert!(reply(s).starts_with("HTTP/1.1 500 "));
    // unknown ids are still just false
    assert!(!b.respond(42, 200, "text/plain", ""));
}

#[test]
fn validate_response_covers_both_modes() {
    assert!(validate_response(200, "text/plain").is_ok());
    assert!(validate_response(100, "x/y").is_ok());
    assert!(validate_response(999, "x/y").is_ok());
    assert!(validate_response(99, "x/y").is_err());
    assert!(validate_response(1000, "x/y").is_err());
    assert!(validate_response(-1, "x/y").is_err());
    assert!(validate_response(200, "a\rb").is_err());
    assert!(validate_response(200, "a\nb").is_err());
    assert!(validate_response(200, "a\0b").is_err());
}

#[test]
fn a_client_that_never_reads_does_not_stall_respond() {
    // The write timeout is 10 s, so this only checks that a small body
    // to a closed peer returns promptly and still counts as answered.
    let b = bridge();
    let mut s = connect(&b);
    s.write_all(b"GET / HTTP/1.1\r\n\r\n").unwrap();
    let (id, _, _, _) = b.accept();
    drop(s);
    let started = std::time::Instant::now();
    assert!(b.respond(id, 200, "text/plain", &"x".repeat(1 << 20)));
    assert!(started.elapsed() < Duration::from_secs(3));
}
