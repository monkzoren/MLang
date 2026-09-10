//! The loom — hot patching a running grid.
//!
//! A served MLang program is a grid that never stops. The loom lets any
//! number of agents keep rewriting that grid while it runs: each agent
//! pulls the live source, edits it, and sends the whole file back with
//! the version it started from. The runtime three-way-merges the file
//! against the live version (so agents editing different strands or
//! definitions never block each other), weaves the result, and applies
//! it without a restart:
//!
//! * a `literal≔X` **definition** in the boot section is rebound at once
//!   — names resolve late, so the next reference anywhere in the grid
//!   runs the new code;
//! * a changed **strand** keeps running its old code until its next
//!   *seam* — the boundary between two iterations of its outermost loop
//!   (`⟳` or `⇉` at the top level of the strand), or its death — and is
//!   then rebuilt on the new code with its stack and locals intact,
//!   resuming inside the new code's outermost loop;
//! * an added strand starts; a removed one retires at its seam.
//!
//! Only literal definitions and strands are hot: boot code with effects
//! (a file read, a print) ran once at start and cannot be re-run
//! honestly, so a patch that changes it is refused.
//!
//! Version history lives here: version 0 is the program as started,
//! every accepted patch is the next. Fault reports name the version a
//! position belongs to, so a strand still running v2 code reports v2
//! coordinates and excerpts.
//!
//! The merge is line-based, like `diff3`: the natural unit of an MLang
//! program is a line — one strand, or one definition — which is why
//! agents working on different machines of the same grid merge cleanly.

use crate::values::{Instr, Op, Value};
use std::sync::{Arc, Mutex};

/// The stamp `mlang pull` writes as the first line of a pulled file, and
/// `mlang patch` reads back to learn the base version and the server.
/// It is a comment, so a stamped file is a valid program too.
pub const STAMP: &str = "※ loom v";

pub struct Version {
    pub text: String,
    pub note: String,
}

/// The shared version store — the VM applies patches to it, and the web
/// bridge serves `GET /.loom` from it on its own thread.
pub struct Loom {
    versions: Mutex<Vec<Version>>,
    /// The fault reports the run has produced (glitches, deadlocks),
    /// newest last, capped — an agent mending a grid over HTTP needs the
    /// report the runtime wrote to stderr.
    faults: Mutex<Vec<String>>,
}

const MAX_FAULTS: usize = 64;

