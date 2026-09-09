//! The web bridge — how MLang programs serve HTTP.
//!
//! `⎆` (accept) hands the program the next request as a value,
//! `⍅` (respond) sends the answer back. The language sees one thing only:
//! a stream of ⟨id method path body⟩ requests and the responses it makes —
//! never sockets, never headers.
//!
//! Two modes, one meaning (the ⌥ design, §5.1, applied to the web):
//!
//! * **Replay** (the default, and what the conformance corpus pins):
//!   requests are framed lines on stdin — `▷ METHOD PATH [nbytes]`, the
//!   body's nbytes following on the next line(s) — and ⍅ writes
//!   `◁ id status content-type nbytes` frames to stdout. A recorded
//!   session is deterministic, byte for byte.
//! * **Live** (`mlang serve`, or MLANG_PORT=… for a welded binary): a real
//!   TCP listener materializes each HTTP/1.1 request into exactly the
//!   value shape the replay frames produce, and ⍅ writes a real response.
//!   The request *stream* is the run's input; its arrival order is the
//!   outside world's timing, like `--parallel` interleaving.
//!
//! Nothing here can hang the grid, and nothing from the outside reaches
//! the program unless it is honest:
//!
//! * a request has a hard 10-second deadline from the moment its
//!   connection is admitted — not per read, so a client trickling one
//!   byte every few seconds is cut off just the same;
//! * header lines are capped at 8 KiB and 100 of them, bodies at 16 MiB
//!   (declared or chunked), and the body must be UTF-8 — the request
//!   value carries a *string*, so bytes that are not one cannot appear;
//! * at most 256 connections are parsed at once and at most 1024
//!   requests wait for ⎆; anything beyond that is answered 503 at once
//!   rather than being held until memory runs out;
//! * writing a response carries a 10-second timeout, so a client that
//!   never reads cannot stall the interpreter thread; a write that fails
//!   is dropped silently — the request still counts as answered.
//!
//! A request that violates any of the parsing rules is answered 400 by
//! the runtime and never reaches the program.

use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

const MAX_BODY: usize = 16 * 1024 * 1024;
const MAX_HEADER_LINE: usize = 8 * 1024;
const MAX_HEADERS: usize = 100;
const MAX_INFLIGHT: usize = 256;
const MAX_QUEUE: usize = 1024;
/// The whole-request parsing deadline (§5.2), and the response write
/// timeout — the same number, so one budget covers both directions.
const DEADLINE: Duration = Duration::from_secs(10);

/// One parsed request, in the shape ⎆ pushes: id, method, path, body.
pub type Request = (i64, String, String, String);

struct Queue {
    items: VecDeque<Request>,
    next_id: i64,
}

/// The live listener. Accepted connections are parsed on their own
/// threads, queued in arrival order, and held open until ⍅ answers them.
pub struct HttpBridge {
    queue: Mutex<Queue>,
    cv: Condvar,
    pending: Mutex<HashMap<i64, TcpStream>>,
    /// Connections currently being parsed — bounded by MAX_INFLIGHT.
    inflight: AtomicUsize,
    /// How long a request may take to arrive in full.
    deadline: Duration,
    pub port: u16,
}

/// Counts one parsing connection out again however the parse ends — a
/// panic on the parsing thread must not leak a slot.
struct InflightSlot<'a>(&'a AtomicUsize);

impl Drop for InflightSlot<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

impl HttpBridge {
    /// Bind 127.0.0.1:port (0 lets the OS choose) and start accepting,
    /// with the 10-second request deadline the spec promises.
    pub fn start(port: u16) -> std::io::Result<Arc<HttpBridge>> {
        HttpBridge::start_with_deadline(port, DEADLINE)
    }

    /// `start`, with the request deadline chosen by the caller. The
    /// runtime always uses the spec's 10 seconds; a shorter budget exists
    /// so tests can prove the deadline bites without waiting it out.
    pub fn start_with_deadline(port: u16, deadline: Duration) -> std::io::Result<Arc<HttpBridge>> {
        let listener = TcpListener::bind(("127.0.0.1", port))?;
        let port = listener.local_addr()?.port();
        let bridge = Arc::new(HttpBridge {
            queue: Mutex::new(Queue { items: VecDeque::new(), next_id: 1 }),
            cv: Condvar::new(),
            pending: Mutex::new(HashMap::new()),
            inflight: AtomicUsize::new(0),
            deadline,
            port,
        });
        let accepting = bridge.clone();
        std::thread::spawn(move || {
            for conn in listener.incoming() {
                let Ok(stream) = conn else { continue };
                let bridge = accepting.clone();
                std::thread::spawn(move || bridge.admit(stream));
            }
        });
        Ok(bridge)
    }

