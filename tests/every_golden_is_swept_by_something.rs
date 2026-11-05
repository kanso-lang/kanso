//! Every gate CI runs against a golden is named by one of the three local
//! sweeps, so a container cannot read "nothing moved" off a board that is
//! missing a row.
//!
//! There are three sweeps and between them they looked like the whole board.
//! On 2026-09-18 they were not: `all_counters.sh` printed that every cost
//! vein and the lazy tier agreed with their goldens, on a tree whose
//! interpreted row had moved 18,540,440 instructions. `interp_instructions`
//! was in neither that list nor `all_compile.sh`'s, along with `interp_memory`,
//! `startup_instructions` and `instructions` -- the last being retired
//! instructions per benchmark, the dimension every allocation counter is blind
//! to. The change was deliberate and CI measured it, so nothing was lost that
//! day. What it showed is that two green sweeps did not mean what a reader
//! would take them to mean.
//!
//! This is the third spec in the family and the widest. The counter sweep
//! reads its veins from the `*_counters.sh` gates themselves, the compile sweep
//! reads its gates from the scripts that name a compile golden, and
//! `the_compile_sweep_names_every_compile_gate.rs` pins that every
//! compile-side golden is read by something the compile sweep runs. Both are
//! scoped to one sweep, and the gap they left was the gates belonging to
//! NEITHER of them, which by construction neither could see.
//!
//! The property here is anchored to CI rather than to a list in this file,
//! because a list in this file is the thing that goes stale. CI running a gate
//! is the obligation: if a pull request can go red on it, a container should be
//! able to check it before pushing, or be told in so many words that it cannot.
//! A gate that refuses on this host still belongs in a sweep -- printing
//! REFUSED says the row is unchecked, and saying nothing says it agreed.

use std::collections::BTreeSet;
use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

const CI: &str = include_str!("../.github/workflows/ci.yml");
const INTERP: &str = include_str!("../scripts/gates/all_interp.sh");

/// Every `scripts/gates/<name>.sh` the workflow invokes.
fn ci_runs() -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut rest = CI;
    while let Some(at) = rest.find("scripts/gates/") {
        rest = &rest[at + "scripts/gates/".len()..];
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_')
            .collect();
        if rest[name.len()..].starts_with(".sh") && !name.is_empty() {
            found.insert(name);
        }
    }
    found
}

/// The text between `word="` and its closing quote.
fn quoted<'a>(script: &'a str, word: &str) -> &'a str {
    script
        .split_once(&format!("{word}=\""))
        .unwrap_or_else(|| panic!("the script declares {word}"))
        .1
        .split_once('"')
        .expect("the declaration is closed")
        .0
}

/// The first word of each line a sweep's `--list` prints. The counter sweep
/// and the compile sweep find their gates in `scripts/gates` rather than
/// declaring them, so they are run rather than read.
fn listed(sweep: &str) -> Vec<String> {
    let out = std::process::Command::new("sh")
        .arg(format!("scripts/gates/{sweep}"))
        .arg("--list")
        .current_dir(root())
        .output()
        .expect("the sweep runs");
    assert!(out.status.success(), "{sweep} --list failed: {out:?}");
    String::from_utf8(out.stdout)
        .expect("the list is text")
        .lines()
        .map(|l| l.split_whitespace().next().expect("a line names its gate").to_string())
        .collect()
}

/// The veins the counter sweep reads.
fn counter_veins() -> Vec<String> {
    listed("all_counters.sh")
}

/// Every gate any sweep names. The counter sweep names a VEIN and runs
/// `<vein>_counters.sh`; the other two name their gates outright.
fn swept() -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for vein in counter_veins() {
        names.insert(format!("{vein}_counters"));
    }
    names.extend(listed("all_compile.sh"));
    for gate in quoted(INTERP, "gates").split_whitespace() {
        names.insert(gate.to_string());
    }
    names
}

fn reads_a_golden(gate: &str) -> bool {
    let path = root().join("scripts/gates").join(format!("{gate}.sh"));
    let Ok(body) = std::fs::read_to_string(&path) else {
        return false;
    };
    // The scan starts AFTER the prefix. Starting at it takes `bench` and stops
    // dead on the slash, which is not in the set below -- so every gate read as
    // naming no golden and this whole spec passed vacuously. It was caught by
    // removing a gate from a sweep and watching the assertion stay green.
    body.match_indices("bench/").any(|(at, _)| {
        let tail = &body[at + "bench/".len()..];
        let name: String = tail
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_' || *c == '.')
            .collect();
        name.contains("golden") && name.ends_with(".txt")
    })
}

#[test]
fn every_golden_reading_gate_ci_runs_is_in_a_sweep() {
    let swept = swept();
    let missing: Vec<String> = ci_runs()
        .into_iter()
        .filter(|g| reads_a_golden(g))
        .filter(|g| !swept.contains(g))
        .collect();
    assert!(
        missing.is_empty(),
        "CI runs these gates against a golden and no local sweep names them, so \
         a container that runs all three sweeps and sees green has not checked \
         them: {missing:?}"
    );
}

/// The sweeps between them must not name a gate that does not exist, which is
/// the other way a list goes wrong.
#[test]
fn no_sweep_names_a_gate_that_is_not_there() {
    let strays: Vec<String> = swept()
        .into_iter()
        .filter(|g| !root().join("scripts/gates").join(format!("{g}.sh")).exists())
        .collect();
    assert!(strays.is_empty(), "a sweep names a gate that does not exist: {strays:?}");
}

/// A sweep that names nothing would pass the first test trivially.
#[test]
fn the_sweeps_are_not_empty() {
    assert!(counter_veins().len() >= 4, "all_counters.sh reads fewer than four veins");
    assert!(listed("all_compile.sh").len() >= 4, "all_compile.sh names fewer than four gates");
    assert!(
        quoted(INTERP, "gates").split_whitespace().count() >= 4,
        "all_interp.sh names fewer than four rows, which is not a sweep"
    );
}