impl Loom {
    pub fn new(text: &str) -> Arc<Loom> {
        Arc::new(Loom {
            versions: Mutex::new(vec![Version {
                text: text.to_string(),
                note: "as started".into(),
            }]),
            faults: Mutex::new(Vec::new()),
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Version>> {
        self.versions.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The live version number (0 = as started).
    pub fn current(&self) -> usize {
        self.lock().len() - 1
    }

    pub fn text(&self, v: usize) -> Option<String> {
        self.lock().get(v).map(|x| x.text.clone())
    }

    /// One line per version: `v3  +1 definition, 1 strand replaced`.
    pub fn log(&self) -> String {
        let mut out = String::new();
        for (i, v) in self.lock().iter().enumerate() {
            out.push_str(&format!("v{i}  {}\n", v.note));
        }
        out
    }

    /// Keep a fault report for `GET /.loom/faults`.
    pub fn record_fault(&self, report: String) {
        let mut f = self.faults.lock().unwrap_or_else(|e| e.into_inner());
        if f.len() >= MAX_FAULTS {
            f.remove(0);
        }
        f.push(report);
    }

    /// Every kept fault report, oldest first, separated by blank lines.
    pub fn faults(&self) -> String {
        self.faults.lock().unwrap_or_else(|e| e.into_inner()).join("\n")
    }

    /// Record an accepted patch; returns its version number.
    pub fn push(&self, text: String, note: String) -> usize {
        let mut vs = self.lock();
        vs.push(Version { text, note });
        vs.len() - 1
    }

    /// Merge a patch written against `base` onto the live version.
    /// Err carries the conflict report, ready to show the agent.
    pub fn merge(&self, base: usize, text: &str) -> Result<String, String> {
        let vs = self.lock();
        let cur = vs.len() - 1;
        let Some(base_v) = vs.get(base) else {
            return Err(format!(
                "✗ patch rejected: base v{base} does not exist (live version is v{cur})\n"
            ));
        };
        if base == cur {
            return Ok(text.to_string());
        }
        let base_lines: Vec<&str> = base_v.text.lines().collect();
        let ours: Vec<&str> = vs[cur].text.lines().collect();
        let theirs: Vec<&str> = text.lines().collect();
        match merge3(&base_lines, &ours, &theirs) {
            Ok(lines) => Ok(lines.join("\n") + "\n"),
            Err(conflicts) => {
                let mut out = format!(
                    "✗ patch conflicts with v{cur} (written against v{base}) — pull v{cur} and reapply your change:\n"
                );
                for c in &conflicts {
                    out.push_str(&format!("  v{cur} lines {}-{}:\n", c.at + 1, c.at + c.ours.len().max(1)));
                    for l in &c.ours {
                        out.push_str(&format!("    │ {l}\n"));
                    }
                    out.push_str("  yours:\n");
                    for l in &c.theirs {
                        out.push_str(&format!("    │ {l}\n"));
                    }
                }
                Err(out)
            }
        }
    }
}

/// Prefix a source with its version stamp.
pub fn stamp(v: usize, url: &str, text: &str) -> String {
    format!("{STAMP}{v} {url}\n{text}")
}

/// Split a stamped source into (version, url, program text). A file
/// without a stamp yields (None, "", text).
pub fn unstamp(text: &str) -> (Option<usize>, &str, &str) {
    if let Some(rest) = text.strip_prefix(STAMP) {
        let (head, body) = match rest.split_once('\n') {
            Some((h, b)) => (h, b),
            None => (rest, ""),
        };
        let mut words = head.split_whitespace();
        let v = words.next().and_then(|w| w.parse().ok());
        let url = words.next().unwrap_or("");
        return (v, url, body);
    }
    (None, "", text)
}

// ── three-way merge ────────────────────────────────────────────────────

pub struct Conflict {
    /// Where in `ours` the conflicting region starts (0-based line).
    pub at: usize,
    pub ours: Vec<String>,
    pub theirs: Vec<String>,
}

/// Longest common subsequence of two line lists, as matched index pairs.
fn lcs_pairs<T: PartialEq>(a: &[T], b: &[T]) -> Vec<(usize, usize)> {
    let (n, m) = (a.len(), b.len());
    // dp[i][j] = LCS length of a[i..], b[j..]
    let mut dp = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i] == b[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }
    let mut pairs = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if a[i] == b[j] {
            pairs.push((i, j));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    pairs
}

/// diff3: regions where both sides agree with the base are stable; in
/// between, a region changed on one side only takes that side, a region
/// changed identically on both takes it once, and anything else is a
/// conflict.
pub fn merge3(base: &[&str], ours: &[&str], theirs: &[&str]) -> Result<Vec<String>, Vec<Conflict>> {
    let mo = lcs_pairs(base, ours);
    let mt = lcs_pairs(base, theirs);
    let mut ours_of = vec![None; base.len()];
    let mut theirs_of = vec![None; base.len()];
    for (b, o) in mo {
        ours_of[b] = Some(o);
    }
    for (b, t) in mt {
        theirs_of[b] = Some(t);
    }
    // Anchors: base lines matched on both sides, in order.
    let mut anchors: Vec<(usize, usize, usize)> = (0..base.len())
        .filter_map(|b| Some((b, ours_of[b]?, theirs_of[b]?)))
        .collect();
    anchors.push((base.len(), ours.len(), theirs.len()));

    let mut out: Vec<String> = Vec::new();
    let mut conflicts = Vec::new();
    let (mut b0, mut o0, mut t0) = (0, 0, 0);
    for &(b1, o1, t1) in &anchors {
        let bc = &base[b0..b1];
        let oc = &ours[o0..o1];
        let tc = &theirs[t0..t1];
        if oc == bc {
            out.extend(tc.iter().map(|s| s.to_string()));
        } else if tc == bc || tc == oc {
            out.extend(oc.iter().map(|s| s.to_string()));
        } else if oc.len() == bc.len() && tc.len() == bc.len() {
            // Adjacent edits: every line is its own unit — a strand or a
            // definition — so two agents changing neighbouring lines
            // merge line by line, and only a line both changed
            // differently conflicts.
            for k in 0..bc.len() {
                let (b, o, t) = (bc[k], oc[k], tc[k]);
                if o == b || o == t {
                    out.push(t.to_string());
                } else if t == b {
                    out.push(o.to_string());
                } else {
                    conflicts.push(Conflict { at: o0 + k, ours: vec![o.to_string()], theirs: vec![t.to_string()] });
                    out.push(o.to_string());
                }
            }
        } else {
            conflicts.push(Conflict {
                at: o0,
                ours: oc.iter().map(|s| s.to_string()).collect(),
                theirs: tc.iter().map(|s| s.to_string()).collect(),
            });
            out.extend(oc.iter().map(|s| s.to_string()));
        }
        if b1 < base.len() {
            out.push(ours[o1].to_string());
        }
        (b0, o0, t0) = (b1 + 1, o1 + 1, t1 + 1);
    }
    if conflicts.is_empty() {
        Ok(out)
    } else {
        Err(conflicts)
    }
}

// ── code identity ──────────────────────────────────────────────────────

/// Structural equality of instruction strips, ignoring positions — the
/// test for "did this strand or definition actually change" (comments
/// and spacing never count).
pub fn instrs_eq(a: &[Instr], b: &[Instr]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| op_eq(&x.op, &y.op))
}

fn op_eq(a: &Op, b: &Op) -> bool {
    match (a, b) {
        (Op::Push(x), Op::Push(y)) => value_eq(x, y),
        (Op::Name(x), Op::Name(y)) => x == y,
        (Op::LMark, Op::LMark) | (Op::LBuild, Op::LBuild) => true,
        (Op::B(c, a1, a2), Op::B(d, b1, b2)) => c == d && a1 == b1 && a2 == b2,
        _ => false,
    }
}

/// Structural value equality: quotations by content, not identity.
pub fn value_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Quot(x), Value::Quot(y)) => instrs_eq(x, y),
        (Value::List(x), Value::List(y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(p, q)| value_eq(p, q))
        }
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Big(x), Value::Big(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x.to_bits() == y.to_bits(),
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Nil, Value::Nil) | (Value::Mark, Value::Mark) => true,
        _ => false,
    }
}

