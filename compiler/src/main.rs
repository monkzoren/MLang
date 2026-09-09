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

fn weave_error(text: &str, e: &LoadError) -> ExitCode {
    let loc = match e.pos {
        Some((r, c)) => format!(" at {r}:{c}"),
        None => String::new(),
    };
    eprintln!("✗ weave error{loc}: {}", e.msg);
    if let Some(pos) = e.pos {
        let lines: Vec<String> = text.lines().map(String::from).collect();
        if let Some(x) = vm::excerpt(&lines, pos) {
            eprintln!("{x}");
        }
    }
    ExitCode::from(2)
}

/// Print a reference text to stdout through a locked handle, ignoring a
/// broken pipe: `mlang ops | head -1` closes our stdout early, and that
/// is the reader's business, not a panic (the exit status stays 0).
fn emit(text: &str) -> ExitCode {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(text.as_bytes()).and_then(|_| out.flush());
    ExitCode::SUCCESS
}

/// The one port shape `mlang serve` and MLANG_PORT accept: plain digits
/// in 0…65535 (0 lets the OS choose). Err carries the offending text so
/// the caller can name it.
fn parse_port(text: &str) -> Result<u16, String> {
    let digits = !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
    match text.parse::<u16>() {
        Ok(p) if digits => Ok(p),
        _ => Err(text.to_string()),
    }
}

/// argv as strings, lossily: a non-UTF-8 argument (a file name from an
/// odd filesystem) reaches ⌂ with replacement characters rather than
/// crashing the toolchain before it starts.
fn argv() -> Vec<String> {
    std::env::args_os()
        .map(|a| a.to_string_lossy().into_owned())
        .collect()
}

/// MLANG_PAR=1 selects the parallel scheduler (strands on OS threads) —
/// the only switch a welded binary has, since its argv belongs to ⌂.
fn parallel_env() -> bool {
    std::env::var("MLANG_PAR")
        .map(|v| !v.is_empty() && v != "0")
        .unwrap_or(false)
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
        // A canvas program's window owns the input — leave the terminal
        // in its normal mode instead of switching to the raw alt-screen.
        let wanted = vm::uses_interactive(prog)
            && !vm::uses_gui(prog)
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

fn run_compiled(
    prog: &vm::CompiledProgram,
    prog_args: Vec<String>,
    parallel: bool,
    http: Option<std::sync::Arc<mlang::http::HttpBridge>>,
) -> ExitCode {
    let session = TerminalSession::start(prog);
    if parallel || parallel_env() {
        let code = mlang::par::run_parallel(prog, prog_args, http);
        drop(session); // restore the terminal before the process exits
        return ExitCode::from(code as u8);
    }
    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut out = std::io::stdout();
    let mut err = std::io::stderr();
    let code = {
        let mut machine = vm::VM::new(&mut reader, &mut out, &mut err);
        machine.args = prog_args;
        // A served program opens its loom (SPEC §4.7) unless MLANG_LOOM=0:
        // the bridge serves the version store, the VM patches it.
        if let Some(bridge) = &http {
            if !loom_disabled() {
                let loom = mlang::loom::Loom::new(&(prog.source.join("\n") + "\n"));
                bridge.attach_loom(loom.clone());
                eprintln!("⟡ the loom is open at http://127.0.0.1:{}/.loom", bridge.port);
                machine.loom = Some(loom);
            }
        }
        machine.http = http;
        machine.run_compiled(prog)
    };
    let _ = out.flush();
    drop(session);
    ExitCode::from(code as u8)
}

fn run_source(text: &str, prog_args: Vec<String>, parallel: bool) -> ExitCode {
    match vm::compile_text(text) {
        Ok(prog) => run_compiled(&prog, prog_args, parallel, None),
        Err(e) => weave_error(text, &e),
    }
}

/// MLANG_LOOM=0 keeps a served program's source private: no /.loom routes.
fn loom_disabled() -> bool {
    std::env::var("MLANG_LOOM").map(|v| v == "0").unwrap_or(false)
}

/// Normalize what `pull`/`patch`/`loom` accept as a server: a full URL,
/// `host:port`, or a bare port on the loopback.
fn loom_url(arg: &str) -> String {
    let base = if arg.starts_with("http://") || arg.starts_with("https://") {
        arg.to_string()
    } else if arg.chars().all(|c| c.is_ascii_digit()) {
        format!("http://127.0.0.1:{arg}")
    } else {
        format!("http://{arg}")
    };
    format!("{}/.loom", base.trim_end_matches('/'))
}

fn loom_client() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(30))
        .build()
}

