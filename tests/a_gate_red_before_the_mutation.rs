//! A row is proof only if the gate it reddened was green to begin with.
//!
//! `prove` read a non-zero exit from a mutated worktree as the mutation's
//! doing and never asked what the gate said before it. A gate already red on
//! HEAD would therefore have been credited to every mutation that shares it:
//! the rows pass, the table says "every row turned its gate red", and none of
//! them proved anything. That is the exact defect the table exists to catch,
//! sitting inside the table's own harness — and it is not hypothetical, since
//! `prove` builds in a worktree with a shared target directory and scratch
//! paths ci.yml never uses, any of which can redden a gate on its own.
//!
//! Both fixtures run the real `prove` over one row: the first on a HEAD whose
//! gate is already red, the second on a clean HEAD. The first is the defect;
//! the second is what says the baseline pass did not break proving.
//!
//! `python-free` is the row they use because it is the cheapest end-to-end
//! one in the table: no setup, and a gate that is two git greps. Each fixture
//! is about two seconds.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Mutex;

/// `prove` names its scratch worktrees by fixed paths, so two of these at
/// once would fight over the same directories and over git's worktree
/// metadata.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

fn git(dir: &Path, args: &[&str]) {
    let done = Command::new("git").args(args).current_dir(dir).output().expect("git runs");
    assert!(
        done.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&done.stderr)
    );
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn discarded(tree: &PathBuf) {
    let _ = Command::new("git")
        .args(["worktree", "remove", "--force"])
        .arg(tree)
        .current_dir(root())
        .output();
    let _ = std::fs::remove_dir_all(tree);
}

/// Ask the ratchet to prove the python-free row on a worktree of HEAD, having
/// first committed a tracked python file there when `crept` — which is what
/// the python-free gate refuses, so the gate is red before any mutation runs.
/// Hands back how `prove` exited and everything it said.
fn proved(key: &str, crept: bool) -> (Output, String) {
    let tree = std::env::temp_dir().join(format!("kanso-baseline-{key}"));
    discarded(&tree);
    for stale in ["kanso-ratchet-base", "kanso-ratchet-1"] {
        discarded(&std::env::temp_dir().join(stale));
    }
    let path = tree.to_str().expect("a path");
    git(root(), &["worktree", "add", "--detach", "--force", path, "HEAD"]);

    if crept {
        std::fs::write(tree.join("crept_in.py"), "pass\n").expect("the file writes");
        git(&tree, &["add", "crept_in.py"]);
        // `git config` in a linked worktree writes the repository's SHARED
        // config, so two fixtures doing it at once would race for its lock.
        // `-c` passes the identity for this one command and writes nothing.
        git(
            &tree,
            &[
                "-c",
                "user.email=spec@kanso.invalid",
                "-c",
                "user.name=spec",
                "commit",
                "--quiet",
                "-m",
                "a tracked python file",
            ],
        );
    }

    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(root().join("scripts/ratchet"))
        .args(["--", "prove", "python-free"])
        .current_dir(&tree)
        .output()
        .expect("the ratchet runs");
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&done.stdout),
        String::from_utf8_lossy(&done.stderr)
    );
    discarded(&tree);
    (done, said)
}

#[test]
fn a_gate_red_before_the_mutation_is_refused_rather_than_credited() {
    let _held = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    // The mutation applies and the gate goes red exactly as it would on a
    // clean tree, so nothing downstream of the baseline can tell this apart
    // from proof.
    let (done, said) = proved("crept", true);
    assert!(!done.status.success(), "a gate red before the mutation is not proof:\n{said}");
    assert!(
        !said.contains("every row turned its gate red"),
        "nothing may claim the row was proved:\n{said}"
    );
    assert!(said.contains("ALREADY RED"), "the refusal says which gate was red first:\n{said}");
    // And WHY. Naming the gate answered the question it was built for on its
    // first CI run and could not answer the second: with valgrind installed the
    // same gate was still red, and a report that says only which gate leaves a
    // reader with hypotheses and no way to choose. The gate wrote down its
    // reason; the ratchet was throwing it away. Here that reason is the file
    // the fixture committed.
    assert!(
        said.contains("crept_in.py"),
        "the refusal carries the gate's own words, not just its name:\n{said}"
    );
}

