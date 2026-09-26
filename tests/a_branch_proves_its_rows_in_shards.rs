//! A pull request proves the rows its diff could have made blind in shards,
//! and together the shards prove each of those rows exactly once.
//!
//! A branch touching src/runtime.c selected 78 rows on that file alone on
//! 2026-09-26, and one job proving such a handful held kanso#1670 for two and
//! a half hours after every other job was green. So ci.yml runs a matrix of jobs, each calling
//! `scripts/ratchet -- touched origin/main shard K N`, and two things can
//! quietly undo that. A shard dropped from the matrix leaves part of the
//! handful unproved while every job that ran is green. And a change to how a
//! shard picks its rows can leave one out, or prove one twice while leaving
//! another out, with the same green.
//!
//! This reads the matrix and the count off ci.yml, makes a branch that touches
//! src/runtime.c in a worktree of HEAD, and asks the harness which rows each
//! shard would prove, with `list`, which names them and stops.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

const CI: &str = include_str!("../.github/workflows/ci.yml");

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

/// The shard numbers the matrix runs, and the count its run line divides by.
fn matrix() -> (Vec<usize>, usize) {
    let line = CI
        .lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix("shard: ["))
        .expect("ci.yml has a `shard: [...]` matrix");
    let shards = line
        .trim_end_matches(']')
        .split(',')
        .map(|s| s.trim().parse().expect("a shard is a number"))
        .collect();
    let run = CI
        .lines()
        .find_map(|l| l.split_once("ratchet -- touched origin/main shard ${{ matrix.shard }} "))
        .expect("the run line proves `touched origin/main shard ${{ matrix.shard }} N`")
        .1;
    (shards, run.trim().parse().expect("N is a number"))
}

/// A worktree of HEAD with one commit on it that adds a line to
/// src/runtime.c, and the commit it branched from.
struct Branch {
    tree: PathBuf,
    base: String,
}

impl Branch {
    fn touching_the_runtime() -> Branch {
        let tree = std::env::temp_dir().join("kanso-touched-shards");
        discarded(&tree);
        git(
            root(),
            &["worktree", "add", "--detach", "--force", tree.to_str().expect("a path"), "HEAD"],
        );
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
        let held = tree.join("src/runtime.c");
        let mut text = std::fs::read_to_string(&held).expect("the runtime reads");
        text.push_str("\n/* a line this branch added */\n");
        std::fs::write(&held, text).expect("the runtime writes");
        // `-c` passes the identity for this one command and writes nothing to
        // the repository's shared config, which a linked worktree would.
        git(
            &tree,
            &[
                "-c",
                "user.email=spec@kanso.invalid",
                "-c",
                "user.name=spec",
                "commit",
                "--quiet",
                "-a",
                "-m",
                "the branch",
            ],
        );
        Branch { tree, base }
    }

    fn ratchet(&self, args: &[&str]) -> (bool, String) {
        let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("run")
            .arg(root().join("scripts/ratchet"))
            .arg("--")
            .args(["touched", &self.base])
            .args(args)
            .current_dir(&self.tree)
            .output()
            .expect("the ratchet runs");
        let said = format!(
            "{}{}",
            String::from_utf8_lossy(&done.stdout),
            String::from_utf8_lossy(&done.stderr)
        );
        (done.status.success(), said)
    }

    /// The rows a listing names, one per line behind two spaces.
    fn listed(&self, args: &[&str]) -> Vec<String> {
        let (ok, said) = self.ratchet(args);
        assert!(ok, "`touched {args:?}` would not list:\n{said}");
        said.lines().filter(|l| l.starts_with("  ")).map(|l| l.trim().to_string()).collect()
    }
}

impl Drop for Branch {
    fn drop(&mut self) {
        discarded(&self.tree);
    }
}

fn discarded(tree: &Path) {
    let _ = Command::new("git")
        .args(["worktree", "remove", "--force"])
        .arg(tree)
        .current_dir(root())
        .output();
    let _ = std::fs::remove_dir_all(tree);
}

#[test]
fn the_matrix_runs_every_shard_the_count_divides_into() {
    let (shards, n) = matrix();
    let every: Vec<usize> = (1..=n).collect();
    assert_eq!(
        shards, every,
        "the matrix runs {shards:?} and the run line divides the rows by {n}"
    );
}

/// All three questions on one branch, because the worktree and its commit are
/// most of what each one costs.
#[test]
fn the_shards_of_a_runtime_branch_prove_its_rows_once() {
    let branch = Branch::touching_the_runtime();
    let whole = branch.listed(&["list"]);
    assert!(
        whole.len() > 1,
        "a runtime branch selects more than one row, so there is something to divide:\n{whole:?}"
    );

    let (_, n) = matrix();
    let mut seen = BTreeSet::new();
    let mut twice = Vec::new();
    for k in 1..=n {
        for row in branch.listed(&["shard", &k.to_string(), &n.to_string(), "list"]) {
            if !seen.insert(row.clone()) {
                twice.push(row);
            }
        }
    }
    assert!(twice.is_empty(), "rows two shards would both prove: {twice:?}");
    let wanted: BTreeSet<String> = whole.into_iter().collect();
    let dropped: Vec<&String> = wanted.difference(&seen).collect();
    assert!(dropped.is_empty(), "rows the branch selected and no shard proves: {dropped:?}");
    let invented: Vec<&String> = seen.difference(&wanted).collect();
    assert!(
        invented.is_empty(),
        "rows a shard proves that the branch never selected: {invented:?}"
    );

    // More shards than rows leaves the last ones nothing. That shard proves
    // nothing, claims nothing, and passes: its rows went to the others.
    let past = (wanted.len() + 1).to_string();
    let (ok, said) = branch.ratchet(&["shard", &past, &past]);
    assert!(ok, "an empty shard is not a failure:\n{said}");
    assert!(
        said.contains("no row falls to this shard"),
        "an empty shard says it is empty:\n{said}"
    );
    assert!(
        !said.contains("every row turned its gate red"),
        "an empty shard claims no row was proved:\n{said}"
    );

    let (ok, said) = branch.ratchet(&["shard", "5", "4", "list"]);
    assert!(!ok, "a shard past the count is refused:\n{said}");
    assert!(said.contains("not 5 of 4"), "the refusal names the shard it was given:\n{said}");
}