/// A loom answer: the body, whatever the status — the text is the report.
fn loom_body(r: Result<ureq::Response, ureq::Error>) -> Result<(u16, String), String> {
    match r {
        Ok(resp) => {
            let status = resp.status();
            resp.into_string().map(|b| (status, b)).map_err(|e| e.to_string())
        }
        Err(ureq::Error::Status(code, resp)) => {
            Ok((code, resp.into_string().unwrap_or_default()))
        }
        Err(ureq::Error::Transport(t)) => Err(t.to_string()),
    }
}

/// `mlang pull <server>`: print the live source, stamped with its version
/// and origin, ready to edit and `mlang patch` back.
fn pull(server: &str) -> ExitCode {
    let url = loom_url(server);
    match loom_body(loom_client().get(&url).call()) {
        Ok((200, body)) => {
            let (v, _, _) = mlang::loom::unstamp(&body);
            eprintln!("⟡ pulled v{} from {url}", v.unwrap_or(0));
            emit(&body)
        }
        Ok((status, body)) => {
            eprint!("✗ {url} answered {status}: {body}");
            ExitCode::from(2)
        }
        Err(e) => {
            eprintln!("✗ cannot reach {url}: {e}");
            ExitCode::from(2)
        }
    }
}

/// `mlang patch <file> [server] [--base N]`: send a pulled-and-edited
/// file back to the grid it came from. The stamp `pull` wrote names the
/// server and the base version; both can be overridden. The server's
/// report is printed; a rejected patch (merge conflict, weave error,
/// or boot code changed) exits 1 with the reason.
fn patch(rest: &[String]) -> ExitCode {
    let mut file = None;
    let mut server = None;
    let mut base: Option<usize> = None;
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        if a == "--base" {
            base = it.next().and_then(|n| n.parse().ok());
            if base.is_none() {
                eprintln!("✗ --base wants a version number");
                return ExitCode::from(2);
            }
        } else if file.is_none() {
            file = Some(a.clone());
        } else {
            server = Some(a.clone());
        }
    }
    let Some(file) = file else {
        eprintln!("✗ patch wants the form: mlang patch <file> [server] [--base N]");
        return ExitCode::from(2);
    };
    let text = match read_source(&file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("✗ {e}");
            return ExitCode::from(2);
        }
    };
    let (stamped, origin, body) = mlang::loom::unstamp(&text);
    let base = match base.or(stamped) {
        Some(b) => b,
        None => {
            eprintln!("✗ {file} carries no loom stamp — pull it from the server first, or pass --base N");
            return ExitCode::from(2);
        }
    };
    let url = match server.as_deref().or(Some(origin).filter(|o| !o.is_empty())) {
        Some(s) => loom_url(s),
        None => {
            eprintln!("✗ which grid? name the server: mlang patch {file} 4321");
            return ExitCode::from(2);
        }
    };
    // Weave locally first: a broken file never reaches the grid. The ⟲
    // migration lines are the loom's, not the program's — set aside.
    let (program, _) = mlang::loom::split_migrations(body);
    if let Err(e) = vm::compile_text(&program) {
        return weave_error(&program, &e);
    }
    let stamped = mlang::loom::stamp(base, &url, body);
    match loom_body(loom_client().post(&url).set("Content-Type", "text/plain; charset=utf-8").send_string(&stamped)) {
        Ok((200, report)) => {
            print!("{report}");
            ExitCode::SUCCESS
        }
        Ok((status, why)) => {
            eprint!("{why}");
            if !why.ends_with('\n') {
                eprintln!();
            }
            eprintln!("✗ patch not applied ({status})");
            ExitCode::from(1)
        }
        Err(e) => {
            eprintln!("✗ cannot reach {url}: {e}");
            ExitCode::from(2)
        }
    }
}

/// `mlang loom <server>`: the version log of a served grid.
fn loom_log(server: &str) -> ExitCode {
    let url = format!("{}/log", loom_url(server));
    match loom_body(loom_client().get(&url).call()) {
        Ok((200, body)) => emit(&body),
        Ok((status, body)) => {
            eprint!("✗ {url} answered {status}: {body}");
            ExitCode::from(2)
        }
        Err(e) => {
            eprintln!("✗ cannot reach {url}: {e}");
            ExitCode::from(2)
        }
    }
}