    fn admit(&self, stream: TcpStream) {
        // Claim a parsing slot before reading a byte; a flood beyond the
        // cap is turned away immediately instead of piling up threads.
        if self.inflight.fetch_add(1, Ordering::SeqCst) >= MAX_INFLIGHT {
            self.inflight.fetch_sub(1, Ordering::SeqCst);
            let _ = write_http_response(&stream, 503, "text/plain", b"too many connections");
            return;
        }
        let _slot = InflightSlot(&self.inflight);
        let Some((method, path, body)) = read_http_request(&stream, self.deadline) else {
            let _ = write_http_response(&stream, 400, "text/plain", b"bad request");
            return;
        };
        // The stream is registered as pending *before* the request is
        // visible to ⎆, so a fast ⍅ can never miss it. Both happen under
        // the queue lock; `respond` takes only the pending lock, so the
        // queue→pending order here cannot deadlock against it.
        let mut q = self.queue.lock().unwrap();
        if q.items.len() >= MAX_QUEUE {
            drop(q);
            let _ = write_http_response(&stream, 503, "text/plain", b"queue full");
            return;
        }
        let id = q.next_id;
        q.next_id += 1;
        self.pending.lock().unwrap().insert(id, stream);
        q.items.push_back((id, method, path, body));
        drop(q);
        self.cv.notify_all();
    }

    /// Park until the next request arrives. Live servers wait forever —
    /// there is no end-of-input on a listening port.
    pub fn accept(&self) -> Request {
        let mut q = self.queue.lock().unwrap();
        loop {
            if let Some(r) = q.items.pop_front() {
                return r;
            }
            q = self.cv.wait(q).unwrap();
        }
    }

    /// Answer a pending request. False when the id is unknown or already
    /// answered — ⍅ turns that into a glitch. Also false when the response
    /// itself is not honest (see `validate_response`): the client then
    /// gets a 500 from the runtime, never a forged header, and the id is
    /// consumed.
    pub fn respond(&self, id: i64, status: i64, ctype: &str, body: &str) -> bool {
        let Some(stream) = self.pending.lock().unwrap().remove(&id) else {
            return false;
        };
        if validate_response(status, ctype).is_err() {
            let _ = write_http_response(&stream, 500, "text/plain", b"bad response");
            return false;
        }
        // A client that stopped reading must not stall the interpreter
        // thread: the write gets the same budget the request had, and a
        // failure is the client's loss, not the program's.
        let _ = write_http_response(&stream, status, ctype, body.as_bytes());
        true
    }
}

/// Is a ⍅ response fit to put on the wire? The status must be a real
/// three-digit code (100…999) and the content type must not contain CR,
/// LF, or NUL — either would let a program (or the data it echoes) forge
/// extra header lines. The replay frame format has the same weakness, so
/// the VM calls this before writing in *either* mode and glitches ⍅ with
/// the returned message; `respond` re-checks as a backstop.
pub fn validate_response(status: i64, ctype: &str) -> Result<(), String> {
    if !(100..=999).contains(&status) {
        return Err(format!("⍅ status must be 100…999, got {status}"));
    }
    if ctype.bytes().any(|b| b == b'\r' || b == b'\n' || b == 0) {
        return Err("⍅ content type must not contain line breaks or NUL".to_string());
    }
    Ok(())
}

/// A socket reader that spends one time budget across every read it
/// makes: each read's timeout is what remains of the deadline, so the
/// total is bounded no matter how the bytes are paced. Lines and exact
/// byte counts come out of one buffer; every failure — timeout, EOF,
/// error, or an over-long line — is None, and None means 400.
struct Reader<'a> {
    stream: &'a TcpStream,
    buf: Vec<u8>,
    pos: usize,
    started: Instant,
    budget: Duration,
}

