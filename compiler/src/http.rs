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
//! Nothing here can hang the grid: reading a live request carries a hard
//! 10-second deadline, a request too large to be honest is answered 400,
//! and a malformed one never reaches the program.

use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

const MAX_BODY: usize = 16 * 1024 * 1024;

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
    pub port: u16,
}

impl HttpBridge {
    /// Bind 127.0.0.1:port (0 lets the OS choose) and start accepting.
    pub fn start(port: u16) -> std::io::Result<Arc<HttpBridge>> {
        let listener = TcpListener::bind(("127.0.0.1", port))?;
        let port = listener.local_addr()?.port();
        let bridge = Arc::new(HttpBridge {
            queue: Mutex::new(Queue { items: VecDeque::new(), next_id: 1 }),
            cv: Condvar::new(),
            pending: Mutex::new(HashMap::new()),
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
        match read_http_request(&stream) {
            Some((method, path, body)) => {
                let id = {
                    let mut q = self.queue.lock().unwrap();
                    let id = q.next_id;
                    q.next_id += 1;
                    q.items.push_back((id, method, path, body));
                    id
                };
                self.pending.lock().unwrap().insert(id, stream);
                self.cv.notify_all();
            }
            None => {
                let _ = write_http_response(&stream, 400, "text/plain", b"bad request");
            }
        }
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
    /// answered — ⍅ turns that into a glitch.
    pub fn respond(&self, id: i64, status: i64, ctype: &str, body: &str) -> bool {
        let Some(stream) = self.pending.lock().unwrap().remove(&id) else {
            return false;
        };
        let _ = write_http_response(&stream, status, ctype, body.as_bytes());
        true
    }
}

/// Read and parse one HTTP/1.1 request. None = answer 400 and close.
fn read_http_request(stream: &TcpStream) -> Option<(String, String, String)> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let mut parts = line.split_ascii_whitespace();
    let method = parts.next()?.to_ascii_uppercase();
    let path = parts.next()?.to_string();
    if path.is_empty() || !path.starts_with('/') {
        return None;
    }
    let mut content_length = 0usize;
    loop {
        line.clear();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let header = line.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().ok()?;
            }
        }
    }
    if content_length > MAX_BODY {
        return None;
    }
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).ok()?;
    Some((method, path, String::from_utf8_lossy(&body).into_owned()))
}

fn reason(status: i64) -> &'static str {
    match status {
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
        500 => "Internal Server Error",
        _ => "",
    }
}

fn write_http_response(
    mut stream: &TcpStream,
    status: i64,
    ctype: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\nCache-Control: no-store\r\n\
         Connection: close\r\n\r\n",
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
/// with the body (and a readability newline) following.
pub fn write_framed(id: i64, status: i64, ctype: &str, body: &str) -> String {
    format!("◁ {} {} {} {}\n{}\n", id, status, ctype, body.len(), body)
}
