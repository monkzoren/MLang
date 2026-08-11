//! The canvas ops (⌸ ▦ ⌶ ⎙): a pixel surface MLang programs draw into.
//!
//! One surface, two backends. Headless — the default whenever there is no
//! interactive display (piped stdin, CI, MLANG_HEADLESS=1, or a build
//! without the `gui` feature) — keeps the frame in memory: ⌸ and ⎙ print
//! one line each (size and title, then a hash per presented frame), so a
//! recorded run pins every pixel byte-for-byte in the conformance corpus,
//! and MLANG_FRAMES=<dir> dumps each presented frame as a PPM image for
//! inspection. Windowed — a real OS window via minifb, chosen when stdin
//! is a terminal and a display is reachable — blits the same buffer to
//! the screen and feeds ⌥ from the window's keyboard and mouse instead of
//! stdin. A program cannot tell the backends apart except by where its
//! input comes from; the pixels it draws are identical.
//!
//! Text is drawn from a baked grayscale glyph strip (font.bin, generated
//! by compiler/font/bake.py), so rendering is identical on every platform
//! — no runtime font stack, no OS text APIs.

use crate::values::Value;
use std::io::Write;
use std::sync::OnceLock;

// ── the baked font ─────────────────────────────────────────────────────

static FONT_BIN: &[u8] = include_bytes!("font.bin");
static FONT: OnceLock<Font> = OnceLock::new();

pub struct Font {
    pub cell_w: usize,
    pub cell_h: usize,
    codepoints: Vec<u32>,
    alpha: Vec<u8>,
}

pub fn font() -> &'static Font {
    FONT.get_or_init(|| Font::parse(FONT_BIN))
}

impl Font {
    fn parse(b: &[u8]) -> Font {
        assert_eq!(&b[..4], b"MFNT", "font.bin is corrupt");
        let (cell_w, cell_h) = (b[5] as usize, b[6] as usize);
        let count = u32::from_le_bytes([b[8], b[9], b[10], b[11]]) as usize;
        let cell = cell_w * cell_h;
        let mut codepoints = Vec::with_capacity(count);
        let mut alpha = Vec::with_capacity(count * cell);
        let mut off = 12;
        for _ in 0..count {
            codepoints.push(u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]]));
            alpha.extend_from_slice(&b[off + 4..off + 4 + cell]);
            off += 4 + cell;
        }
        Font { cell_w, cell_h, codepoints, alpha }
    }

    fn glyph(&self, c: char) -> Option<&[u8]> {
        let i = self.codepoints.binary_search(&(c as u32)).ok()?;
        let cell = self.cell_w * self.cell_h;
        Some(&self.alpha[i * cell..(i + 1) * cell])
    }
}

// ── the canvas ─────────────────────────────────────────────────────────

pub struct Gui {
    pub w: usize,
    pub h: usize,
    fb: Vec<u32>,
    frame: u32,
    dump_dir: Option<String>,
    #[cfg(feature = "gui")]
    win: Option<win::WinState>,
}

impl Gui {
    /// Open a canvas. A real window appears only when the `gui` feature is
    /// compiled in, nothing forces headless mode, stdin is a terminal, and
    /// the OS can actually open one; otherwise the canvas is headless.
    pub fn open(w: usize, h: usize, title: &str, force_headless: bool) -> Gui {
        let dump_dir = std::env::var("MLANG_FRAMES").ok().filter(|s| !s.is_empty());
        let mut gui = Gui {
            w,
            h,
            fb: vec![0; w * h],
            frame: 0,
            dump_dir,
            #[cfg(feature = "gui")]
            win: None,
        };
        #[cfg(feature = "gui")]
        {
            use std::io::IsTerminal;
            let headless_env = std::env::var("MLANG_HEADLESS")
                .map(|v| !v.is_empty() && v != "0")
                .unwrap_or(false);
            if !force_headless && !headless_env && std::io::stdin().is_terminal() {
                gui.win = win::WinState::open(w, h, title);
            }
        }
        #[cfg(not(feature = "gui"))]
        let _ = (title, force_headless);
        gui
    }

    pub fn is_windowed(&self) -> bool {
        #[cfg(feature = "gui")]
        {
            self.win.is_some()
        }
        #[cfg(not(feature = "gui"))]
        false
    }

