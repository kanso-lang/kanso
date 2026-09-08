//! Every counter-bearing golden in bench/ is one the trend gate walks.
//!
//! The gate reads a hand-written list of `[["bench/…" "prefix_"]]` bindings,
//! and that list has been short five times. Three cost goldens were simply
//! never entered. Then readbench joined the OBJECTIVE with two of its rows as
//! welfare terms while this gate could not see its golden at all, so either
//! row could have moved by any amount in silence. Then livebench. Then the
//! consolidated run program, whose four rows are the objective's entire
//! runtime side. The gate's own comments say how each was found: "by asking
//! which files in bench/ this program names, which is now the only method
//! that has ever found one of these."
//!
//! This asks it mechanically. A golden the gate does not walk is one whose
//! regressions arrive unpriced, and finding that by hand a sixth time is not
//! a plan.
//!
//! The excuse list is one file long and carries its reason, the way the page
//! sweep's and the compile sweep's do. An entry here is a claim about what a
//! file holds, so it says what that is.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

const GATE: &str = include_str!("../scripts/trend_gate/trend_gate.kso");

/// `bench/compile_libraries_golden.txt` holds five sonames rather than
/// counters, and `scripts/gates/compile_libraries.sh` diffs it byte for byte.
/// There is no number in it to trend. The gate's own comment on `bench_read`
/// already says so, and this repeats it here because a reader of the excuse
/// list should not have to go looking.
const EXCUSED: [(&str, &str); 1] = [(
    "compile_libraries_golden.txt",
    "five sonames rather than counters; its own gate diffs it byte for byte",
)];

/// Every `[["bench/…" …]]` binding the gate names.
fn walked() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in GATE.lines() {
        let Some(open) = line.find("[[\"bench/") else { continue };
        let rest = &line[open + 3..];
        let Some(close) = rest.find('"') else { continue };
        out.insert(rest[..close].trim_start_matches("bench/").to_string());
    }
    out
}

/// Every golden in bench/, by file name.
fn goldens_on_disk() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for entry in std::fs::read_dir(root().join("bench")).expect("the bench directory reads") {
        let path = entry.expect("a directory entry reads").path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
        if name.contains("golden") && name.ends_with(".txt") {
            out.insert(name.to_string());
        }
    }
    out
}

#[test]
fn the_gate_walks_every_golden_it_does_not_excuse() {
    let walked = walked();
    assert!(!walked.is_empty(), "the gate names at least one golden");
    let excused: BTreeSet<&str> = EXCUSED.iter().map(|(name, _)| *name).collect();
    let missing: Vec<_> = goldens_on_disk()
        .into_iter()
        .filter(|g| !walked.contains(g) && !excused.contains(g.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "scripts/trend_gate/trend_gate.kso does not walk {missing:?}. A golden \
         the gate does not walk is one whose regressions arrive unpriced — \
         list it beside its siblings, or excuse it here with what it holds \
         instead of counters."
    );
}

/// And an excuse names a file that is there. A stale excuse is how a list
/// stops being checked: the name stops matching anything, the entry stops
/// meaning anything, and the file it was written for is long gone.
#[test]
fn every_excuse_names_a_golden_that_exists() {
    let on_disk = goldens_on_disk();
    for (name, why) in EXCUSED {
        assert!(
            on_disk.contains(name),
            "this spec excuses {name} ({why}) and bench/ has no such file"
        );
    }
}

/// And an excused golden is one the gate really does not walk. An excuse for
/// a file already on the list reads as coverage the list does not owe, and
/// hides the day the list drops it.
#[test]
fn no_excuse_covers_a_golden_the_gate_already_walks() {
    let walked = walked();
    let redundant: Vec<_> =
        EXCUSED.iter().map(|(name, _)| *name).filter(|name| walked.contains(*name)).collect();
    assert!(
        redundant.is_empty(),
        "the gate walks {redundant:?} and this spec also excuses them — an \
         excuse for a listed golden hides the day it leaves the list"
    );
}
