//! Raw-terminal support for ⌦ (read one key) and ⍜ (terminal size).
//!
//! The first ⌦ puts the controlling terminal into raw mode — no line
//! buffering, no echo, no ^C signal, no ^S/^Q flow control — and turns on
//! VT (ANSI) processing for output so full-screen programs work even in a
//! legacy Windows console. The runner restores the original modes when the
//! program ends. When stdin is not a terminal (a pipe, a conformance run)
//! everything here is a no-op and ⍜ reports a fixed 24×80, so recorded
//! behavior stays deterministic.

use std::sync::Mutex;
use sys::Saved;

static SAVED: Mutex<Option<Saved>> = Mutex::new(None);

pub fn enter_raw() {
    let mut saved = SAVED.lock().unwrap();
    if saved.is_none() {
        *saved = sys::enter_raw();
    }
}

pub fn restore() {
    if let Some(orig) = SAVED.lock().unwrap().take() {
        sys::restore(orig);
    }
}

/// (rows, cols) of the terminal, or (24, 80) when there is none.
pub fn size() -> (i64, i64) {
    sys::size().unwrap_or((24, 80))
}

#[cfg(unix)]
use unix as sys;
#[cfg(unix)]
mod unix {
    pub struct Saved(libc::termios);

    pub fn enter_raw() -> Option<Saved> {
        unsafe {
            if libc::isatty(0) == 0 {
                return None;
            }
            let mut orig: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(0, &mut orig) != 0 {
                return None;
            }
            let mut raw = orig;
            // No line buffering, no echo, no ^C/^Z signals, no ^S/^Q flow
            // control (^S must reach the program — it's the save key).
            raw.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ISIG | libc::IEXTEN);
            raw.c_iflag &= !(libc::IXON | libc::ICRNL | libc::BRKINT | libc::INPCK | libc::ISTRIP);
            raw.c_cc[libc::VMIN] = 1;
            raw.c_cc[libc::VTIME] = 0;
            if libc::tcsetattr(0, libc::TCSANOW, &raw) != 0 {
                return None;
            }
            Some(Saved(orig))
        }
    }

    pub fn restore(saved: Saved) {
        unsafe {
            libc::tcsetattr(0, libc::TCSANOW, &saved.0);
        }
    }

    pub fn size() -> Option<(i64, i64)> {
        unsafe {
            if libc::isatty(1) == 0 {
                return None;
            }
            let mut ws: libc::winsize = std::mem::zeroed();
            if libc::ioctl(1, libc::TIOCGWINSZ, &mut ws) != 0 || ws.ws_row == 0 {
                return None;
            }
            Some((ws.ws_row as i64, ws.ws_col as i64))
        }
    }
}

#[cfg(windows)]
use windows as sys;
#[cfg(windows)]
mod windows {
    // Hand-rolled kernel32 bindings — the layouts are stable ABI.
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Coord {
        x: i16,
        y: i16,
    }
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct SmallRect {
        left: i16,
        top: i16,
        right: i16,
        bottom: i16,
    }
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct ScreenInfo {
        size: Coord,
        cursor: Coord,
        attrs: u16,
        window: SmallRect,
        max_size: Coord,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetStdHandle(which: u32) -> isize;
        fn GetConsoleMode(handle: isize, mode: *mut u32) -> i32;
        fn SetConsoleMode(handle: isize, mode: u32) -> i32;
        fn GetConsoleScreenBufferInfo(handle: isize, info: *mut ScreenInfo) -> i32;
    }
    const STD_INPUT: u32 = -10i32 as u32;
    const STD_OUTPUT: u32 = -11i32 as u32;
    const ENABLE_PROCESSED_INPUT: u32 = 0x0001;
    const ENABLE_LINE_INPUT: u32 = 0x0002;
    const ENABLE_ECHO_INPUT: u32 = 0x0004;
    const ENABLE_VIRTUAL_TERMINAL_INPUT: u32 = 0x0200;
    const ENABLE_PROCESSED_OUTPUT: u32 = 0x0001;
    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;

    pub struct Saved {
        stdin_mode: u32,
        stdout_mode: u32,
    }

