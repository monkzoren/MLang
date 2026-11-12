//! The memory budget is process-wide, so its test lives alone in its own
//! binary: no other test can be running when the limit is lowered.

/// Running out of memory is a glitch with coordinates, not an aborted
/// process: the budget (mem.rs) is checked between instructions, ⍥
/// catches it, and unwinding to the handler frees what the body built.
#[test]
fn out_of_memory_is_a_glitch_not_an_abort() {
    mlang::mem::set_limit_mib(64);
    let (code, out, err) = mlang::run_text("«x»[1][∂⧺]⟳", "");
    assert_eq!(code, 1);
    assert_eq!(out, "");
    assert!(err.contains("out of memory — the grid holds more than its 64 MiB budget"), "{err}");
    assert!(err.contains("↑ 1:"), "the fault carries coordinates: {err}");

    let (code, out, _) = mlang::run_text("[«x»[1][∂⧺]⟳][⌫«caught»⍞]⍥«alive»⍞", "");
    assert_eq!((code, out.as_str()), (0, "caught\nalive\n"));
    mlang::mem::set_limit_mib(0);
}
