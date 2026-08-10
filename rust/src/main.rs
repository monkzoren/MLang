//! mlang — the native MLang engine.

mod forms;
mod lex;
mod values;
mod vm;

use lex::LoadError;
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

fn run_source(text: &str) -> ExitCode {
    let prog = match forms::parse_source(text) {
        Ok(p) => p,
        Err(e) => return weave_error(&e),
    };
    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut out = std::io::stdout();
    let mut err = std::io::stderr();
    let code = {
        let mut machine = vm::VM::new(&mut reader, &mut out, &mut err);
        match machine.run_program(&prog) {
            Ok(c) => c,
            Err(e) => return weave_error(&e),
        }
    };
    let _ = out.flush();
    ExitCode::from(code as u8)
}

const USAGE: &str = "mlang — the Matrix language (native engine)

usage:
  mlang run <file|->     run a program (.ml, rain or flat form)
  mlang eval <code>      run flat-form source given as an argument
  mlang rain <file|->    render flat source as the vertical rain grid
  mlang flat <file|->    render rain source as flat lines
  mlang ops              print the sigil reference table
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let (cmd, arg) = match args.len() {
        2 if args[1] == "ops" => (args[1].as_str(), ""),
        3 => (args[1].as_str(), args[2].as_str()),
        _ => {
            eprint!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match cmd {
        "run" => match read_source(arg) {
            Ok(text) => run_source(&text),
            Err(e) => {
                eprintln!("✗ {e}");
                ExitCode::from(2)
            }
        },
        "eval" => run_source(arg),
        "rain" | "flat" => {
            let text = match read_source(arg) {
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
        "ops" => {
            print!("{}", include_str!("ops.txt"));
            ExitCode::SUCCESS
        }
        _ => {
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
