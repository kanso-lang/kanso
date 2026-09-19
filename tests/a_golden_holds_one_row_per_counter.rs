//! One golden file, one row per counter name.
//!
//! `rows()` in tests/the_objective_reads_what_the_gate_watches.rs adds every
//! row it reads into a map keyed by name:
//!
//!     *out.entry(format!("{prefix}{}", name.trim())).or_default() += n;
//!
//! The `+=` is right and has to stay. A cost golden holds one row per sample
//! and the gate's own `totals` sums them, and `bench/objective_sources.txt`
//! maps one objective counter onto several gate keys. Summing is the whole
//! shape of the thing.
//!
//! What the `+=` cannot tell apart is a second SAMPLE from a second copy of
//! the same row. A golden that carries `codegen_instructions_release=` twice
//! reads as their sum, and nothing says so at the point of the mistake.
//!
//! WHAT IT COST, 2026-09-18. Two branches merged main into a golden by
//! appending the carried-forward row under a header block instead of replacing
//! the row already there. kanso#1504 carried 6,824,133,280 and 6,841,691,425
//! in one file; the gate read 13,665,824,705, exactly the two added.
//! kanso#1502 carried 6,824,133,280 and 6,820,866,344 the same way. Both sat
//! green through the counter sweep, the compile sweep and the three page
//! gates, because not one of those reads a golden for a repeated key.
//!
//! What did speak was `the_objective_reads_what_the_gate_watches`, and what it
//! said was:
//!
//!     `codegen_instructions_release` reads 6841691425 from welfare and
//!     13665824705 from codegen_instructions_release. The link is wrong or
//!     a pool joined the sum.
//!
//! The link was fine and no pool had joined anything. That spec compares
//! totals, so a doubled row reaches it as a wrong total and it reports the
//! last thing that could have caused one. This file reports the first: the
//! file, the name, and how many times it appears.
//!
//! IT IS DELIBERATELY NOT A VALUE CHECK. Whether a row holds the right number
//! is what the gates are for and what CI measures. This asserts only that
//! there is one of it to read, which is the property a merge breaks and no
//! measurement can restore.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// The gate's own list, read the way
/// tests/the_objective_reads_what_the_gate_watches.rs reads it, so a golden
/// added to the gate is covered here without a second list to keep.
fn watched(gate: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in gate.lines() {
        let Some(open) = line.find("[[\"bench/") else { continue };
        let rest = &line[open + 3..];
        let mut parts = rest.split('"').filter(|p| !p.contains('[') && !p.contains(']'));
        if let Some(file) = parts.next() {
            out.push(file.to_string());
        }
    }
    out
}

/// A golden's numeric rows, in the two spellings the goldens use: `name=value`
/// and `name value`. Comments and blanks are skipped, exactly as the reader
/// under test skips them.
fn names(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let name = match line.split_once('=') {
            Some((name, value)) => {
                if value.trim().parse::<i128>().is_err() {
                    continue;
                }
                name
            }
            None => {
                let mut words = line.split_whitespace();
                match (words.next(), words.next(), words.next()) {
                    (Some(n), Some(v), None) if v.parse::<i128>().is_ok() => n,
                    _ => continue,
                }
            }
        };
        out.push(name.trim().to_string());
    }
    out
}

#[test]
fn a_golden_holds_one_row_per_counter() {
    let root = root();
    let gate = std::fs::read_to_string(root.join("scripts/trend_gate/trend_gate.kso"))
        .expect("the trend gate is where the goldens are named");
    let files = watched(&gate);
    assert!(
        files.len() > 20,
        "the gate named {} goldens, so this spec is reading nothing",
        files.len()
    );

    let mut read = 0usize;
    let mut repeated: Vec<String> = Vec::new();
    for file in &files {
        let Ok(text) = std::fs::read_to_string(root.join(file)) else { continue };
        read += 1;
        let mut count: HashMap<String, usize> = HashMap::new();
        for name in names(&text) {
            *count.entry(name).or_default() += 1;
        }
        let mut over: Vec<_> = count.into_iter().filter(|(_, n)| *n > 1).collect();
        over.sort();
        for (name, n) in over {
            repeated.push(format!("{file}: `{name}` appears {n} times"));
        }
    }
    assert!(read > 20, "only {read} of the gate's goldens could be opened");

    assert!(
        repeated.is_empty(),
        "a golden holds one row per counter, and these hold more:\n  {}\n\n\
         The reader in tests/the_objective_reads_what_the_gate_watches.rs adds \
         rows with `+=`, so a repeated name is read as the SUM of its copies and \
         the gate compares a number no measurement produced. This is what a merge \
         does when it appends the carried-forward row under a new header block \
         instead of replacing the row already there: delete the stale row, keep \
         one, and say in the header which reading the remaining one is.",
        repeated.join("\n  ")
    );
}
