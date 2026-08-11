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

/// MLANG_PAR=1 selects the parallel scheduler (strands on OS threads) —
/// the only switch a welded binary has, since its argv belongs to ⌂.
fn parallel_env() -> bool {
    std::env::var("MLANG_PAR")
        .map(|v| !v.is_empty() && v != "0")
        .unwrap_or(false)
}

fn run_compiled(prog: &vm::CompiledProgram, prog_args: Vec<String>, parallel: bool) -> ExitCode {
    if parallel || parallel_env() {
        return ExitCode::from(mlang::par::run_parallel(prog, prog_args) as u8);
    }
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
    ExitCode::from(code as u8)
}

fn run_source(text: &str, prog_args: Vec<String>, parallel: bool) -> ExitCode {
    match vm::compile_text(text) {
        Ok(prog) => run_compiled(&prog, prog_args, parallel),
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
  mlang run [--parallel] <file|-> [args…]   compile and run (args reach ⌂)
  mlang eval [--parallel] <code> [args…]    run flat-form source directly
                                  --parallel (or MLANG_PAR=1, which welded
                                  binaries also honor): one OS thread per
                                  strand instead of the deterministic
                                  round-robin scheduler
  mlang check <file|->            compile only; report weave errors
  mlang rain <file|->             render flat source as the vertical rain grid
  mlang flat <file|->             render rain source as flat lines
  mlang ops                       print the sigil reference table
  mlang std                       print the standard library source
";

fn main() -> ExitCode {
    // A welded binary runs its embedded program — it is not a compiler.
    // Its command-line arguments belong to that program (⌂), which is what
    // lets a welded editor open a file dropped onto the executable.
    if let Some(extracted) = payload::self_payload() {
        return match extracted {
            Ok(prog) => run_compiled(&prog, std::env::args().skip(1).collect(), false),
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
        _ => {
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
