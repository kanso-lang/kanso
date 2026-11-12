//! The trend gate walks bench/ itself, so the only hand-kept parts are its
//! `prefixed` rows and its `not_counters` list. Each must name a file that is
//! there: a row whose file is gone stops meaning anything, and a golden
//! renamed out from under its prefix would have its fields summed under bare
//! names, beside a vein they collide with.

#[path = "support/trend_goldens.rs"]
mod trend_goldens;

use std::path::Path;

fn gate() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(root.join("scripts/trend_gate/trend_gate.kso"))
        .expect("the trend gate reads")
}

fn on_disk(path: &str) -> bool {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(path).is_file()
}

#[test]
fn every_prefix_names_a_golden_that_exists() {
    let rows = trend_goldens::prefixed(&gate());
    assert!(rows.len() > 10, "the gate's `prefixed` list read as {} rows", rows.len());
    let gone: Vec<_> = rows.iter().map(|(p, _)| p).filter(|p| !on_disk(p)).collect();
    assert!(gone.is_empty(), "the trend gate prefixes {gone:?}, and bench/ has no such file");
}

#[test]
fn every_golden_kept_out_exists() {
    let kept_out = trend_goldens::rows(&gate(), "not_counters = [");
    assert!(!kept_out.is_empty(), "the gate's `not_counters` list read as empty");
    let gone: Vec<_> = kept_out.iter().filter(|p| !on_disk(p)).collect();
    assert!(gone.is_empty(), "the trend gate keeps out {gone:?}, and bench/ has no such file");
}

#[test]
fn the_walk_reaches_the_goldens_on_disk() {
    let walked = trend_goldens::walked();
    assert!(walked.len() > 20, "the walk found {} goldens", walked.len());
    assert!(
        walked.iter().any(|(p, x)| p == "bench/compile_allocs_golden.txt" && x.is_empty()),
        "an unprefixed golden is walked with no prefix"
    );
    assert!(
        walked.iter().any(|(p, x)| p == "bench/cost_golden_run.txt" && x == "run_"),
        "a prefixed golden is walked with its prefix"
    );
    assert!(
        !walked.iter().any(|(p, _)| p == "bench/compile_libraries_golden.txt"),
        "a golden kept out is not walked"
    );
}
