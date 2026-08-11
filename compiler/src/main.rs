//! mlang — the MLang toolchain: compiler, runner, and runtime.
//!
//! This one binary is the whole toolchain. `mlang build` copies its own
//! runtime image and welds the compiled program into it, producing a
//! standalone native executable. A welded binary detects its payload at
//! startup and runs it directly — it never behaves as a compiler.

use mlang::lex::LoadError;
use mlang::{forms, payload, vm};
use std::io::{BufReader, Read, Write};
use std::process::ExitCode;

fn read_source(path: &str) -> Result<String, String> {
    if path == "-" {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| e.to_string())?;
        Ok(buf)
    } else {
        std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))
    }
}

fn weave_error(e: &LoadError) -> ExitCode {
    let loc = match e.pos {
        Some((r, c)) => format!(" at {r}:{c}"),
        None => String::new(),
    };
    eprintln!("✗ weave error{loc}: {}", e.msg);
    ExitCode::from(2)
}

/// Puts a real terminal into an interactive session for programs that
/// use ⌥: raw input, SGR mouse reporting, the alternate screen, hidden
/// cursor. Restores everything on drop, so a glitch exit cleans up too.
/// Does nothing when stdin/stdout are pipes — recorded runs see only
/// the program's own bytes.
struct TerminalSession {
    active: bool,
    #[cfg(unix)]
    saved: Option<libc::termios>,
    #[cfg(windows)]
    saved: Option<(u32, u32)>,
}

impl TerminalSession {
    fn start(prog: &vm::CompiledProgram) -> Self {
        use std::io::IsTerminal;
        let wanted = vm::uses_interactive(prog)
            && std::io::stdin().is_terminal()
            && std::io::stdout().is_terminal();
        let mut session = TerminalSession {
            active: false,
            saved: None,
        };
        if wanted && session.enter_raw() {
            session.active = true;
            print!("\x1b[?1049h\x1b[?25l\x1b[?1000;1006h\x1b[2J\x1b[H");
            let _ = std::io::stdout().flush();
        }
        session
    }

    #[cfg(unix)]
    fn enter_raw(&mut self) -> bool {
        unsafe {
            let mut t: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(libc::STDIN_FILENO, &mut t) != 0 {
                return false;
            }
            self.saved = Some(t);
            libc::cfmakeraw(&mut t);
            // keep output post-processing so \n still moves to column 0
            t.c_oflag |= libc::OPOST;
            libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &t) == 0
        }
    }

    #[cfg(unix)]
    fn leave_raw(&mut self) {
        if let Some(t) = self.saved.take() {
            unsafe {
                libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &t);
            }
        }
    }

    #[cfg(windows)]
    fn enter_raw(&mut self) -> bool {
        use windows_sys::Win32::System::Console::*;
        unsafe {
            let hin = GetStdHandle(STD_INPUT_HANDLE);
            let hout = GetStdHandle(STD_OUTPUT_HANDLE);
            let (mut min, mut mout) = (0u32, 0u32);
            if GetConsoleMode(hin, &mut min) == 0 || GetConsoleMode(hout, &mut mout) == 0 {
                return false;
            }
            self.saved = Some((min, mout));
            let raw_in = (min & !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT))
                | ENABLE_VIRTUAL_TERMINAL_INPUT;
            let vt_out = mout | ENABLE_VIRTUAL_TERMINAL_PROCESSING;
            SetConsoleMode(hin, raw_in) != 0 && SetConsoleMode(hout, vt_out) != 0
        }
    }

    #[cfg(windows)]
    fn leave_raw(&mut self) {
        use windows_sys::Win32::System::Console::*;
        if let Some((min, mout)) = self.saved.take() {
            unsafe {
                SetConsoleMode(GetStdHandle(STD_INPUT_HANDLE), min);
                SetConsoleMode(GetStdHandle(STD_OUTPUT_HANDLE), mout);
            }
        }
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if self.active {
            print!("\x1b[?1000;1006l\x1b[?25h\x1b[?1049l");
            let _ = std::io::stdout().flush();
            self.leave_raw();
        }
    }
}

