//! End-to-end tests of native binary creation: payload round-trip, and
//! actually welding + executing a standalone binary.

use mlang::{payload, vm};

/// A scratch directory unique to this test process, so concurrent test runs
/// (two `cargo test`s, or CI matrix jobs sharing a temp dir) don't trample
/// each other's binaries.
fn scratch_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("mlang-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A representative valid payload: definitions, a strand, a list literal
/// with every value kind, a quotation and source lines.
fn sample_payload() -> Vec<u8> {
    let src = "[∂×]≔²\n⇊\n9²↥a ⟨1 «two» 2.5 ∅ [∂×]⟩↥a ¯42↥a\n↧a⍞↧a⍞↧a⍞";
    payload::serialize(&vm::compile_text(src).unwrap())
}

#[test]
fn payload_round_trips() {
    let src = "[∂×]≔²\n⇊\n9²↥a ⟨1 «two» 2.5 ∅⟩↥a ¯42↥a\n↧a⍞↧a⍞↧a⍞";
    let prog = vm::compile_text(src).unwrap();
    let bytes = payload::serialize(&prog);
    let back = payload::deserialize(&bytes).unwrap();
    // Serialization is canonical: a round trip reproduces identical bytes.
    assert_eq!(bytes, payload::serialize(&back));
    // And the deserialized program behaves identically.
    let run = |p: &vm::CompiledProgram| {
        let mut stdin = std::io::Cursor::new(Vec::new());
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = vm::VM::new(&mut stdin, &mut out, &mut err).run_compiled(p);
        (code, String::from_utf8(out).unwrap())
    };
    assert_eq!(run(&prog), run(&back));
    assert_eq!(run(&back).1, "81\n⟨1 «two» 2.5 ∅⟩\n¯42\n");
}

#[test]
fn extract_rejects_plain_images_and_corrupt_footers(){
    assert!(payload::extract(b"just an ordinary file").is_none());
    let mut bogus = vec![0u8; 4];
    bogus.extend_from_slice(&u64::MAX.to_le_bytes());
    bogus.extend_from_slice(payload::MAGIC);
    assert!(matches!(payload::extract(&bogus), Some(Err(_))));
    // A footer length one byte longer than the body is still corrupt, and
    // must not index before the start of the image.
    let mut off_by_one = vec![0u8; 4];
    off_by_one.extend_from_slice(&5u64.to_le_bytes());
    off_by_one.extend_from_slice(payload::MAGIC);
    assert!(matches!(payload::extract(&off_by_one), Some(Err(_))));
}

#[test]
fn truncated_payload_is_an_error_at_every_length() {
    let bytes = sample_payload();
    assert!(payload::deserialize(&bytes).is_ok());
    // Every proper prefix is a truncated program: never Ok, never a panic.
    for k in 0..bytes.len() {
        let r = std::panic::catch_unwind(|| payload::deserialize(&bytes[..k]));
        match r {
            Ok(Err(_)) => {}
            Ok(Ok(_)) => panic!("truncation to {k} bytes was accepted"),
            Err(_) => panic!("truncation to {k} bytes panicked"),
        }
    }
    // And a payload with extra bytes on the end disagrees with its footer.
    let mut padded = bytes.clone();
    padded.push(0);
    assert!(payload::deserialize(&padded).is_err());
}

#[test]
fn wrong_format_version_is_rejected() {
    let mut bytes = sample_payload();
    // FORMAT_VERSION is the first u32 LE; bump it to something we don't speak.
    bytes[0] = bytes[0].wrapping_add(1);
    let err = payload::deserialize(&bytes).unwrap_err();
    assert!(err.contains("payload format v"), "unexpected error: {err}");
    // Also a version from the far future, whose remaining bytes we would
    // otherwise happily misparse.
    bytes[..4].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(payload::deserialize(&bytes).is_err());
}

#[test]
fn overflowing_length_fields_are_errors_not_aborts() {
    let bytes = sample_payload();
    // The boot code's instruction count is the u64 right after the version.
    // Each of these would either wrap `pos + n` or ask the allocator for an
    // absurd capacity if the reader trusted it.
    for bogus in [u64::MAX, u64::MAX - 4, u64::MAX / 2, 1 << 40, 1 << 32, bytes.len() as u64] {
        let mut b = bytes.clone();
        b[4..12].copy_from_slice(&bogus.to_le_bytes());
        let r = std::panic::catch_unwind(|| payload::deserialize(&b));
        assert!(matches!(r, Ok(Err(_))), "count {bogus:#x} did not yield Err");
    }
    // Every u64 in the payload is a candidate length field; stamp each
    // aligned and unaligned position with a huge value and expect Err.
    for pos in 0..bytes.len().saturating_sub(8) {
        let mut b = bytes.clone();
        b[pos..pos + 8].copy_from_slice(&u64::MAX.to_le_bytes());
        let r = std::panic::catch_unwind(|| payload::deserialize(&b));
        assert!(r.is_ok(), "u64::MAX at byte {pos} panicked");
    }
}

#[test]
fn deeply_nested_payload_is_rejected_without_overflowing_the_stack() {
    // version, then one boot instruction: Push(Quot([Push(Quot([...]))]))
    // repeated far past any stack budget.
    let mut b = Vec::new();
    b.extend_from_slice(&2u32.to_le_bytes());
    for _ in 0..200_000 {
        b.extend_from_slice(&1u64.to_le_bytes()); // code: 1 instr
        b.extend_from_slice(&0u32.to_le_bytes()); // pos
        b.extend_from_slice(&0u32.to_le_bytes());
        b.push(0); // Op::Push
        b.push(5); // Value::Quot
    }
    let r = std::panic::catch_unwind(|| payload::deserialize(&b));
    assert!(matches!(r, Ok(Err(_))));
}

#[test]
fn corrupted_payloads_never_panic() {
    let bytes = sample_payload();
    let mut panics = Vec::new();
    // Single-byte corruptions: every position, several replacement values.
    for pos in 0..bytes.len() {
        for patch in [0x00u8, 0x01, 0x7f, 0x80, 0xff, bytes[pos] ^ 0x01, bytes[pos] ^ 0x10] {
            let mut b = bytes.clone();
            b[pos] = patch;
            if std::panic::catch_unwind(|| payload::deserialize(&b)).is_err() {
                panics.push(format!("byte {pos} := {patch:#04x}"));
            }
        }
    }
    // Multi-byte corruptions from a tiny deterministic generator.
    let mut seed = 0x9e37_79b9_7f4a_7c15u64;
    for _ in 0..500 {
        let mut b = bytes.clone();
        let mut spots = Vec::new();
        for _ in 0..4 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let pos = (seed as usize) % b.len();
            let val = (seed >> 32) as u8;
            b[pos] = val;
            spots.push(format!("{pos}:={val:#04x}"));
        }
        if std::panic::catch_unwind(|| payload::deserialize(&b)).is_err() {
            panics.push(spots.join(" "));
        }
    }
    assert!(panics.is_empty(), "deserialize panicked on corruptions: {panics:?}");
}