// ── the boot section's shape ───────────────────────────────────────────

/// What a boot strip is made of: the definitions it binds, in order,
/// each with the value its expression computes, and every instruction
/// that is *not* part of a pure definition — the boot code that ran once
/// with effects and cannot be re-run. The VM computes it (§4.7): a
/// definition's expression is everything since the previous `≔`, and it
/// counts only if it evaluates, without effects, to exactly one value.
pub struct BootShape {
    pub defs: Vec<(char, Value, crate::values::Pos)>,
    pub code: Vec<Instr>,
}

// ── migrations ─────────────────────────────────────────────────────────

/// The migration marker: a patch line `⟲ code` just above a strand line
/// runs `code` once, at that strand's seam, on the old stack and locals,
/// before the new code takes over — Erlang's `code_change`, as a line.
pub const MIGRATE: char = '⟲';

/// Pull the migration lines out of a merged patch. Returns the text the
/// loom stores — every `⟲` line rewritten as a comment, so row numbers
/// stay put and the history shows what was migrated — and the
/// migrations as (row, code) in file order.
pub fn split_migrations(text: &str) -> (String, Vec<(u32, String)>) {
    let mut stored = String::new();
    let mut migrations = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(code) = trimmed.strip_prefix(MIGRATE) {
            migrations.push((i as u32 + 1, code.to_string()));
            stored.push_str(&format!("※ {MIGRATE}{code}\n"));
        } else {
            stored.push_str(line);
            stored.push('\n');
        }
    }
    (stored, migrations)
}

// ── strand matching ────────────────────────────────────────────────────

pub enum StrandAction {
    /// Unchanged: old slot index, new strand index.
    Keep(usize, usize),
    /// Same place in the grid, different code.
    Replace(usize, usize),
    Retire(usize),
    Start(usize),
}

/// How alike two strips are: shared instructions (as an LCS) over the
/// longer strip, 0…1.
fn similarity(a: &[Instr], b: &[Instr]) -> f64 {
    struct Key<'a>(&'a Instr);
    impl PartialEq for Key<'_> {
        fn eq(&self, o: &Self) -> bool {
            op_eq(&self.0.op, &o.0.op)
        }
    }
    let longest = a.len().max(b.len());
    if longest == 0 {
        return 1.0;
    }
    let ka: Vec<Key> = a.iter().map(Key).collect();
    let kb: Vec<Key> = b.iter().map(Key).collect();
    lcs_pairs(&ka, &kb).len() as f64 / longest as f64
}

/// A changed strand is the same strand when at least this much of its
/// code survived the edit; below it, the old one retires and the new one
/// starts fresh — an unrelated strand must not inherit a stranger's
/// stack and locals.
const SAME_STRAND: f64 = 0.5;