impl<'a> Reader<'a> {
    fn new(stream: &'a TcpStream, budget: Duration) -> Reader<'a> {
        Reader { stream, buf: Vec::new(), pos: 0, started: Instant::now(), budget }
    }

    /// Pull more bytes from the socket, or None when the deadline has
    /// passed, the peer closed, or the read failed.
    fn fill(&mut self) -> Option<()> {
        let remaining = self.budget.checked_sub(self.started.elapsed())?;
        if remaining.is_zero() {
            return None;
        }
        self.stream.set_read_timeout(Some(remaining)).ok()?;
        if self.pos > 0 {
            self.buf.drain(..self.pos);
            self.pos = 0;
        }
        let mut chunk = [0u8; 8192];
        let n = self.stream.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        self.buf.extend_from_slice(&chunk[..n]);
        Some(())
    }

    /// One line without its terminator (LF, CRLF); None if it would
    /// exceed `max` bytes.
    fn line(&mut self, max: usize) -> Option<Vec<u8>> {
        loop {
            if let Some(i) = self.buf[self.pos..].iter().position(|&b| b == b'\n') {
                let mut line = self.buf[self.pos..self.pos + i].to_vec();
                self.pos += i + 1;
                if line.last() == Some(&b'\r') {
                    line.pop();
                }
                if line.len() > max {
                    return None;
                }
                return Some(line);
            }
            if self.buf.len() - self.pos > max {
                return None;
            }
            self.fill()?;
        }
    }

    fn exact(&mut self, n: usize) -> Option<Vec<u8>> {
        while self.buf.len() - self.pos < n {
            self.fill()?;
        }
        let out = self.buf[self.pos..self.pos + n].to_vec();
        self.pos += n;
        Some(out)
    }
}

/// Read and parse one HTTP/1.1 request within `budget`. None = answer
/// 400 and close. Everything the request value needs — method, path,
/// and the UTF-8 body — comes out; everything else is checked and
/// discarded.
fn read_http_request(stream: &TcpStream, budget: Duration) -> Option<(String, String, String)> {
    let mut r = Reader::new(stream, budget);

    // Request line: exactly `METHOD PATH HTTP/1.x`, single spaces.
    let line = String::from_utf8(r.line(MAX_HEADER_LINE)?).ok()?;
    let mut parts = line.split(' ');
    let (Some(method), Some(path), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return None;
    };
    if method.is_empty() || !method.bytes().all(is_token_byte) {
        return None;
    }
    if !path.starts_with('/') || path.bytes().any(|b| b <= b' ' || b == 0x7f) {
        return None;
    }
    let minor = version.strip_prefix("HTTP/1.")?;
    if minor.len() != 1 || !minor.as_bytes()[0].is_ascii_digit() {
        return None;
    }
    let method = method.to_ascii_uppercase();
    let path = path.to_string();

    // Headers: each capped in length, all capped in count.
    let mut content_length: Option<usize> = None;
    let mut chunked = false;
    let mut expect_continue = false;
    let mut count = 0usize;
    loop {
        let header = r.line(MAX_HEADER_LINE)?;
        if header.is_empty() {
            break;
        }
        count += 1;
        if count > MAX_HEADERS {
            return None;
        }
        let header = String::from_utf8_lossy(&header);
        let (name, value) = header.split_once(':')?;
        let value = value.trim_matches(|c| c == ' ' || c == '\t');
        if name.eq_ignore_ascii_case("content-length") {
            // Plain digits only: no sign, no whitespace inside, and two
            // Content-Length headers must agree or the request is
            // smuggling something.
            if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let n: usize = value.parse().ok()?;
            if content_length.is_some_and(|prev| prev != n) {
                return None;
            }
            content_length = Some(n);
        } else if name.eq_ignore_ascii_case("transfer-encoding") {
            // Only the plain chunked coding is understood; anything else
            // (gzip, or chunked as one of several) cannot be framed.
            if value.eq_ignore_ascii_case("chunked") {
                chunked = true;
            } else {
                return None;
            }
        } else if name.eq_ignore_ascii_case("expect") {
            if value.eq_ignore_ascii_case("100-continue") {
                expect_continue = true;
            } else {
                return None;
            }
        }
    }
    // Both framings at once is the classic request-smuggling shape.
    if chunked && content_length.is_some() {
        return None;
    }
    if content_length.is_some_and(|n| n > MAX_BODY) {
        return None;
    }

    // A client waiting for permission to send its body gets it now —
    // the size has been checked, so the answer is never "no".
    if expect_continue {
        let _ = stream.set_write_timeout(Some(DEADLINE));
        let mut s = stream;
        if s.write_all(b"HTTP/1.1 100 Continue\r\n\r\n").is_err() {
            return None;
        }
    }

    let body = if chunked {
        read_chunked_body(&mut r)?
    } else {
        r.exact(content_length.unwrap_or(0))?
    };
    // The request value carries a string; bytes that are not UTF-8 have
    // no honest representation in it.
    let body = String::from_utf8(body).ok()?;
    Some((method, path, body))
}

/// Decode a `Transfer-Encoding: chunked` body: hex-sized chunks (any
/// `;extension` ignored), a zero chunk, then trailers up to a blank line.
/// The 16 MiB cap applies to the decoded total.
fn read_chunked_body(r: &mut Reader) -> Option<Vec<u8>> {
    let mut body = Vec::new();
    loop {
        let size_line = r.line(MAX_HEADER_LINE)?;
        let size_line = String::from_utf8_lossy(&size_line);
        let hex = size_line.split(';').next().unwrap_or("").trim();
        if hex.is_empty() || hex.len() > 16 {
            return None;
        }
        let size = usize::from_str_radix(hex, 16).ok()?;
        if size == 0 {
            // trailers, then the terminating blank line
            let mut trailers = 0usize;
            loop {
                let t = r.line(MAX_HEADER_LINE)?;
                if t.is_empty() {
                    break;
                }
                trailers += 1;
                if trailers > MAX_HEADERS {
                    return None;
                }
            }
            return Some(body);
        }
        if body.len().checked_add(size)? > MAX_BODY {
            return None;
        }
        body.extend_from_slice(&r.exact(size)?);
        // each chunk's data is followed by its own CRLF
        if !r.line(2)?.is_empty() {
            return None;
        }
    }
}

/// RFC 9110 token characters — what a method name may be made of.
fn is_token_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b)
}

