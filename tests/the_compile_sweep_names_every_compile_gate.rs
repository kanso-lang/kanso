//! The compile-side sweep reads every compile-side golden.
//!
//! `scripts/gates/all_counters.sh` walks the RUNTIME cost goldens and nothing
//! else. The compile side is separate gates, two of whose counters are
//! welfare terms, and a change under `lib/` moves every one of them because
//! the library is `include_str!`'d into the compiler. On 2026-09-05 a
//! twelve-line library change read as a welfare RISE with those veins stale and
//! a FALL once they were regenerated, and nothing in the sweep beside them
//! would have said so.
//!
//! `all_compile.sh` closes that. It finds its gates in `scripts/gates` by the
//! goldens they name, as `all_counters.sh` finds its veins, so there is no list
//! here to keep in step with it (kanso#2345). What this file pins is the
//! plumbing: `--list` runs and names gates that exist, every compile golden on
//! disk is read by something the sweep runs, and the sweep builds before it
//! reads.

use std::collections::BTreeSet;
use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

const SWEEP: &str = include_str!("../scripts/gates/all_compile.sh");

/// The gates the sweep runs, as `--list` prints them.
fn named() -> BTreeSet<String> {
    let out = std::process::Command::new("sh")
        .arg("scripts/gates/all_compile.sh")
        .arg("--list")
        .current_dir(root())
        .output()
        .expect("the sweep runs");
    assert!(out.status.success(), "all_compile.sh --list failed: {out:?}");
    String::from_utf8(out.stdout).expect("the list is text").lines().map(str::to_string).collect()
}

/// `--list` names real gates, and enough of them to be a sweep. A derivation
/// that matched nothing would leave every test below it passing vacuously.
#[test]
fn the_sweep_lists_gates_that_exist() {
    let named = named();
    assert!(named.len() >= 4, "all_compile.sh --list names {named:?}, which is not a sweep");
    for gate in &named {
        let path = root().join(format!("scripts/gates/{gate}.sh"));
        assert!(path.exists(), "the sweep names {gate}, which is not in scripts/gates");
    }
    for skipped in ["all_compile", "all_counters", "build_benchmarks"] {
        assert!(!named.contains(skipped), "the sweep names {skipped}, which is not a gate");
    }
}

/// And every compile-side GOLDEN is reachable from the sweep — which the test
/// above does not say, because it derives from gate SCRIPTS and a golden read
/// by a cargo test names no script at all.
///
/// That gap was not hypothetical. `bench/compile_golden_modules.txt` is read by
/// `tests/compile_cost.rs` and by nothing under `scripts/gates`, so on
/// 2026-09-06 a library change moved it, `all_compile.sh` reported every gate
/// AGREED, and CI failed twice on the one target the sweep could not see. The
/// property above was true the whole time. It was the wrong property: a sweep
/// is covering when every golden has a reader in it, not when every reader that
/// happens to be a script is listed.
fn compile_goldens_on_disk() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for entry in std::fs::read_dir(root().join("bench")).expect("the bench directory reads") {
        let path = entry.expect("a directory entry reads").path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
        // the runtime side is `all_counters.sh`'s, and `every_counter_gate_is_in_the_sweep.rs`
        // carries this same property for it
        if name.starts_with("cost_golden") || name == "instructions_golden.txt" {
            continue;
        }
        if (name.starts_with("compile_") && name.contains("golden"))
            || (name.starts_with("entry_") && name.contains("golden"))
            || (name.starts_with("library_") && name.contains("golden"))
            || name == "text_golden.txt"
            || name.starts_with("emitted_golden")
        {
            out.insert(name.to_string());
        }
    }
    out
}

/// Everything the sweep actually runs, as text: the gate scripts it names, plus
/// the body of any cargo test it invokes with `--test`.
fn what_the_sweep_reads() -> String {
    let mut body = String::new();
    for gate in named() {
        let path = root().join(format!("scripts/gates/{gate}.sh"));
        body.push_str(&std::fs::read_to_string(&path).expect("a named gate reads"));
    }
    for rest in SWEEP.split("--test ").skip(1) {
        let target = rest.split_whitespace().next().unwrap_or_default();
        let path = root().join(format!("tests/{target}.rs"));
        if let Ok(text) = std::fs::read_to_string(&path) {
            body.push_str(&text);
        }
    }
    body
}

#[test]
fn every_compile_golden_has_a_reader_in_the_sweep() {
    let read = what_the_sweep_reads();
    let orphans: Vec<_> =
        compile_goldens_on_disk().into_iter().filter(|g| !read.contains(g.as_str())).collect();
    assert!(
        orphans.is_empty(),
        "the compile sweep runs nothing that reads {orphans:?} — a golden with no \
         reader in the sweep is a vein that moves and says so only in CI"
    );
}

/// The gates read `*.ll` and the linked binaries out of the working directory,
/// and not one of them produces those files — so the sweep has to, before it
/// runs any of them.
///
/// Getting this wrong is quiet. On 2026-09-06 the sweep read artifacts three
/// minutes stale and reported `machine_code` and `emitted_code` MOVED on a tree
/// identical to HEAD. That direction wastes a round. The same staleness the
/// other way — artifacts older than the source — is a sweep that says nothing
/// moved after an edit that moved a vein, and a session trusting it pushes and
/// finds out from CI.
#[test]
fn the_sweep_builds_the_artifacts_before_it_reads_them() {
    let built = SWEEP.find("scripts/gates/build_benchmarks.sh").expect(
        "the sweep runs scripts/gates/build_benchmarks.sh — the gates read \
             *.ll and the linked binaries out of the working directory and none \
             of them produces those files",
    );
    let read =
        SWEEP.find("\"scripts/gates/$g.sh\"").expect("the sweep runs each named gate by path");
    assert!(
        built < read,
        "the sweep runs a gate at byte {read} before building at byte {built} — \
         a gate reaching a stale artifact reports the last build's veins, and \
         the silent direction of that is a sweep that says nothing moved"
    );
}
