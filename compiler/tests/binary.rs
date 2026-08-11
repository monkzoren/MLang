//! End-to-end tests of native binary creation: payload round-trip, and
//! actually welding + executing a standalone binary.

use mlang::{payload, vm};

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
}

#[test]
fn welded_binary_runs_standalone() {
    let exe = env!("CARGO_BIN_EXE_mlang");
    let dir = std::env::temp_dir().join("mlang-weld-test");
    std::fs::create_dir_all(&dir).unwrap();
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