/// Pair the live source strands with the patched ones. Unchanged strands
/// anchor the alignment (LCS on code identity); between anchors, each old
/// strand continues as the most similar new one (if similar enough), and
/// the leftovers retire or start. The plan lists new strands in source
/// order, retirements last.
pub fn plan_strands(old: &[Arc<Vec<Instr>>], new: &[Vec<Instr>]) -> Vec<StrandAction> {
    struct Key<'a>(&'a [Instr]);
    impl PartialEq for Key<'_> {
        fn eq(&self, o: &Self) -> bool {
            instrs_eq(self.0, o.0)
        }
    }
    let ok: Vec<Key> = old.iter().map(|c| Key(c)).collect();
    let nk: Vec<Key> = new.iter().map(|c| Key(c)).collect();
    let mut anchors = lcs_pairs(&ok, &nk);
    anchors.push((old.len(), new.len()));
    // continues_as[n] = the old strand that lives on as new strand n.
    let mut continues_as: Vec<Option<usize>> = vec![None; new.len()];
    let mut retired: Vec<usize> = Vec::new();
    let (mut o0, mut n0) = (0, 0);
    for &(o1, n1) in &anchors {
        for o in o0..o1 {
            let best = (n0..n1)
                .filter(|&n| continues_as[n].is_none())
                .map(|n| (n, similarity(&old[o], &new[n])))
                .filter(|&(_, s)| s >= SAME_STRAND)
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            match best {
                Some((n, _)) => continues_as[n] = Some(o),
                None => retired.push(o),
            }
        }
        if o1 < old.len() {
            continues_as[n1] = Some(o1);
        }
        (o0, n0) = (o1 + 1, n1 + 1);
    }
    let mut plan: Vec<StrandAction> = continues_as
        .iter()
        .enumerate()
        .map(|(n, o)| match o {
            Some(o) if instrs_eq(&old[*o], &new[n]) => StrandAction::Keep(*o, n),
            Some(o) => StrandAction::Replace(*o, n),
            None => StrandAction::Start(n),
        })
        .collect();
    plan.extend(retired.into_iter().map(StrandAction::Retire));
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(b: &str, o: &str, t: &str) -> Result<String, usize> {
        let bl: Vec<&str> = b.lines().collect();
        let ol: Vec<&str> = o.lines().collect();
        let tl: Vec<&str> = t.lines().collect();
        merge3(&bl, &ol, &tl).map(|v| v.join("\n")).map_err(|c| c.len())
    }

    #[test]
    fn disjoint_edits_merge() {
        assert_eq!(m("a\nb\nc", "A\nb\nc", "a\nb\nC"), Ok("A\nb\nC".into()));
    }

    #[test]
    fn insertions_on_both_sides_merge() {
        assert_eq!(m("a\nb", "a\nx\nb", "a\nb\ny"), Ok("a\nx\nb\ny".into()));
    }

    #[test]
    fn same_change_merges_once() {
        assert_eq!(m("a\nb", "a\nB", "a\nB"), Ok("a\nB".into()));
    }

    #[test]
    fn overlapping_change_conflicts() {
        assert_eq!(m("a\nb\nc", "a\nB\nc", "a\nb2\nc"), Err(1));
    }

    #[test]
    fn deletion_versus_edit_conflicts() {
        assert_eq!(m("a\nb\nc", "a\nc", "a\nB\nc"), Err(1));
    }

    #[test]
    fn adjacent_line_edits_merge() {
        assert_eq!(m("a\nb\nc", "A\nb\nc", "a\nB\nc"), Ok("A\nB\nc".into()));
        assert_eq!(m("a\nb", "A\nb", "a2\nB"), Err(1));
    }

    #[test]
    fn migrations_split_out_and_keep_rows() {
        let (stored, m) = split_migrations("a\n⟲ 0⇒n\nb\n");
        assert_eq!(stored, "a\n※ ⟲ 0⇒n\nb\n");
        assert_eq!(m, vec![(2, " 0⇒n".to_string())]);
    }

    #[test]
    fn stamp_round_trips() {
        let s = stamp(3, "http://127.0.0.1:4321", "1⍞\n");
        assert_eq!(unstamp(&s), (Some(3), "http://127.0.0.1:4321", "1⍞\n"));
        assert_eq!(unstamp("1⍞\n"), (None, "", "1⍞\n"));
    }
}