fn run_compiled(prog: &vm::CompiledProgram, prog_args: Vec<String>) -> ExitCode {
    let session = TerminalSession::start(prog);
    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut out = std::io::stdout();
    let mut err = std::io::stderr();
    let code = {
        let mut machine = vm::VM::new(&mut reader, &mut out, &mut err);
        machine.args = prog_args;
        machine.run_compiled(prog)
    };
    let _ = out.flush();
    drop(session);
    ExitCode::from(code as u8)
}

fn run_source(text: &str, prog_args: Vec<String>) -> ExitCode {
    match vm::compile_text(text) {
        Ok(prog) => run_compiled(&prog, prog_args),
        Err(e) => weave_error(&e),
    }
}

fn build(src_path: &str, out_path: &str) -> ExitCode {
    let text = match read_source(src_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("✗ {e}");
            return ExitCode::from(2);
        }
    };
    let prog = match vm::compile_text(&text) {
        Ok(p) => p,
        Err(e) => return weave_error(&e),
    };
    let exe = match std::env::current_exe().and_then(std::fs::read) {
        Ok(image) => image,
        Err(e) => {
            eprintln!("✗ cannot read the runtime image: {e}");
            return ExitCode::from(2);
        }
    };
    let image = payload::weld(&exe, &prog);
    if let Err(e) = std::fs::write(out_path, image) {
        eprintln!("✗ {out_path}: {e}");
        return ExitCode::from(2);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(out_path, std::fs::Permissions::from_mode(0o755));
    }
    let strands = prog.strands.len();
    eprintln!("⇓ wove {src_path} → {out_path} ({strands} strand{})",
              if strands == 1 { "" } else { "s" });
    ExitCode::SUCCESS
}

const USAGE: &str = "mlang — the Matrix language toolchain

usage:
  mlang build <file|-> -o <out>   compile to a standalone native executable
  mlang run <file|-> [args…]      compile and run immediately (args reach ⌂)
  mlang eval <code> [args…]       run flat-form source given as an argument
  mlang check <file|->            compile only; report weave errors
  mlang rain <file|->             render flat source as the vertical rain grid
  mlang flat <file|->             render rain source as flat lines
  mlang ops                       print the sigil reference table
  mlang std                       print the standard library source
  mlang ui                        print the Construct, the UI library source
";

fn main() -> ExitCode {
    // A welded binary runs its embedded program — it is not a compiler.
    // Its command-line arguments belong to that program (⌂), which is what
    // lets a welded editor open a file dropped onto the executable.
    if let Some(extracted) = payload::self_payload() {
        return match extracted {
            Ok(prog) => run_compiled(&prog, std::env::args().skip(1).collect()),
            Err(e) => {
                eprintln!("✗ corrupt program payload: {e}");
                ExitCode::from(2)
            }
        };
    }

    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("");
    match (cmd, args.len()) {
        ("build", 5) if args[3] == "-o" => build(&args[2], &args[4]),
        ("run", n) if n >= 3 => match read_source(&args[2]) {
            Ok(text) => run_source(&text, args[3..].to_vec()),
            Err(e) => {
                eprintln!("✗ {e}");
                ExitCode::from(2)
            }
        },
        ("eval", n) if n >= 3 => run_source(&args[2], args[3..].to_vec()),
        ("check", 3) => match read_source(&args[2]) {
            Ok(text) => match vm::compile_text(&text) {
                Ok(prog) => {
                    eprintln!("✓ weaves clean ({} strands)", prog.strands.len());
                    ExitCode::SUCCESS
                }
                Err(e) => weave_error(&e),
            },
            Err(e) => {
                eprintln!("✗ {e}");
                ExitCode::from(2)
            }
        },
        ("rain", 3) | ("flat", 3) => {
            let text = match read_source(&args[2]) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("✗ {e}");
                    return ExitCode::from(2);
                }
            };
            let rendered = if cmd == "rain" {
                forms::to_rain(&text)
            } else {
                forms::to_flat(&text)
            };
            match rendered {
                Ok(s) => {
                    print!("{s}");
                    ExitCode::SUCCESS
                }
                Err(e) => weave_error(&e),
            }
        }
        ("ops", 2) => {
            print!("{}", include_str!("ops.txt"));
            ExitCode::SUCCESS
        }
        ("std", 2) => {
            print!("{}", vm::STD_SOURCE);
            ExitCode::SUCCESS
        }
        ("ui", 2) => {
            print!("{}", vm::UI_SOURCE);
            ExitCode::SUCCESS
        }
        _ => {
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
