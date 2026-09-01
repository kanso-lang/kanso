//! A pull request pays for the ratchet rows its own diff could have made
//! blind, and for no others.
//!
//! Two rows went blind on 2026-08-30 and the nightly said so, twice, to
//! nobody. Neither was caught per-PR, and the reason is sharper than "the
//! cheap half is cheap": that half applies every mutation to a worktree of
//! HEAD, and both mutations still APPLIED. #1171's sed matched and put its
//! statement in a comment; #1188's sed matched and the build then failed.
//! Applying is not proving, and only proving costs a build.
//!
//! What both had in common is that the change which killed them touched the
//! very file their own mutation patches — #1171 src/runtime.c, #1157 and
//! #1188 src/lib.rs. So `touched` intersects the files a mutation names with
//! the files a branch changes and proves that handful.
//!
//! These fixtures assert the selection rather than the proving, because the
//! proving is a build per row and the selection is the part that can go
//! quietly wrong: a mutation whose paths stop matching selects nothing, exits
//! zero, and reads as a branch that broke no rows.

use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

/// `git worktree add` and `git worktree remove` both write the repository's
/// worktree metadata, and three fixtures doing it at once is a lock contention
/// git reports as a plain failure. One at a time; each is about a second.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

fn git(dir: &Path, args: &[&str]) {
    let done = Command::new("git").args(args).current_dir(dir).output().expect("git runs");
    assert!(
        done.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&done.stderr)
    );
}

/// Touch `paths` on a branch off HEAD in a worktree of this repository, and
/// ask the ratchet which rows that branch would have to prove.
fn selected(key: &str, paths: &[&str]) -> String {
    let _held = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tree = std::env::temp_dir().join(format!("kanso-touched-{key}"));
    let _ = Command::new("git")
        .args(["worktree", "remove", "--force"])
        .arg(&tree)
        .current_dir(root)
        .output();
    let _ = std::fs::remove_dir_all(&tree);
    git(root, &["worktree", "add", "--detach", "--force", tree.to_str().expect("a path"), "HEAD"]);

    let base = String::from_utf8_lossy(
        &Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&tree)
            .output()
            .expect("rev-parse runs")
            .stdout,
    )
    .trim()
    .to_string();

    for path in paths {
        let held = tree.join(path);
        let mut text = std::fs::read_to_string(&held).expect("the file reads");
        text.push_str("\n/* a line this branch added */\n");
        std::fs::write(&held, text).expect("the file writes");
    }
    git(&tree, &["config", "user.email", "spec@kanso.invalid"]);
    git(&tree, &["config", "user.name", "spec"]);
    git(&tree, &["commit", "--quiet", "-a", "-m", "the branch"]);

    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(root.join("scripts/ratchet"))
        .args(["--", "touched", &base, "list"])
        .current_dir(&tree)
        .output()
        .expect("the ratchet runs");
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&done.stdout),
        String::from_utf8_lossy(&done.stderr)
    );
    let _ = Command::new("git")
        .args(["worktree", "remove", "--force"])
        .arg(&tree)
        .current_dir(root)
        .output();
    assert!(done.status.success(), "the listing should not fail:\n{said}");
    said
}

#[test]
fn a_branch_touching_the_runtime_pays_for_the_runtime_rows() {
    let said = selected("runtime", &["src/runtime.c"]);
    // The row #1171 made blind: its mutation is the only one that patches the
    // in-place byte append, and it patches src/runtime.c to do it.
    assert!(
        said.contains("a byte builder that writes the right length and the wrong bytes"),
        "the row a src/runtime.c change made blind must be selected by one:\n{said}"
    );
    assert!(
        !said.contains("a python call creeping back into a harness"),
        "a row patching nothing this branch touched must not be selected:\n{said}"
    );
}

#[test]
fn a_branch_touching_the_front_end_pays_for_the_front_end_row() {
    let said = selected("frontend", &["src/lib.rs"]);
    // The row #1157 and #1188 made blind, twice over.
    assert!(
        said.contains("a front-end pass that owns the program's names"),
        "the row two src/lib.rs changes made blind must be selected:\n{said}"
    );
}

#[test]
fn a_branch_touching_neither_pays_for_neither() {
    let said = selected("prose", &["README.md"]);
    assert!(
        said.contains("touches no file any mutation patches"),
        "a prose-only branch owes the ratchet nothing:\n{said}"
    );
}
