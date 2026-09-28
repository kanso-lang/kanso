//! Every callgrind gate that takes a second reading of itself has both of its
//! profiles copied out of the cost-goldens job.
//!
//! A row that reads apart between two runners can only be explained by
//! diffing the two runs' profiles, and a job keeps nothing once it ends. ci.yml
//! has a step that copies `/tmp/cg.<name>` and `/tmp/cg.<name>2` into the
//! artifact for each name in one `for n in ...` line. That line said
//! `compile entry library` until 2026-09-28, and the interpreter row, which
//! is the counter STATUS.md's "A welfare counter reads three parts per
//! billion" row is about, was never kept: the next time it read six apart
//! there would have been nothing to compare.
//!
//! So this reads the gates off disk, finds each name written both as
//! `--callgrind-out-file=/tmp/cg.<name>` and as `/tmp/cg.<name>2`, and asserts
//! the copy line names exactly that set.

use std::collections::BTreeSet;
use std::path::Path;

const CI: &str = include_str!("../.github/workflows/ci.yml");

/// Names a gate writes a profile under, from `--callgrind-out-file=/tmp/cg.X`.
fn written() -> BTreeSet<String> {
    let gates = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/gates");
    let mut names = BTreeSet::new();
    for entry in std::fs::read_dir(&gates).expect("scripts/gates is readable") {
        let path = entry.expect("a directory entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("sh") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("a gate is readable");
        for piece in text.split("--callgrind-out-file=/tmp/cg.").skip(1) {
            let name: String =
                piece.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
            if !name.is_empty() {
                names.insert(name);
            }
        }
    }
    names
}

/// The names whose second reading is also written: `X` where `X2` is.
fn read_twice() -> BTreeSet<String> {
    let all = written();
    all.iter().filter(|n| all.contains(&format!("{n}2"))).cloned().collect()
}

/// The names on the copy step's `for n in ...; do` line.
fn copied() -> BTreeSet<String> {
    let step = CI
        .split("- name: the profiles this job counted")
        .nth(1)
        .expect("ci.yml has a step named `the profiles this job counted`");
    let line = step
        .lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix("for n in "))
        .expect("the step copies profiles in a `for n in ...` loop");
    line.trim_end_matches("; do").split_whitespace().map(String::from).collect()
}

#[test]
fn every_profile_read_twice_is_copied_out() {
    let twice = read_twice();
    assert!(
        twice.contains("interp") && twice.contains("compile"),
        "the gate scan found {twice:?}, which is missing a row it must see"
    );
    let copied = copied();
    let missing: Vec<_> = twice.difference(&copied).collect();
    assert!(
        missing.is_empty(),
        "the gates read {missing:?} twice and the artifact step does not copy them"
    );
    let stale: Vec<_> = copied.difference(&twice).collect();
    assert!(
        stale.is_empty(),
        "the artifact step copies {stale:?}, which no gate writes with a second reading"
    );
}
