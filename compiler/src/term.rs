//! Terminal size for ⍜. Raw-mode handling lives in the runner's
//! TerminalSession (main.rs); this module only answers how big the
//! terminal is — ⟨24 80⟩ when there is none, so recorded runs are
//! deterministic.

/// (rows, cols) of the terminal, or (24, 80) when there is none.
pub fn size() -> (i64, i64) {
    sys::size().unwrap_or((24, 80))
}

#[cfg(unix)]
use unix as sys;
#[cfg(unix)]
mod unix {
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
    pub fn size() -> Option<(i64, i64)> {
        use windows_sys::Win32::System::Console::*;
        unsafe {
            let mut info: CONSOLE_SCREEN_BUFFER_INFO = std::mem::zeroed();
            if GetConsoleScreenBufferInfo(GetStdHandle(STD_OUTPUT_HANDLE), &mut info) == 0 {
                return None;
            }
            let rows = (info.srWindow.Bottom - info.srWindow.Top + 1) as i64;
            let cols = (info.srWindow.Right - info.srWindow.Left + 1) as i64;
            if rows <= 0 || cols <= 0 {
                return None;
            }
            Some((rows, cols))
        }
    }
}
