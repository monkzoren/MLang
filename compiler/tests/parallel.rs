//! The parallel scheduler versus the deterministic engine.
//!
//! Programs whose channels are single-producer single-consumer and that
//! print from one strand must produce byte-identical output in both modes;
//! spawn/join programs whose ordering is enforced by ⋈ must too. Deadlock
//! detection must survive thread-timing races (runs are repeated).

use std::process::{Command, Stdio};

fn mlang() -> &'static str {
    env!("CARGO_BIN_EXE_mlang")
}

fn example(name: &str) -> String {
    format!("{}/../examples/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn run(args: &[&str], stdin: &str, envs: &[(&str, &str)]) -> (Option<i32>, String, String) {
    let mut cmd = Command::new(mlang());
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().unwrap();
    {
        use std::io::Write;
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(stdin.as_bytes())
            .unwrap();
    }
    let out = child.wait_with_output().unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn parallel_matches_sequential_on_spsc_programs() {
    let cases: &[(&str, &str)] = &[
        ("mandelbrot.ml", "z\nw\nr\nq\n"),
        ("pipeline.ml", ""),
        ("pipeline-manual.ml", ""),
        ("parallel-sum.ml", ""),
        ("hello.ml", ""),
        ("std-tour.ml", ""),
    ];
    for (name, stdin) in cases {
        let path = example(name);
        let seq = run(&["run", &path], stdin, &[]);
        for round in 0..3 {
            let par = run(&["run", "--parallel", &path], stdin, &[]);
            assert_eq!(
                seq, par,
                "{name} (round {round}): parallel output diverged from sequential"
            );
        }
    }
}

#[test]
fn parallel_calc_keeps_answers_in_order() {
    // ⋈ serializes the per-line evaluator strands, so answers keep input
    // order even on real threads — including the caught-glitch line.
    let path = example("calc.ml");
    let stdin = "3 4 +\n10 2 - 6 ×\noops\n2 63 ^\n";
    let seq = run(&["run", &path], stdin, &[]);
    for round in 0..3 {
        let par = run(&["run", "--parallel", &path], stdin, &[]);
        assert_eq!(seq, par, "calc.ml (round {round}) diverged");
    }
}

/// Replay a cursor-addressed ANSI stream onto a 24×70 screen, returning
/// one rendered snapshot per ESC[2J clear (plus the final state).
fn render_screens(raw: &str) -> Vec<String> {
    let mut screens = Vec::new();
    let mut cells = std::collections::HashMap::new();
    let (mut row, mut col) = (1usize, 1usize);
    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0;
    let render = |cells: &std::collections::HashMap<(usize, usize), char>| {
        (1..=24)
            .map(|r| {
                (1..=70)
                    .map(|c| cells.get(&(r, c)).copied().unwrap_or(' '))
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    while i < chars.len() {
        if chars[i] == '\x1b' && chars.get(i + 1) == Some(&'[') {
            let start = i + 2;
            let mut j = start;
            while j < chars.len() && !chars[j].is_ascii_alphabetic() {
                j += 1;
            }
            let params: String = chars[start..j].iter().collect();
            match chars.get(j) {
                Some('H') => {
                    let mut it = params.split(';').map(|p| p.parse().unwrap_or(1));
                    row = it.next().unwrap_or(1);
                    col = it.next().unwrap_or(1);
                }
                Some('J') if params == "2" => {
                    screens.push(render(&cells));
                    cells.clear();
                }
                _ => {}
            }
            i = j + 1;
        } else if chars[i] == '\n' {
            row += 1;
            col = 1;
            i += 1;
        } else {
            cells.insert((row, col), chars[i]);
            col += 1;
            i += 1;
        }
    }
    screens.push(render(&cells));
    screens
}

#[test]
fn parallel_dive_converges_to_the_sequential_image() {
    // THE DIVE paints rows as they arrive on one shared channel, so its
    // parallel byte stream is racy by design — but every frame overwrites
    // whole rows at absolute positions, so the rendered screens must be
    // identical to the sequential run's.
    let path = example("mandelbrot-dive.ml");
    let seq = run(&["run", &path, "2"], "", &[]);
    assert_eq!(seq.0, Some(0));
    let seq_screens = render_screens(&seq.1);
    for round in 0..2 {
        let par = run(&["run", "--parallel", &path, "2"], "", &[]);
        assert_eq!(par.0, Some(0), "round {round}: {}", par.2);
        assert_eq!(
            seq_screens,
            render_screens(&par.1),
            "dive (round {round}): parallel screens diverged"
        );
    }
}

#[test]
fn parallel_detects_deadlock() {
    for _ in 0..5 {
        let (code, _, err) = run(&["eval", "--parallel", "↧z"], "", &[]);
        assert_eq!(code, Some(1));
        assert!(err.contains("✗ deadlock"), "stderr was: {err}");
        assert!(err.contains("waiting on channel z"), "stderr was: {err}");
    }
}

#[test]
fn parallel_glitch_kills_only_its_strand() {
    // Strand 0 divides by zero and dies; strand 1 still answers. Exit 1.
    let (code, out, err) = run(&["eval", "--parallel", "1 0÷\n«alive»⍞"], "", &[]);
    assert_eq!(code, Some(1));
    assert_eq!(out, "alive\n");
    assert!(err.contains("✗ glitch in strand 0"), "stderr was: {err}");
}

#[test]
fn welded_binary_honors_mlang_par() {
    let exe = mlang();
    let dir = std::env::temp_dir().join("mlang-par-test");
    std::fs::create_dir_all(&dir).unwrap();
    let bin = dir.join("mandelbrot");
    let build = Command::new(exe)
        .args([
            "build",
            &example("mandelbrot.ml"),
            "-o",
            bin.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(build.status.success(), "build failed: {build:?}");

    let go = |envs: &[(&str, &str)]| {
        let mut cmd = Command::new(&bin);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (k, v) in envs {
            cmd.env(k, v);
        }
        let mut child = cmd.spawn().unwrap();
        {
            use std::io::Write;
            child.stdin.as_mut().unwrap().write_all(b"z\nq\n").unwrap();
        }
        let out = child.wait_with_output().unwrap();
        (out.status.code(), String::from_utf8_lossy(&out.stdout).into_owned())
    };
    let seq = go(&[]);
    let par = go(&[("MLANG_PAR", "1")]);
    assert_eq!(seq.0, Some(0));
    assert_eq!(seq, par, "welded MLANG_PAR=1 output diverged");
}
