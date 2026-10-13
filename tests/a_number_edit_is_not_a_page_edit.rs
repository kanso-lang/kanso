//! `scripts/page_drift` counts log entries written since docs/compiler.html
//! last moved. It took the page's newest commit as the move, and
//! `golden_prose --write` commits the page whenever a golden it quotes moves:
//! a data-golden span's number is rewritten and no sentence changes. On
//! kanso#1913 that rewrite reset the count to zero while #1905 still owed
//! sections for the day's entries, and on main the operator arms' rows did
//! the same, so the gate read 3/3 where the page was four entries behind.
//!
//! A commit moves the page when its removed and added lines still differ
//! with the digits, commas and points taken out.

use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let done = Command::new("git").args(args).current_dir(dir).output().expect("git runs");
    assert!(done.status.success(), "git {args:?}: {done:?}");
}

const ENTRIES: [&str; 4] = [
    "2026-10-12 — a file names only what it imports",
    "2026-10-12 — a wide tail call keeps its jump",
    "2026-10-12 — arms travel with the value",
    "2026-10-12 — mixed bit operators are parenthesized",
];

/// The page moves, four entries land, and then the page is committed again
/// with `after` in place of its first paragraph. Answers what the gate says.
fn drift(key: &str, after: &str) -> (bool, String) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let stage = std::env::temp_dir().join(format!("kanso-number-edit-{key}"));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(stage.join("docs")).expect("a docs directory");
    std::fs::create_dir_all(stage.join("design")).expect("a design directory");

    git(&stage, &["init", "-q", "."]);
    git(&stage, &["config", "user.email", "spec@example.invalid"]);
    git(&stage, &["config", "user.name", "the spec"]);

    let page = |first: &str| format!("{first}\n<p>a second paragraph that stays.</p>\n");
    let before = "<p>compiling takes <span data-golden=\"compile\">25,761,215</span></p>";
    std::fs::write(stage.join("design/compiler-log.md"), "# log\n").expect("the log writes");
    std::fs::write(stage.join("docs/compiler.html"), page(before)).expect("the page writes");
    git(&stage, &["add", "-A"]);
    git(&stage, &["commit", "-qm", "the page moves"]);

    let mut log = String::from("# log\n");
    for entry in ENTRIES {
        log.push_str(&format!("\n## {entry}\n\nwhat it says.\n"));
    }
    std::fs::write(stage.join("design/compiler-log.md"), log).expect("the log grows");
    git(&stage, &["add", "design/compiler-log.md"]);
    git(&stage, &["commit", "-qm", "entries"]);

    std::fs::write(stage.join("docs/compiler.html"), page(after)).expect("the page rewrites");
    git(&stage, &["add", "docs/compiler.html"]);
    git(&stage, &["commit", "-qm", "the page again"]);

    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(root.join("scripts/page_drift"))
        .current_dir(&stage)
        .output()
        .expect("page_drift runs");
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&done.stdout),
        String::from_utf8_lossy(&done.stderr)
    );
    let _ = std::fs::remove_dir_all(&stage);
    (done.status.success(), said)
}

/// What `golden_prose --write` commits: one span's number, and a wider one.
#[test]
fn a_rewritten_number_leaves_the_page_where_it_was() {
    let (green, said) =
        drift("number", "<p>compiling takes <span data-golden=\"compile\">125,881,551</span></p>");
    assert!(!green, "a number edit pays none of the four: {said}");
    assert!(said.contains("the log is 4 entries ahead"), "the four still count: {said}");
}

/// A sentence that changes is a page edit, and pays the debt.
#[test]
fn a_rewritten_sentence_moves_the_page() {
    let (green, said) = drift(
        "prose",
        "<p>a file sees only its own imports; compiling takes \
         <span data-golden=\"compile\">25,881,551</span></p>",
    );
    assert!(green, "a prose edit moves the page: {said}");
    assert!(said.contains("page drift 0/3"), "nothing is owed after it: {said}");
}

fn git_at(dir: &Path, when: &str, args: &[&str]) {
    let done = Command::new("git")
        .args(args)
        .env("GIT_AUTHOR_DATE", when)
        .env("GIT_COMMITTER_DATE", when)
        .current_dir(dir)
        .output()
        .expect("git runs");
    assert!(done.status.success(), "git {args:?}: {done:?}");
}

/// The page's sections are written on a side branch, the four entries land
/// on the trunk after that, and then the side branch merges. The page moved
/// on the trunk at the merge, so nothing is owed. Walking every commit that
/// touched the page by date reached the side branch's own commit, which is
/// older than the four, and read the page four entries behind.
#[test]
fn a_page_merged_in_moves_where_it_merges() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let stage = std::env::temp_dir().join("kanso-number-edit-merged");
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(stage.join("docs")).expect("a docs directory");
    std::fs::create_dir_all(stage.join("design")).expect("a design directory");

    git(&stage, &["init", "-q", "-b", "trunk", "."]);
    git(&stage, &["config", "user.email", "spec@example.invalid"]);
    git(&stage, &["config", "user.name", "the spec"]);

    std::fs::write(stage.join("design/compiler-log.md"), "# log\n").expect("the log writes");
    std::fs::write(stage.join("docs/compiler.html"), "<p>a first paragraph.</p>\n")
        .expect("the page writes");
    git(&stage, &["add", "-A"]);
    git_at(&stage, "2026-10-12T01:00:00Z", &["commit", "-qm", "the page moves"]);

    git(&stage, &["checkout", "-q", "-b", "sections"]);
    std::fs::write(
        stage.join("docs/compiler.html"),
        "<p>a first paragraph.</p>\n<p>a section for the day.</p>\n",
    )
    .expect("the page grows");
    git(&stage, &["add", "docs/compiler.html"]);
    git_at(&stage, "2026-10-12T02:00:00Z", &["commit", "-qm", "the sections"]);

    git(&stage, &["checkout", "-q", "trunk"]);
    let mut log = String::from("# log\n");
    for entry in ENTRIES {
        log.push_str(&format!("\n## {entry}\n\nwhat it says.\n"));
    }
    std::fs::write(stage.join("design/compiler-log.md"), log).expect("the log grows");
    git(&stage, &["add", "design/compiler-log.md"]);
    git_at(&stage, "2026-10-12T03:00:00Z", &["commit", "-qm", "entries"]);
    git_at(
        &stage,
        "2026-10-12T04:00:00Z",
        &["merge", "-q", "--no-ff", "-m", "the sections merge", "sections"],
    );

    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(root.join("scripts/page_drift"))
        .current_dir(&stage)
        .output()
        .expect("page_drift runs");
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&done.stdout),
        String::from_utf8_lossy(&done.stderr)
    );
    let _ = std::fs::remove_dir_all(&stage);
    assert!(done.status.success(), "the merge moved the page: {said}");
    assert!(said.contains("page drift 0/3"), "nothing is owed after it: {said}");
}