    /// Fill a rectangle, clipped to the canvas.
    pub fn rect(&mut self, x: i64, y: i64, rw: i64, rh: i64, color: u32) {
        if rw <= 0 || rh <= 0 {
            return;
        }
        let x0 = x.max(0).min(self.w as i64) as usize;
        let y0 = y.max(0).min(self.h as i64) as usize;
        let x1 = (x + rw).max(0).min(self.w as i64) as usize;
        let y1 = (y + rh).max(0).min(self.h as i64) as usize;
        for row in y0..y1 {
            self.fb[row * self.w + x0..row * self.w + x1].fill(color);
        }
    }

    /// Draw text from the baked font, top-left corner at (x, y). Newlines
    /// wrap to the next glyph row; characters outside the strip draw a
    /// hollow box. Glyph coverage alpha-blends over what is already there.
    pub fn text(&mut self, s: &str, x: i64, y: i64, color: u32) {
        let f = font();
        let (cw, chh) = (f.cell_w as i64, f.cell_h as i64);
        let (mut cx, mut cy) = (x, y);
        for c in s.chars() {
            if c == '\n' {
                cx = x;
                cy += chh;
                continue;
            }
            match f.glyph(c) {
                Some(a) => self.blend_glyph(a, cx, cy, color),
                None => {
                    // a hollow box marks a character the strip lacks
                    self.rect(cx + 1, cy + 3, cw - 2, 1, color);
                    self.rect(cx + 1, cy + chh - 4, cw - 2, 1, color);
                    self.rect(cx + 1, cy + 3, 1, chh - 6, color);
                    self.rect(cx + cw - 2, cy + 3, 1, chh - 6, color);
                }
            }
            cx += cw;
        }
    }

    fn blend_glyph(&mut self, alpha: &[u8], x: i64, y: i64, color: u32) {
        let f = font();
        let (fr, fg, fb) = ((color >> 16) & 0xff, (color >> 8) & 0xff, color & 0xff);
        for gy in 0..f.cell_h {
            let py = y + gy as i64;
            if py < 0 || py >= self.h as i64 {
                continue;
            }
            for gx in 0..f.cell_w {
                let a = alpha[gy * f.cell_w + gx] as u32;
                if a == 0 {
                    continue;
                }
                let px = x + gx as i64;
                if px < 0 || px >= self.w as i64 {
                    continue;
                }
                let i = py as usize * self.w + px as usize;
                let d = self.fb[i];
                let (dr, dg, db) = ((d >> 16) & 0xff, (d >> 8) & 0xff, d & 0xff);
                let r = (fr * a + dr * (255 - a)) / 255;
                let g = (fg * a + dg * (255 - a)) / 255;
                let b = (fb * a + db * (255 - a)) / 255;
                self.fb[i] = (r << 16) | (g << 8) | b;
            }
        }
    }

    /// Present the frame: blit to the window, or (headless) print a frame
    /// hash so recorded runs pin the pixels. MLANG_FRAMES=<dir> also dumps
    /// the frame as a PPM either way.
    pub fn present(&mut self, out: &mut dyn Write) -> Result<(), String> {
        self.frame += 1;
        if let Some(dir) = self.dump_dir.clone() {
            self.dump_ppm(&dir);
        }
        #[cfg(feature = "gui")]
        if let Some(ws) = &mut self.win {
            return ws
                .window
                .update_with_buffer(&self.fb, self.w, self.h)
                .map_err(|e| format!("cannot present: {e}"));
        }
        let _ = writeln!(out, "⎙ {} #{:016x}", self.frame, self.hash());
        Ok(())
    }

