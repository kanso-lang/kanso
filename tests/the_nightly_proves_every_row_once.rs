//! The nightly ratchet proves the table in shards, and together they prove
//! every row exactly once.
//!
//! One job proving all of it stopped fitting in its ninety minutes on
//! 2026-09-14 and was cancelled every night after, having read its baseline and
//! printed nothing about any row. So `.github/workflows/ratchet.yml` runs a
//! matrix of jobs, each calling `scripts/ratchet -- shard K N`, and two things
//! can quietly undo that. A shard dropped from the matrix leaves a slice of the
//! table unproved while every job that did run is green. And a change to how
//! `shard` picks its rows can leave one out, or prove one twice while leaving
//! another out, with the same green.
//!
//! This reads the matrix and the count off the workflow and asks the harness
//! itself which rows each shard would prove, with `list`, which names them and
//! stops.

use std::collections::BTreeSet;
use std::process::Command;

const NIGHTLY: &str = include_str!("../.github/workflows/ratchet.yml");

fn ratchet(args: &[&str]) -> std::process::Output {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(root.join("scripts/ratchet"))
        .arg("--")
        .args(args)
        .current_dir(root)
        .output()
        .expect("the ratchet runs")
}

fn listed(k: usize, n: usize) -> Vec<String> {
    let out = ratchet(&["shard", &k.to_string(), &n.to_string(), "list"]);
    assert!(
        out.status.success(),
        "shard {k} of {n} would not list: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.starts_with("  "))
        .map(|l| l.trim().to_string())
        .collect()
}

/// The shard numbers the matrix runs, and the count its run line divides by.
fn matrix() -> (Vec<usize>, usize) {
    let line = NIGHTLY
        .lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix("shard: ["))
        .expect("ratchet.yml has a `shard: [...]` matrix");
    let shards = line
        .trim_end_matches(']')
        .split(',')
        .map(|s| s.trim().parse().expect("a shard is a number"))
        .collect();
    let run = NIGHTLY
        .lines()
        .find_map(|l| l.split_once("ratchet -- shard ${{ matrix.shard }} "))
        .expect("the run line proves `shard ${{ matrix.shard }} N`")
        .1;
    (shards, run.trim().parse().expect("N is a number"))
}

#[test]
fn the_matrix_runs_every_shard_the_count_divides_into() {
    let (shards, n) = matrix();
    let every: Vec<usize> = (1..=n).collect();
    assert_eq!(
        shards, every,
        "the matrix runs {shards:?} and the run line divides the table by {n}"
    );
}

#[test]
fn the_shards_prove_every_row_and_none_twice() {
    let (_, n) = matrix();
    let whole = listed(1, 1);
    assert!(whole.len() > 1, "the table listed as {} rows", whole.len());
    let table: BTreeSet<&String> = whole.iter().collect();
    assert_eq!(
        table.len(),
        whole.len(),
        "two rows print the same line, so this spec cannot tell them apart"
    );

    let mut seen: Vec<String> = Vec::new();
    for k in 1..=n {
        seen.extend(listed(k, n));
    }
    let proved: BTreeSet<&String> = seen.iter().collect();
    let missing: Vec<&&String> = table.difference(&proved).collect();
    assert!(missing.is_empty(), "no shard proves {missing:#?}");
    assert_eq!(
        seen.len(),
        whole.len(),
        "{} rows proved across the shards, {} in the table",
        seen.len(),
        whole.len()
    );
}

/// A shard outside 1..=N would prove nothing, or rows another shard already
/// proves, and exit green. It is refused instead.
#[test]
fn a_shard_outside_its_count_is_refused() {
    for (k, n) in [("0", "8"), ("9", "8")] {
        let out = ratchet(&["shard", k, n, "list"]);
        assert!(!out.status.success(), "shard {k} of {n} was accepted");
        assert!(
            String::from_utf8_lossy(&out.stderr).contains("a shard is `shard K N`"),
            "shard {k} of {n} failed for another reason: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