    pub fn enter_raw() -> Option<Saved> {
        unsafe {
            let hin = GetStdHandle(STD_INPUT);
            let hout = GetStdHandle(STD_OUTPUT);
            let (mut min, mut mout) = (0u32, 0u32);
            if GetConsoleMode(hin, &mut min) == 0 || GetConsoleMode(hout, &mut mout) == 0 {
                return None; // not a console
            }
            // Raw keys, arriving as VT escape sequences like on Unix; ANSI
            // output processing on, so this works even in legacy conhost.
            let raw_in = (min & !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT))
                | ENABLE_VIRTUAL_TERMINAL_INPUT;
            let vt_out = mout | ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING;
            if SetConsoleMode(hin, raw_in) == 0 {
                return None;
            }
            SetConsoleMode(hout, vt_out);
            Some(Saved { stdin_mode: min, stdout_mode: mout })
        }
    }

    pub fn restore(saved: Saved) {
        unsafe {
            SetConsoleMode(GetStdHandle(STD_INPUT), saved.stdin_mode);
            SetConsoleMode(GetStdHandle(STD_OUTPUT), saved.stdout_mode);
        }
    }

    pub fn size() -> Option<(i64, i64)> {
        unsafe {
            let mut info = ScreenInfo::default();
            if GetConsoleScreenBufferInfo(GetStdHandle(STD_OUTPUT), &mut info) == 0 {
                return None;
            }
            let rows = (info.window.bottom - info.window.top + 1) as i64;
            let cols = (info.window.right - info.window.left + 1) as i64;
            if rows <= 0 || cols <= 0 {
                return None;
            }
            Some((rows, cols))
        }
    }
}

/// One keystroke from the raw byte stream, decoded to MLang's key names:
/// printable characters stand for themselves (UTF-8 aware); «⏎» «⌫» «⇥»
/// «⎋» for enter/backspace/tab/escape; «↑» «↓» «←» «→» «⇱» «⇲» «⇞» «⇟»
/// «⌦» «⎀» for the navigation block; «^A»…«^Z» for control chords.
/// `None` at end of input. The byte → key mapping is pure, so piped input
/// produces identical runs — VT sequences in the pipe decode like live
/// keys.
pub fn read_key(r: &mut dyn std::io::BufRead) -> Option<String> {
    let b = byte(r)?;
    Some(match b {
        // Enter is the newline string — «⏎» in MLang source is exactly \n.
        b'\r' | b'\n' => "\n".into(),
        0x7f | 0x08 => "⌫".into(),
        b'\t' => "⇥".into(),
        0x1b => match byte(r) {
            None => "⎋".into(),
            Some(b'[') | Some(b'O') => {
                let mut params = String::new();
                let fin = loop {
                    match byte(r) {
                        None => return Some("⎋".into()),
                        Some(c @ 0x40..=0x7e) => break c,
                        Some(c) => params.push(c as char),
                    }
                };
                match (fin, params.as_str()) {
                    (b'A', _) => "↑".into(),
                    (b'B', _) => "↓".into(),
                    (b'C', _) => "→".into(),
                    (b'D', _) => "←".into(),
                    (b'H', _) => "⇱".into(),
                    (b'F', _) => "⇲".into(),
                    (b'~', "1") | (b'~', "7") => "⇱".into(),
                    (b'~', "4") | (b'~', "8") => "⇲".into(),
                    (b'~', "2") => "⎀".into(),
                    (b'~', "3") => "⌦".into(),
                    (b'~', "5") => "⇞".into(),
                    (b'~', "6") => "⇟".into(),
                    _ => "⎋".into(),
                }
            }
            // Alt-chords decode as ⎋ plus the key, e.g. «⎋x».
            Some(other) => format!("⎋{}", char::from(other)),
        },
        0x01..=0x1a => format!("^{}", char::from(b + 0x40)),
        0x00 => "^@".into(),
        _ if b < 0x80 => char::from(b).to_string(),
        _ => {
            // UTF-8: continuation length from the leading byte.
            let extra = if b >= 0xf0 {
                3
            } else if b >= 0xe0 {
                2
            } else {
                1
            };
            let mut buf = vec![b];
            for _ in 0..extra {
                buf.push(byte(r)?);
            }
            String::from_utf8_lossy(&buf).into_owned()
        }
    })
}

fn byte(r: &mut dyn std::io::BufRead) -> Option<u8> {
    let mut b = [0u8; 1];
    match r.read_exact(&mut b) {
        Ok(()) => Some(b[0]),
        Err(_) => None,
    }
}
