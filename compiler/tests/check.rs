//! `mlang check` — what is wrong with a program that weaves clean.
//!
//! Both of these are static properties of the text that the runtime only
//! announces once the program has already gone wrong: a pathway that
//! terminates nowhere waits for the deadlock, and a name that is bound
//! nowhere waits for execution to reach it. `check` says them up front.

use mlang::vm;

fn census(src: &str) -> String {
    vm::static_channel_census(&vm::compile_text(src).unwrap())
}

fn unbound(src: &str) -> Vec<char> {
    vm::unbound_names(&vm::compile_text(src).unwrap())
        .into_iter()
        .map(|(c, _)| c)
        .collect()
}

#[test]
fn a_pathway_that_terminates_nowhere_is_named_without_running() {
    // The M2 shape: a unit spliced in before its connections exist.
    let out = census("⇊\n1⇒g[g][⌫0⇒g]⟳\n[]⇉γδ\n");
    assert!(out.contains("channel γ is received at 1 site and never sent to"), "{out}");
    assert!(out.contains("channel δ is sent to at 1 site and never received"), "{out}");
}

#[test]
fn a_wired_grid_has_a_clean_census() {
    assert_eq!(census("⇊\n⟨1 2⟩⇈α\n[∂×]⇉αβ\n⇟β⌫\n"), "");
}

#[test]
fn a_name_bound_nowhere_is_found_before_it_is_reached() {
    // Ω is referenced on a branch that never runs, so the program exits 0 —
    // and is still wrong.
    assert_eq!(unbound("⇊\n0[Ω][]?\n"), vec!['Ω']);
}

#[test]
fn names_bound_anywhere_are_not_reported() {
    // ⇒ｒｏ is ordinary MLang: store into the local ｒ, then call the word ｏ.
    // Flagging adjacency alone would condemn most of std/.
    assert_eq!(unbound("[7]≔ｏ\n⇊\n1⇒ｒｏ⌫\n"), Vec::<char>::new());
}

#[test]
fn a_multi_glyph_name_reports_every_glyph_it_left_behind() {
    // ⇒pos binds p; o and s become references, which is the whole trap.
    assert_eq!(unbound("⇊\n0⇒pos⌫\n"), vec!['o', 's']);
}

#[test]
fn reading_a_local_as_a_channel_is_caught_twice_over() {
    // ↧cnt receives from channel c — never a local — and strands n and t.
    let src = "⇊\n«»⇒q [⇒w ↧cnt w]⇉γα\n";
    assert_eq!(unbound(src), vec!['n', 't']);
    assert!(census(src).contains("channel c is received"), "{}", census(src));
}