fn reason(status: i64) -> &'static str {
    match status {
        100 => "Continue",
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        411 => "Length Required",
        413 => "Content Too Large",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "",
    }
}

/// Write one complete response and close. The page ⎆/⍅ serve is
/// same-origin with its own API, so no CORS header is offered: a page on
/// another origin must not be able to read what this server answers.
fn write_http_response(
    mut stream: &TcpStream,
    status: i64,
    ctype: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let _ = stream.set_write_timeout(Some(DEADLINE));
    let head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nConnection: close\r\n\r\n",
        status,
        reason(status),
        ctype,
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

/// Parse one replay frame from a byte source:
///     ▷ METHOD PATH [nbytes]
/// followed, when nbytes is present, by exactly nbytes of body and an
/// optional line ending. Blank lines between frames are skipped.
/// Ok(None) is clean end of input; Err carries the offending line.
pub fn read_framed(
    next: &mut dyn FnMut() -> Option<u8>,
) -> Result<Option<(String, String, String)>, String> {
    let line = loop {
        let mut bytes: Vec<u8> = Vec::new();
        loop {
            match next() {
                None if bytes.is_empty() => return Ok(None),
                None => break,
                Some(b'\n') => break,
                Some(b) => bytes.push(b),
            }
        }
        let line = String::from_utf8_lossy(&bytes).trim_end_matches('\r').to_string();
        if !line.trim().is_empty() {
            break line;
        }
    };
    let mut parts = line.split_ascii_whitespace();
    if parts.next() != Some("▷") {
        return Err(line.clone());
    }
    let (Some(method), Some(path)) = (parts.next(), parts.next()) else {
        return Err(line.clone());
    };
    let body = match parts.next() {
        None => String::new(),
        Some(n) => {
            let Ok(n) = n.parse::<usize>() else {
                return Err(line.clone());
            };
            if n > MAX_BODY || parts.next().is_some() {
                return Err(line.clone());
            }
            let mut bytes = Vec::with_capacity(n);
            for _ in 0..n {
                match next() {
                    Some(b) => bytes.push(b),
                    None => return Err(line.clone()),
                }
            }
            String::from_utf8_lossy(&bytes).into_owned()
        }
    };
    Ok(Some((method.to_ascii_uppercase(), path.to_string(), body)))
}

/// Format one replay response frame, the ⍅ counterpart of ▷:
///     ◁ id status content-type nbytes
/// with the body (and a readability newline) following. The caller is
/// expected to have passed `validate_response` first, so the frame line
/// cannot be broken by its own content type.
pub fn write_framed(id: i64, status: i64, ctype: &str, body: &str) -> String {
    format!("◁ {} {} {} {}\n{}\n", id, status, ctype, body.len(), body)
}
