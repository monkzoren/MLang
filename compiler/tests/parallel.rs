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
fn parallel_rpn_keeps_answers_in_order() {
    // ⋈ serializes the per-line evaluator strands, so answers keep input
    // order even on real threads — including the caught-glitch line.
    let path = example("rpn.ml");
    let stdin = "3 4 +\n10 2 - 6 ×\noops\n2 63 ^\n";
    let seq = run(&["run", &path], stdin, &[]);
    for round in 0..3 {
        let par = run(&["run", "--parallel", &path], stdin, &[]);
        assert_eq!(seq, par, "rpn.ml (round {round}) diverged");
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
fn parallel_deadlock_after_a_producer_finishes() {
    // Strand 0 sends one value and finishes; strand 1 wants two. When
    // strand 1 parks on the second receive, strand 0 may still be live,
    // so no verdict is possible yet — it must be re-checked when strand
    // 0 finishes, and then reported. Repeated, because the race between
    // the park and the finish goes both ways.
    for _ in 0..5 {
        let (code, _, err) = run(&["eval", "--parallel", "1↥z\n↧z↧z"], "", &[]);
        assert_eq!(code, Some(1), "stderr was: {err}");
        assert!(err.contains("✗ deadlock"), "stderr was: {err}");
        assert!(err.contains("strand 1 (row 2) waiting on channel z"), "stderr was: {err}");
    }
}

#[test]
fn parallel_deadlock_on_spawn_then_self_join() {
    // The spawned strand joins itself; its spawner joins it. Both waits
    // are on strand 1, neither can ever complete.
    for _ in 0..5 {
        let (code, _, err) = run(&["eval", "--parallel", "[⍳⋈]⚡⋈"], "", &[]);
        assert_eq!(code, Some(1), "stderr was: {err}");
        assert!(err.contains("✗ deadlock"), "stderr was: {err}");
        assert!(err.contains("strand 0 (row 1) waiting on strand 1"), "stderr was: {err}");
        assert!(err.contains("strand 1 (⚡ of strand 0) waiting on strand 1"), "stderr was: {err}");
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
    let dir = std::env::temp_dir().join(format!("mlang-par-test-{}", std::process::id()));
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

// ── the loom on threads ───────────────────────────────────────────────
//
// A seam is a point in a deterministic schedule, and a strand running on
// its own OS thread has none the runtime can observe. Definitions need no
// seam — they are shared state, resolved at call time, and the bus rebinds
// them all under one lock — so a patch that only rebinds definitions is hot
// under --parallel as well. One that moves a strand is refused, and says so.

/// Only the definition changes: the marker the program splits on is built
/// at runtime, so the strand line does not contain it and stays put.
const DEF_ONLY: &str = concat!(
    "«alpha»≔G\n«al»«pha»⧺≔P\n«be»«ta»⧺≔Q\n⇊\n",
    "1⇒g[g][⎆∂∅=[⌫0⇒g][⇒r r2@«/grow»=[⟐P⊆Q⊇⟡0@⍕][G]?⇒a",
    " ⟨r0@ 200 «text/plain» a⟩⍅]?]⟳\n",
);

/// Replacing «one» everywhere rewrites the strand line too.
const MOVES_A_STRAND: &str = concat!(
    "«one»≔G\n⇊\n",
    "1⇒g[g][⎆∂∅=[⌫0⇒g][⇒r r2@«/grow»=[⟐«one»⊆«two»⊇⟡0@⍕][G]?⇒a",
    " ⟨r0@ 200 «text/plain» a⟩⍅]?]⟳\n",
);

const FRAMES: &str = "▷ GET /\n▷ GET /grow\n▷ GET /\n";

fn body_lines(out: &str) -> Vec<String> {
    out.lines().filter(|l| !l.starts_with('◁')).map(String::from).collect()
}

#[test]
fn a_definition_rebind_is_hot_on_threads_too() {
    let src = scratch("par-loom-def", DEF_ONLY);
    let seq = run(&["run", &src], FRAMES, &[]);
    let par = run(&["run", "--parallel", &src], FRAMES, &[]);
    assert_eq!(seq.0, Some(0));
    assert_eq!(par.0, Some(0));
    // Identical either way: accepted, and the next answer comes from the new
    // definition rather than the cached old one.
    assert_eq!(body_lines(&seq.1), vec!["alpha", "200", "beta"]);
    assert_eq!(body_lines(&par.1), vec!["alpha", "200", "beta"]);
}

#[test]
fn moving_a_strand_is_refused_on_threads_and_the_grid_runs_on() {
    let src = scratch("par-loom-strand", MOVES_A_STRAND);
    let seq = run(&["run", &src], FRAMES, &[]);
    let par = run(&["run", "--parallel", &src], FRAMES, &[]);
    // The deterministic engine re-weaves the strand at its seam.
    assert_eq!(body_lines(&seq.1), vec!["one", "200", "two"]);
    // On threads it is refused, and nothing else changes: the grid keeps
    // answering on the code it already had, and exits cleanly.
    assert_eq!(body_lines(&par.1), vec!["one", "422", "one"]);
    assert_eq!(par.0, Some(0));
}

fn scratch(name: &str, src: &str) -> String {
    let dir = std::env::temp_dir().join("mlang-parallel-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.ml"));
    std::fs::write(&path, src).unwrap();
    path.to_string_lossy().into_owned()
}