/// `mlang serve` (and MLANG_PORT for welded binaries): start the live web
/// listener and announce it, then run the program against it.
fn start_bridge(port: u16) -> Result<std::sync::Arc<mlang::http::HttpBridge>, ExitCode> {
    match mlang::http::HttpBridge::start(port) {
        Ok(bridge) => {
            eprintln!("⇓ the grid is listening on http://127.0.0.1:{}", bridge.port);
            Ok(bridge)
        }
        Err(e) => {
            eprintln!("✗ cannot listen on port {port}: {e}");
            Err(ExitCode::from(2))
        }
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
        Err(e) => return weave_error(&text, &e),
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
  mlang run [--parallel] <file|-> [args…]   compile and run (args reach ⌂)
  mlang eval [--parallel] <code> [args…]    run flat-form source directly
                                  --parallel (or MLANG_PAR=1, which welded
                                  binaries also honor): one OS thread per
                                  strand instead of the deterministic
                                  round-robin scheduler
  mlang serve [--parallel] <file> [port] [args…]   run with a live web
                                  listener for ⎆/⍅ (default port 4321;
                                  MLANG_PORT does the same for a welded
                                  binary — without it, ⎆ replays request
                                  frames from stdin)
  mlang check <file|->            compile only; report weave errors
  mlang pull <server>             print a served grid's live source, stamped
                                  with its version (server: URL, host:port,
                                  or a bare port on 127.0.0.1)
  mlang patch <file> [server] [--base N]   weave an edited pull back into
                                  the running grid — no restart (SPEC §4.7);
                                  the stamp names the server and base version
  mlang loom <server>             the served grid's version log
  mlang hub [--listen A:P] [--workers N] <file|-> [args…]
                                  run a program with its work channel (α)
                                  distributed over TCP to joined workers and
                                  its results channel (β) fed by them
  mlang worker [--connect A:P] <file|-> [args…]
                                  join a hub: work arrives on α, sends to β
                                  return to the hub  (--work G / --results G
                                  rename the bridged channels on both sides)
  mlang rain <file|->             render flat source as the vertical rain grid
  mlang flat <file|->             render rain source as flat lines
  mlang ops                       print the sigil reference table
  mlang std                       print the standard library source
  mlang ui                        print the Construct, the UI library source
  mlang json                      print the Operator, the JSON library source
";

/// Parse and dispatch `mlang hub` / `mlang worker`. Flags come before the
/// source file; everything after the file belongs to the program (⌂).
fn net_cmd(hub: bool, rest: &[String]) -> ExitCode {
    let mut listen = "0.0.0.0:7777".to_string();
    let mut connect = "127.0.0.1:7777".to_string();
    let mut min_workers = 1usize;
    let mut opts = mlang::net::NetOpts::default();
    let mut file: Option<String> = None;
    let mut prog_args: Vec<String> = Vec::new();

    let glyph = |flag: &str, s: &str| -> Result<char, String> {
        let mut chars = s.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => Ok(c),
            _ => Err(format!("{flag} wants a single glyph, got {s:?}")),
        }
    };
    let mut i = 0;
    while i < rest.len() {
        let arg = &rest[i];
        if file.is_some() {
            prog_args.push(arg.clone());
            i += 1;
            continue;
        }
        let mut flag_value = |flag: &str| -> Result<String, String> {
            i += 1;
            rest.get(i).cloned().ok_or(format!("{flag} wants a value"))
        };
        let outcome = match arg.as_str() {
            "--listen" if hub => flag_value("--listen").map(|v| listen = v),
            "--connect" if !hub => flag_value("--connect").map(|v| connect = v),
            "--workers" if hub => flag_value("--workers").and_then(|v| {
                v.parse()
                    .map(|n| min_workers = n)
                    .map_err(|_| format!("--workers wants a count, got {v:?}"))
            }),
            "--work" => flag_value("--work").and_then(|v| glyph("--work", &v).map(|g| opts.work = g)),
            "--results" => {
                flag_value("--results").and_then(|v| glyph("--results", &v).map(|g| opts.results = g))
            }
            _ => {
                file = Some(arg.clone());
                Ok(())
            }
        };
        if let Err(e) = outcome {
            eprintln!("✗ {e}");
            return ExitCode::from(2);
        }
        i += 1;
    }
    if opts.work == opts.results {
        eprintln!("✗ the work and results channels must be different glyphs");
        return ExitCode::from(2);
    }
    let Some(file) = file else {
        eprint!("{USAGE}");
        return ExitCode::from(2);
    };
    let text = match read_source(&file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("✗ {e}");
            return ExitCode::from(2);
        }
    };
    let prog = match vm::compile_text(&text) {
        Ok(p) => p,
        Err(e) => return weave_error(&text, &e),
    };
    let code = if hub {
        mlang::net::run_hub(&prog, prog_args, &listen, min_workers, opts)
    } else {
        mlang::net::run_worker(&prog, prog_args, &connect, opts)
    };
    ExitCode::from(code as u8)
}

