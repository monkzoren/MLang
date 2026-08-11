//! The MLang toolchain library: parser, compiler, runtime, and the
//! payload format used to weld compiled programs into native binaries.

pub mod forms;
pub mod gui;
pub mod lex;
pub mod par;
pub mod payload;
pub mod term;
pub mod values;
pub mod vm;

use std::io::Cursor;

/// Compile and run MLang source with captured I/O.
/// Returns (exit_code, stdout, stderr). Exit 2 = weave (load) error.
pub fn run_text(text: &str, stdin_text: &str) -> (i32, String, String) {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = match vm::compile_text(text) {
        Ok(prog) => {
            let mut stdin = Cursor::new(stdin_text.as_bytes().to_vec());
            let mut machine = vm::VM::new(&mut stdin, &mut out, &mut err);
            machine.force_headless = true; // recorded runs never open a window
            machine.run_compiled(&prog)
        }
        Err(e) => {
            let loc = match e.pos {
                Some((r, c)) => format!(" at {r}:{c}"),
                None => String::new(),
            };
            err.extend_from_slice(format!("✗ weave error{loc}: {}\n", e.msg).as_bytes());
            2
        }
    };
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}
