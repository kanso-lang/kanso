//! What the ratchet's `touched` mode selects, and what it refuses to select.
//!
//! A mutation carries `grep -q '<literal>' <file>` so that it refuses rather
//! than silently applying when the text it patches has moved. That guard is the
//! row's dependency on the tree: a change can only make a mutation inert by
//! moving what the guard matches. The rule used to read the whole script and
//! select any row that MENTIONED a changed path, which on a goldens-heavy
//! branch is most of the table — measured against kanso#1249's 45 changed
//! files, 33 rows selected where 12 were at risk.
//!
//! Goldens come out because a golden guard cannot go stale from a
//! regeneration: over the mutations on disk, exactly two carry a value-shaped
//! number and both are POST-conditions checking what the mutation itself just
//! wrote. Every other golden guard matches a key name and a digit class —
//! `^jsonbench [0-9]`, `^defines=999999` — which a new number still satisfies.
//!
//! `select` drives that rule with a file list written here rather than asked of
//! git, so these are diffs this file wrote down rather than a repository shaped
//! for the check.
//!
//! The counts are deliberately NOT pinned. They move whenever a mutation is
//! added — kanso#1252 added one — and a spec that goes red for that is a
//! tripwire on the wrong thing. What is pinned is the pair of properties that
//! tell the two rules apart, and each is false under the naming rule.

use std::process::Command;

fn selected(files: &[&str]) -> Vec<String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(root.join("scripts/ratchet"))
        .arg("--")
        .arg("select")
        .args(files)
        .current_dir(root)
        .output()
        .expect("the ratchet runs");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.starts_with("  "))
        .map(|l| l.trim().to_string())
        .collect()
}

/// A branch that only regenerates goldens has made no mutation inert, so it has
/// nothing to prove. Under the naming rule this selected twenty-one rows on
/// kanso#1249 and forty minutes of CI went to proving them.
#[test]
fn a_diff_of_goldens_alone_selects_nothing() {
    let rows = selected(&[
        "bench/instructions_golden.txt",
        "bench/text_golden.txt",
        "bench/emitted_golden_others.txt",
        "bench/compile_instructions_golden.txt",
    ]);
    assert!(rows.is_empty(), "a goldens-only diff selected {} rows: {rows:#?}", rows.len());
}

/// And the same diff with a source file in it selects exactly what the source
/// file selects alone. The goldens are still there; they contribute nothing.
/// This is the property the whole change is about, and the one the naming rule
/// cannot satisfy — under it the goldens each drag their own rows in.
#[test]
fn adding_goldens_to_a_source_diff_selects_no_extra_row() {
    let alone = selected(&["src/runtime.c"]);
    let with_goldens = selected(&[
        "src/runtime.c",
        "bench/instructions_golden.txt",
        "bench/text_golden.txt",
        "bench/emitted_golden_others.txt",
    ]);
    assert!(!alone.is_empty(), "src/runtime.c selected nothing, so this spec is reading nothing");
    assert_eq!(alone, with_goldens, "the goldens pulled in rows the source file did not");
}

/// Every row a source file selects guards on that file. Reading the scripts
/// here rather than trusting the selection is the check on the check: a rule
/// that selected on something other than the guard would satisfy the two
/// properties above and still be wrong.
#[test]
fn every_selected_row_guards_on_the_file_that_selected_it() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let rows = selected(&["src/runtime.c"]);
    assert!(!rows.is_empty(), "src/runtime.c selected no rows, so this is reading nothing");
    let dir = root.join("scripts/ratchet/mutations");
    let guarding: Vec<String> = std::fs::read_dir(&dir)
        .expect("the mutations read")
        .filter_map(|e| {
            let path = e.ok()?.path();
            let body = std::fs::read_to_string(&path).ok()?;
            let guards = body.lines().any(|l| l.contains("grep -q") && l.contains("src/runtime.c"));
            guards.then(|| path.file_name()?.to_str().map(str::to_string))?
        })
        .collect();
    assert_eq!(
        rows.len(),
        guarding.len(),
        "{} rows were selected and {} mutations guard on src/runtime.c",
        rows.len(),
        guarding.len()
    );
}