fn main() -> ExitCode {
    // A welded binary runs its embedded program — it is not a compiler.
    // Its command-line arguments belong to that program (⌂), which is what
    // lets a welded editor open a file dropped onto the executable.
    if let Some(extracted) = payload::self_payload() {
        return match extracted {
            Ok(prog) => {
                // MLANG_PORT turns a welded server binary live; anything
                // else (or nothing) runs in replay mode.
                let wanted = std::env::var_os("MLANG_PORT")
                    .map(|p| p.to_string_lossy().into_owned())
                    .filter(|p| !p.is_empty());
                let http = match wanted.as_deref().map(parse_port) {
                    Some(Ok(port)) => match start_bridge(port) {
                        Ok(bridge) => Some(bridge),
                        Err(code) => return code,
                    },
                    Some(Err(_)) => {
                        eprintln!("✗ MLANG_PORT must be a port number");
                        return ExitCode::from(2);
                    }
                    None => None,
                };
                run_compiled(&prog, argv().into_iter().skip(1).collect(), false, http)
            }
            Err(e) => {
                eprintln!("✗ corrupt program payload: {e}");
                ExitCode::from(2)
            }
        };
    }

    let args = argv();
    let cmd = args.get(1).map(String::as_str).unwrap_or("");
    match (cmd, args.len()) {
        ("build", 5) if args[3] == "-o" => build(&args[2], &args[4]),
        ("build", _) => {
            eprintln!("✗ build wants the form: mlang build <src> -o <out>");
            ExitCode::from(2)
        }
        ("run", n) if n >= 3 => {
            let par = args[2] == "--parallel";
            let file = if par { args.get(3) } else { Some(&args[2]) };
            let rest = if par { 4 } else { 3 };
            match file {
                Some(f) => match read_source(f) {
                    Ok(text) => run_source(&text, args.get(rest..).unwrap_or(&[]).to_vec(), par),
                    Err(e) => {
                        eprintln!("✗ {e}");
                        ExitCode::from(2)
                    }
                },
                None => {
                    eprint!("{USAGE}");
                    ExitCode::from(2)
                }
            }
        }
        ("serve", n) if n >= 3 => {
            let par = args[2] == "--parallel";
            let fi = if par { 3 } else { 2 };
            let Some(file) = args.get(fi) else {
                eprint!("{USAGE}");
                return ExitCode::from(2);
            };
            let text = match read_source(file) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("✗ {e}");
                    return ExitCode::from(2);
                }
            };
            let prog = match vm::compile_text(&text) {
                Ok(p) => p,
                Err(e) => return weave_error(&text, &e),
            };
            // An optional port follows the file; anything after it — or a
            // first argument that does not start with a digit — belongs
            // to ⌂. Something that *looks* like a port but is not one
            // (70000, 4321x) is a usage error, never silently demoted to
            // a program argument.
            let (port, rest) = match args.get(fi + 1) {
                Some(p) if p.starts_with(|c: char| c.is_ascii_digit()) => match parse_port(p) {
                    Ok(port) => (port, fi + 2),
                    Err(bad) => {
                        eprintln!("✗ port must be a number 0…65535, got «{bad}»");
                        return ExitCode::from(2);
                    }
                },
                _ => (4321, fi + 1),
            };
            let bridge = match start_bridge(port) {
                Ok(b) => b,
                Err(code) => return code,
            };
            run_compiled(
                &prog,
                args.get(rest..).unwrap_or(&[]).to_vec(),
                par,
                Some(bridge),
            )
        }
        ("eval", n) if n >= 3 => {
            let par = args[2] == "--parallel";
            let (code, rest) = if par { (args.get(3), 4) } else { (Some(&args[2]), 3) };
            match code {
                Some(c) => run_source(c, args.get(rest..).unwrap_or(&[]).to_vec(), par),
                None => {
                    eprint!("{USAGE}");
                    ExitCode::from(2)
                }
            }
        }
        ("hub", n) if n >= 3 => net_cmd(true, &args[2..]),
        ("worker", n) if n >= 3 => net_cmd(false, &args[2..]),
        ("pull", 3) => pull(&args[2]),
        ("patch", n) if n >= 3 => patch(&args[2..]),
        ("loom", 3) => loom_log(&args[2]),
        ("check", 3) => match read_source(&args[2]) {
            Ok(text) => match vm::compile_text(&text) {
                Ok(prog) => {
                    eprintln!("✓ weaves clean ({} strands)", prog.strands.len());
                    ExitCode::SUCCESS
                }
                Err(e) => weave_error(&text, &e),
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
                Ok(s) => emit(&s),
                Err(e) => weave_error(&text, &e),
            }
        }
        ("ops", 2) => emit(include_str!("ops.txt")),
        ("std", 2) => emit(vm::STD_SOURCE),
        ("ui", 2) => emit(vm::UI_SOURCE),
        ("json", 2) => emit(vm::JSON_SOURCE),
        _ => {
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
