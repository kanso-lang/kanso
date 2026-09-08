//! `scripts/page_drift` counts log entries written since docs/compiler.html
//! last moved, and fails past a budget of three. The count stood in for a
//! question it cannot ask — has the presented design fallen behind what the
//! compiler does — and Clay ruled on 2026-09-08 that it counts the wrong
//! thing: "there is no specific correlation between a number of log entries
//! and specific changes to the HTML."
//!
//! The whole-float gavel is the worked example. It was ruled on 2026-09-06
//! and filed as one entry; the behaviour it decided shipped in kanso#1285 as
//! a second; the page sentence it falsified was corrected in kanso#1315 as a
//! third. One page edit was owed across the three, and the ruling was not the
//! entry that owed it. So rulings come out of the count and the entries
//! recording what shipped stay in.
//!
//! The gate had no spec at all before this one, which is why the defect
//! survived a redesign of the log's heading conventions underneath it.

use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let done = Command::new("git").args(args).current_dir(dir).output().expect("git runs");
    assert!(done.status.success(), "git {args:?}: {done:?}");
}

/// Build a repository whose page moved once, then append `entries` to the log
/// as one commit, and answer what the gate says about it.
fn drift(key: &str, entries: &[&str]) -> (bool, String) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let stage = std::env::temp_dir().join(format!("kanso-drift-{key}"));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(stage.join("docs")).expect("a docs directory");
    std::fs::create_dir_all(stage.join("design")).expect("a design directory");

    git(&stage, &["init", "-q", "."]);
    git(&stage, &["config", "user.email", "spec@example.invalid"]);
    git(&stage, &["config", "user.name", "the spec"]);

    std::fs::write(stage.join("design/compiler-log.md"), "# log\n").expect("the log writes");
    std::fs::write(stage.join("docs/compiler.html"), "<p>the page</p>\n").expect("the page writes");
    git(&stage, &["add", "-A"]);
    git(&stage, &["commit", "-qm", "the page moves"]);

    let mut log = String::from("# log\n");
    for entry in entries {
        log.push_str(&format!("\n## {entry}\n\nwhat it says.\n"));
    }
    std::fs::write(stage.join("design/compiler-log.md"), log).expect("the log grows");
    git(&stage, &["add", "design/compiler-log.md"]);
    git(&stage, &["commit", "-qm", "entries"]);

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

/// The shape kanso#1313 arrived in: nine rulings and one entry that is not
/// one. Before the fix this read 10/3 and failed the whole cost-goldens job.
#[test]
fn a_batch_of_rulings_owes_the_page_nothing() {
    let (green, said) = drift(
        "rulings",
        &[
            "2026-09-05 — gavel: corpus first",
            "2026-09-05 — gavel: no machine-code-size term in welfare",
            "2026-09-05 — gavel: one row, one value",
            "2026-09-06 — gavel: bump clang to 19",
            "2026-09-06 — gavel: a whole float keeps its point",
            "2026-09-06 — gavel: one consolidated run program",
            "2026-09-06 — directive: the framework's features wait",
            "2026-09-07 — gavel: the welfare history's baseline",
            "2026-09-08 — gavel: page_drift counts the wrong thing",
            "2026-09-07 — the one-program gavel re-priced the declined queue",
        ],
    );
    assert!(green, "nine rulings and one entry is one entry: {said}");
    assert!(said.contains("page drift 1/3"), "the one non-ruling is the count: {said}");
    assert!(
        said.contains("(9 rulings not counted)"),
        "what was skipped is printed, not swallowed: {said}"
    );
}

/// The exemption is not a way past the gate. Four entries recording shipped
/// work are four entries, and the page still owes them.
#[test]
fn four_shipped_entries_still_spend_the_budget() {
    let (green, said) = drift(
        "shipped",
        &[
            "2026-09-07 — a beat pop with nothing to do",
            "2026-09-07 — the sizing walk called on every immediate",
            "2026-09-07 — every constant is built once on native",
            "2026-09-07 — three more shortcuts on the text phases",
        ],
    );
    assert!(!green, "four is past a budget of three: {said}");
    assert!(said.contains("the log is 4 entries ahead"), "the count is the four: {said}");
    assert!(said.contains("a beat pop with nothing to do"), "the entries are named: {said}");
}

/// The marker is `— gavel:`, and an entry that merely talks about a gavel is
/// not one. Without this the exemption would be a word anybody could sprinkle
/// into a heading to buy their way past the budget.
#[test]
fn an_entry_that_mentions_a_gavel_is_not_a_ruling() {
    let (green, said) = drift(
        "mentions",
        &[
            "2026-09-07 — the one-program gavel re-priced the declined queue",
            "2026-09-07 — a gavel landed and this is what it cost",
            "2026-09-07 — the gavel: not a ruling, just a colon",
            "2026-09-07 — what the directive asked for, measured",
        ],
    );
    assert!(!green, "four mentions are four entries: {said}");
    assert!(said.contains("the log is 4 entries ahead"), "none of the four is exempt: {said}");
}