    /// FNV-1a over the RGB bytes, row-major — the frame's identity in a
    /// recorded golden.
    fn hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for &px in &self.fb {
            for b in [(px >> 16) as u8, (px >> 8) as u8, px as u8] {
                h ^= b as u64;
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        h
    }

    fn dump_ppm(&self, dir: &str) {
        let _ = std::fs::create_dir_all(dir);
        let mut buf = Vec::with_capacity(self.w * self.h * 3 + 32);
        buf.extend_from_slice(format!("P6\n{} {}\n255\n", self.w, self.h).as_bytes());
        for &px in &self.fb {
            buf.extend_from_slice(&[(px >> 16) as u8, (px >> 8) as u8, px as u8]);
        }
        let _ = std::fs::write(format!("{dir}/frame-{:03}.ppm", self.frame), buf);
    }

    /// Block until the window produces an input event (⌥'s windowed
    /// source). ∅ once the window is closed. Headless canvases never call
    /// this — ⌥ keeps reading stdin there.
    pub fn wait_event(&mut self) -> Value {
        #[cfg(feature = "gui")]
        if let Some(ws) = &mut self.win {
            return ws.wait_event();
        }
        Value::Nil
    }
}

// ── the windowed backend ───────────────────────────────────────────────

#[cfg(feature = "gui")]
mod win {
    use crate::values::Value;
    use minifb::{InputCallback, Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    /// Characters typed into the window, queued by minifb's input callback.
    struct Chars(Arc<Mutex<VecDeque<u32>>>);

    impl InputCallback for Chars {
        fn add_char(&mut self, uni_char: u32) {
            self.0.lock().unwrap().push_back(uni_char);
        }
    }

    pub struct WinState {
        pub window: Window,
        chars: Arc<Mutex<VecDeque<u32>>>,
        mouse_was_down: bool,
        pending: VecDeque<Value>,
    }

    impl WinState {
        pub fn open(w: usize, h: usize, title: &str) -> Option<WinState> {
            let mut window = Window::new(title, w, h, WindowOptions::default()).ok()?;
            let chars = Arc::new(Mutex::new(VecDeque::new()));
            window.set_input_callback(Box::new(Chars(chars.clone())));
            Some(WinState { window, chars, mouse_was_down: false, pending: VecDeque::new() })
        }

        /// Block until an event arrives, pumping the window meanwhile.
        /// The mapping mirrors the terminal's ⌥ events exactly — same
        /// strings, same ⟨«⌖» x y⟩ shape (pixel coordinates here) — so an
        /// editor's dispatch runs unchanged against either backend.
        pub fn wait_event(&mut self) -> Value {
            loop {
                if let Some(e) = self.pending.pop_front() {
                    return e;
                }
                if !self.window.is_open() {
                    return Value::Nil;
                }
                self.window.update();
                self.collect();
                if self.pending.is_empty() {
                    std::thread::sleep(std::time::Duration::from_millis(4));
                }
            }
        }

        fn collect(&mut self) {
            let ctrl = self.window.is_key_down(Key::LeftCtrl)
                || self.window.is_key_down(Key::RightCtrl);
            {
                // printable characters come from the input callback; control
                // characters are covered by the key events below, and a held
                // Ctrl means the key arrives as a chord, not a character
                let mut q = self.chars.lock().unwrap();
                while let Some(u) = q.pop_front() {
                    if u >= 32 && u != 127 && !ctrl {
                        if let Some(c) = char::from_u32(u) {
                            self.pending.push_back(Value::str(c.to_string()));
                        }
                    }
                }
            }
            for k in self.window.get_keys_pressed(KeyRepeat::Yes) {
                let special = match k {
                    Key::Up => Some("↑"),
                    Key::Down => Some("↓"),
                    Key::Left => Some("←"),
                    Key::Right => Some("→"),
                    Key::Home => Some("⇱"),
                    Key::End => Some("⇲"),
                    Key::PageUp => Some("⇞"),
                    Key::PageDown => Some("⇟"),
                    Key::Delete => Some("⌦"),
                    Key::Insert => Some("⎀"),
                    Key::Enter => Some("↵"),
                    Key::Backspace => Some("⌫"),
                    Key::Tab => Some("⇥"),
                    Key::Escape => Some("⎋"),
                    _ => None,
                };
                if let Some(e) = special {
                    self.pending.push_back(Value::str(e));
                } else if ctrl {
                    if let Some(letter) = key_letter(k) {
                        self.pending.push_back(Value::str(format!("^{letter}")));
                    }
                }
            }
            let down = self.window.get_mouse_down(MouseButton::Left);
            if down && !self.mouse_was_down {
                if let Some((x, y)) = self.window.get_mouse_pos(MouseMode::Discard) {
                    self.pending.push_back(Value::List(std::sync::Arc::new(vec![
                        Value::str("⌖"),
                        Value::int(x as i64),
                        Value::int(y as i64),
                    ])));
                }
            }
            self.mouse_was_down = down;
        }
    }

    fn key_letter(k: Key) -> Option<char> {
        let n = k as usize;
        let a = Key::A as usize;
        if (a..a + 26).contains(&n) {
            Some((b'A' + (n - a) as u8) as char)
        } else {
            None
        }
    }
}
