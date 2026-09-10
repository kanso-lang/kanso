//! Every got file a cost gate writes is printed where the log's tail reaches.
//!
//! A gate that disagrees with its golden prints the row it measured, and a
//! session regenerating that golden copies the row out of CI's job log. The
//! cost-goldens job runs nineteen counter steps and two callgrind dumps after
//! the compile gates, so a gate's own message is thousands of lines from the
//! end and the log API hands back a tail. `ci.yml` answers that with a step
//! near the end that cats the got files back out.
//!
//! That step named its files by hand: `compile_ir_got.txt` arrived with the
//! first compile vein in kanso#1214, `entry_ir_got.txt` was added by hand in
//! kanso#1330 and `library_ir_got.txt` in kanso#1337, the second at the cost
//! of a round. Two more existed and were never added. On kanso#1361 the
//! `compile_allocs` row went red, its measured value was in neither the tail
//! nor the printing step, and the gate refuses to measure off a host the
//! golden names — so the number could not be read at all and the round was
//! spent copying the three rows that were reachable.
//!
//! The step globs `*_got.txt` now. This spec pins the property the glob
//! satisfies rather than the glob itself, because the failure mode is somebody
//! writing the list out by hand again: every got file any gate writes must be
//! one the printing step will cat.

use std::collections::BTreeSet;
use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

const CI: &str = include_str!("../.github/workflows/ci.yml");

/// The `for f in ...; do ... cat "$f"; done` list in the printing step.
fn printed() -> Vec<String> {
    let step = CI
        .split_once("the rows as measured here, to copy into the goldens")
        .expect("ci.yml carries the printing step")
        .1;
    let list = step
        .split_once("for f in ")
        .expect("the step loops over a file list")
        .1
        .split_once(';')
        .expect("the list is closed")
        .0;
    list.split_whitespace().map(str::to_string).collect()
}

/// Every `<name>_got.txt` written by a script under scripts/gates.
fn written() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for entry in std::fs::read_dir(root().join("scripts/gates")).expect("the gates directory reads")
    {
        let path = entry.expect("a directory entry reads").path();
        if path.extension().and_then(|e| e.to_str()) != Some("sh") {
            continue;
        }
        let body = std::fs::read_to_string(&path).expect("a gate reads");
        for word in body.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.')) {
            if word.ends_with("_got.txt") && word.len() > "_got.txt".len() {
                out.insert(word.to_string());
            }
        }
    }
    out
}

/// A got file is printed when the list names it outright or a glob covers it.
fn covered(name: &str, list: &[String]) -> bool {
    list.iter().any(|pattern| {
        if let Some(suffix) = pattern.strip_prefix('*') {
            name.ends_with(suffix)
        } else {
            pattern == name
        }
    })
}

#[test]
fn every_got_file_a_gate_writes_is_printed_by_the_job_log() {
    let list = printed();
    assert!(!list.is_empty(), "the printing step names files");
    let files = written();
    assert!(
        files.len() >= 5,
        "the gates write at least the five got files this spec was written over, found {files:?}"
    );
    let missed: Vec<&String> = files.iter().filter(|f| !covered(f, &list)).collect();
    assert!(
        missed.is_empty(),
        "these got files are written by a gate and never printed, so a red row's \
         measured value cannot be read out of CI's job log: {missed:?}. The step's \
         list is {list:?}."
    );
}