#[test]
fn welded_binary_runs_standalone() {
    let exe = env!("CARGO_BIN_EXE_mlang");
    let dir = scratch_dir("weld-test");
    let src = dir.join("prog.ml");
    let bin = dir.join("prog");
    std::fs::write(&src, "«woven»⍞ 6‼⍞ 3⍸⇈q\n⇟q∑⍞").unwrap();

    let build = std::process::Command::new(exe)
        .args(["build", src.to_str().unwrap(), "-o", bin.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(build.status.success(), "build failed: {:?}", build);

    // The built program must run standalone — no toolchain involvement.
    let run = std::process::Command::new(&bin).output().unwrap();
    assert_eq!(run.status.code(), Some(0));
    assert_eq!(String::from_utf8(run.stdout).unwrap(), "woven\n720\n3\n");

    // And it is a runtime, not a compiler: CLI-looking args don't matter.
    let run2 = std::process::Command::new(&bin).arg("ops").output().unwrap();
    assert_eq!(String::from_utf8(run2.stdout).unwrap(), "woven\n720\n3\n");
}

#[test]
fn welded_editor_opens_and_saves_a_dropped_file() {
    let exe = env!("CARGO_BIN_EXE_mlang");
    let dir = scratch_dir("editor-test");
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/editor.ml");
    let bin = dir.join("matrixpad");
    let note = dir.join("note.txt");
    std::fs::write(&note, "There is no spoon.\n").unwrap();

    let build = std::process::Command::new(exe)
        .args(["build", src.to_str().unwrap(), "-o", bin.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(build.status.success(), "build failed: {:?}", build);

    // Dropping a .txt onto the executable launches it with the file's path
    // as the argument. The session below is raw keystrokes: End, Enter,
    // type a line, ^S (saves — the name is known), ^X (clean, not dirty).
    use std::io::Write;
    let mut child = std::process::Command::new(&bin)
        .arg(note.to_str().unwrap())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"\x1b[F\rFree your mind.\x13\x18")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("opened"), "stdout: {stdout}");
    assert!(stdout.contains("saved"), "stdout: {stdout}");
    assert_eq!(
        std::fs::read_to_string(&note).unwrap(),
        "There is no spoon.\nFree your mind.\n"
    );
}