#[test]
fn a_clean_head_still_proves_its_row() {
    let _held = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let (done, said) = proved("clean", false);
    assert!(done.status.success(), "a row on a clean HEAD still proves its gate:\n{said}");
    assert!(
        said.contains("1 gates green before any mutation"),
        "the baseline reads the row's gate before the mutation:\n{said}"
    );
    assert!(
        said.contains("every row turned its gate red"),
        "the mutation still reddens the gate it claims:\n{said}"
    );
}

#[test]
fn a_scope_that_names_no_row_is_refused() {
    // Selecting nothing and reporting that every row turned its gate red is
    // the same false green in miniature, and a typo is all it takes.
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(root().join("scripts/ratchet"))
        .args(["--", "prove", "no-such-job"])
        .current_dir(root())
        .output()
        .expect("the ratchet runs");
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&done.stdout),
        String::from_utf8_lossy(&done.stderr)
    );
    assert!(!done.status.success(), "an empty selection proves nothing:\n{said}");
    assert!(said.contains("no row is named by"), "the refusal names the scope:\n{said}");
    assert!(
        !said.contains("every row turned its gate red"),
        "nothing may claim rows were proved:\n{said}"
    );
}

/// The same `prove` over the same row, on a copy of the ratchet whose
/// `work_gate` has been pointed at the python-free gate. That one constant is
/// the whole edit: it makes a CHEAP gate host-bound, which is otherwise a
/// property only the four callgrind gates have and only a runner can exercise.
///
/// Everything else is the shipped program, run end to end, and the baseline
/// answers the gate here exactly as CI's runner answers a compile gate there.
fn proved_under_a_host_bound_gate(key: &str) -> (Output, String) {
    let tree = std::env::temp_dir().join(format!("kanso-bound-{key}"));
    discarded(&tree);
    for stale in ["kanso-ratchet-base", "kanso-ratchet-1"] {
        discarded(&std::env::temp_dir().join(stale));
    }
    let path = tree.to_str().expect("a path");
    git(root(), &["worktree", "add", "--detach", "--force", path, "HEAD"]);

    let staged = std::env::temp_dir().join(format!("kanso-bound-program-{key}"));
    let _ = std::fs::remove_dir_all(&staged);
    std::fs::create_dir_all(&staged).expect("the staged program has a home");
    let source = root().join("scripts/ratchet");
    std::fs::copy(source.join("main.kso"), staged.join("main.kso")).expect("main copies");
    let program = std::fs::read_to_string(source.join("ratchet.kso")).expect("the ratchet reads");
    let real = "work_gate = \"sh scripts/gates/instructions.sh\"";
    assert!(program.contains(real), "the work gate is still spelled the way this fixture edits");
    let bent = program.replace(real, "work_gate = \"sh scripts/gates/python_free.sh\"");
    std::fs::write(staged.join("ratchet.kso"), bent).expect("the staged ratchet writes");

    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(&staged)
        .args(["--", "prove", "python-free"])
        .current_dir(&tree)
        .output()
        .expect("the ratchet runs");
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&done.stdout),
        String::from_utf8_lossy(&done.stderr)
    );
    discarded(&tree);
    let _ = std::fs::remove_dir_all(&staged);
    (done, said)
}

#[test]
fn a_host_bound_row_is_proved_where_its_gate_is_green() {
    let _held = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let (done, said) = proved_under_a_host_bound_gate("green");
    // The baseline ran the gate and it was green, which is the whole question:
    // this run DID land on a host that can answer it.
    assert!(
        said.contains("1 gates green before any mutation"),
        "the baseline answered the gate:\n{said}"
    );
    // So the row is proved, rather than skipped for a property of the gate
    // that says nothing about this run. Skipping it printed
    //
    //     ratchet: no row on this runner could be proved; none was claimed
    //
    // on a runner that had just proved it could answer the gate, and the row
    // vanished from the report without a line. kanso#1338's ratchet job is the
    // instance in production: five gates green, five rows selected, three
    // proved, and the two compile rows gone.
    assert!(
        !said.contains("no row on this runner could be proved"),
        "a gate the baseline answered is not an excuse to skip its row:\n{said}"
    );
    assert!(said.contains("ratchet: 1 rows"), "the row runs:\n{said}");
    assert!(
        said.contains("every row turned its gate red"),
        "and it turns the gate it claims red:\n{said}"
    );
    assert!(done.status.success(), "a proved row is not a failure:\n{said}");
}
