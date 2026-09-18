//! A knot that a run demands allocates one cell, and both engines say so.
//!
//! Ruled 2026-08-24, on the archive entry "a demanded knot counts, and the
//! oracle moves". The day before had found it: native reports
//! `thunk_allocs=1` where the oracle reports `0`, because the oracle's
//! `knotted` builds its cell without touching the counter. The gavel names
//! which side moves -- the oracle -- and calls it bookkeeping with no
//! semantic change anywhere.
//!
//! What was missing was the FIXTURE, not the comparison. That claim was made
//! the other way round first, on a reading of `tests/golden.rs` -- which does
//! run the mem vein on one engine only. `tests/oracle.rs` runs the other:
//! `mem_corpus_interp_matches_the_semantic_counters` has asserted the interp's
//! thunk triple against those same goldens all along. It stayed green because
//! the corpus held one knot case and that one was undemanded, so both engines
//! read zero and agreed by saying nothing. Adding
//! `a_demanded_knot_allocates_one_cell` to the vein turns that loop red
//! against the unfixed interpreter, which was checked rather than assumed.
//!
//! This file is still worth its own round: it enters where a user enters, by
//! running the real binary both ways rather than compiling in-process, and it
//! PINS the triple instead of only comparing the two engines to each other.
//!
//! The counters are semantics, not a resource heuristic: an engine may
//! choose when to do work, and `an_undemanded_knot_allocates_nothing` pins
//! that choice, but it may not disagree about what the work WAS. So this
//! file asserts the whole triple on both engines rather than the one row
//! that moved, because a fix that bumped the counter in the wrong place
//! would satisfy a narrower assertion.

use std::path::PathBuf;
use std::process::Command;

/// The knot is the second arm's, so `picked 2` is what demands it. The first
/// arm is the undemanded fixture's case and is not exercised here.
const LIBRARY: &str = r#"import "std/io"
import "std/list"

x = [x]

pub play = picked 2

fn picked 1
  io/write "one\n"

fn picked _
  io/write "{length (list/to_list x)}\n"
"#;

fn kanso() -> PathBuf {
    let mut exe = std::env::current_exe().expect("the test binary has a path");
    exe.pop();
    if exe.ends_with("deps") {
        exe.pop();
    }
    exe.join("kanso")
}

/// Stage the library beside the entry that imports it, the way the mem vein
/// reaches its fixtures, and answer what the run printed and counted.
fn ran(interp: bool) -> (String, Vec<(String, u64)>) {
    let stage = std::env::temp_dir().join(format!("kanso-demanded-knot-{}", interp as u8));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).expect("a staging directory");
    std::fs::write(stage.join("demanded.kso"), LIBRARY).expect("the library writes");
    let entry = stage.join("run_demanded.kso");
    std::fs::write(&entry, "import \"./demanded\"\n\ndemanded/play\n").expect("the entry writes");

    let mut command = Command::new(kanso());
    command.arg("run").arg(&entry).env("KANSO_COUNTERS", "1");
    if interp {
        command.arg("--interp");
    }
    let out = command.output().expect("kanso runs");
    assert!(
        out.status.success(),
        "the run failed ({}):\n{}",
        if interp { "oracle" } else { "native" },
        String::from_utf8_lossy(&out.stderr)
    );

    let counted = String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter_map(|line| line.split_once('='))
        .filter(|(key, _)| key.starts_with("thunk_"))
        .filter_map(|(key, value)| Some((key.to_string(), value.trim().parse().ok()?)))
        .collect();
    let printed = String::from_utf8_lossy(&out.stdout).to_string();
    let _ = std::fs::remove_dir_all(&stage);
    (printed, counted)
}

/// What each engine must say. Pinned rather than compared to each other
/// alone, because two engines agreeing on a wrong number is the failure a
/// differential assertion cannot see.
fn expected() -> Vec<(&'static str, u64)> {
    vec![("thunk_allocs", 1), ("thunk_forces", 1), ("thunk_evals", 1)]
}

fn shared(counted: &[(String, u64)]) -> Vec<(&str, u64)> {
    let wanted = expected();
    counted
        .iter()
        .filter(|(key, _)| wanted.iter().any(|(name, _)| name == key))
        .map(|(key, value)| (key.as_str(), *value))
        .collect()
}

#[test]
fn the_native_engine_counts_one_cell_for_a_demanded_knot() {
    let (printed, counted) = ran(false);
    assert_eq!(printed, "1\n", "the demanded knot prints its length");
    assert_eq!(shared(&counted), expected());
}

#[test]
fn the_oracle_counts_one_cell_for_a_demanded_knot() {
    let (printed, counted) = ran(true);
    assert_eq!(printed, "1\n", "the demanded knot prints its length");
    assert_eq!(shared(&counted), expected());
}

#[test]
fn the_two_engines_agree_on_every_thunk_counter() {
    let (native_out, native) = ran(false);
    let (oracle_out, oracle) = ran(true);
    assert_eq!(native_out, oracle_out, "the engines print the same thing");
    assert_eq!(shared(&native), shared(&oracle), "native {native:?} against oracle {oracle:?}");
}
