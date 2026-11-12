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

// ── the stack-effect pass ─────────────────────────────────────────────

fn stack(src: &str) -> Vec<((u32, u32), String)> {
    mlang::check::stack_effects(&vm::compile_text(src).unwrap())
        .findings
        .into_iter()
        .map(|f| (f.pos, f.msg))
        .collect()
}

#[test]
fn an_underflow_is_named_with_what_was_needed_and_what_was_there() {
    assert_eq!(stack("1+"), vec![((1, 2), "+ needs 2 values and the stack holds 1 here".into())]);
}

#[test]
fn a_wrong_type_is_named() {
    assert_eq!(stack("1«a»+⍞"), vec![((1, 5), "+ expects a number and is given a string".into())]);
    assert_eq!(stack("«a»⍸").len(), 1);
    assert_eq!(stack("⟨1⟩«a»⧺").len(), 1);
    assert_eq!(stack("1«a»<").len(), 1);
}

#[test]
fn a_fault_inside_a_definition_names_its_caller() {
    let f = stack("[+]≔A\n⇊\n1 A⍞\n");
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].0, (1, 2));
    assert!(f[0].1.ends_with("— in A, called at 3:3"), "{}", f[0].1);
}

#[test]
fn both_arms_of_a_branch_are_followed() {
    // the fault is in the arm this run would not take
    assert_eq!(stack("1 2[∂][«x»+]?").len(), 1);
    // arms that agree leave a known stack behind them…
    assert_eq!(stack("1[2][3]?+").len(), 1);
    // …and arms that disagree leave an unknown one: silence, not a guess
    assert_eq!(stack("1[2 3][]?++"), vec![]);
}

#[test]
fn a_loop_is_followed_to_its_exit() {
    // a balanced loop leaves the stack as it found it: the + underflows
    assert_eq!(stack("3⇒n[n 0>][n 1-⇒n]⟳+").len(), 1);
    // a loop that grows the stack leaves an unknown depth
    assert_eq!(stack("3⇒n[n 0>][n n 1-⇒n]⟳+"), vec![]);
    // each/fold bodies see the accumulator beneath the item
    assert_eq!(stack("0 250⍸[+]∀⍞"), vec![]);
    assert_eq!(stack("4⍸0[+]⍀⍞"), vec![]);
}

#[test]
fn what_cannot_be_known_is_not_reported() {
    // a caught glitch is the program's own business
    assert_eq!(stack("[1«a»+][⌫]⍥"), vec![]);
    // a value off a channel has no known type
    assert_eq!(stack("⇊\n1↥α\n↧α«a»⧺⍞\n"), vec![]);
    // a recursive definition's effect is not known
    assert_eq!(stack("[∂0>[1-F][]?]≔F\n⇊\n3F++\n"), vec![]);
    // code that asks a value its type is about to branch on the answer
    assert_eq!(stack("⟨1⟩⍙«str»=[«a»⧺][]?"), vec![]);
    // after ↯ nothing runs
    assert_eq!(stack("«boom»↯+"), vec![]);
}

/// The guard against false alarms: every corpus program that runs clean
/// — exit 0, nothing on stderr — must check clean. A finding here is a
/// glyph the pass called certain to glitch in a program that does not.
#[test]
fn every_corpus_program_that_runs_clean_checks_clean() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../conformance/");
    let cases: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(format!("{root}cases.json")).unwrap()).unwrap();
    let expected: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(format!("{root}expected.json")).unwrap()).unwrap();
    let mut clean = 0;
    for case in cases["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let want = &expected[name];
        if want["exit"] != 0 || want["stderr"] != "" {
            continue;
        }
        clean += 1;
        let found = stack(case["source"].as_str().unwrap());
        assert!(found.is_empty(), "{name}: {found:?}");
    }
    assert!(clean > 100, "only {clean} clean cases — is the corpus where it should be?");
}
