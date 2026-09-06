//! The compile-side sweep names every compile-side gate, and cannot quietly
//! miss one.
//!
//! `scripts/gates/all_counters.sh` walks the eleven RUNTIME cost goldens and
//! nothing else. The compile side is separate gates, two of whose counters are
//! welfare terms, and a change under `lib/` moves every one of them because
//! `lib/*.kso` is `include_str!`'d into the compiler. On 2026-09-05 a
//! twelve-line library change read as a welfare RISE with those veins stale and
//! a FALL once they were regenerated, and nothing in the sweep beside them
//! would have said so.
//!
//! `all_compile.sh` closes that. A sweep that names all but one looks like
//! coverage and is not, which is the lesson
//! `every_counter_gate_is_in_the_sweep.rs` already carries for the other half,
//! so the list is pinned to a property of the tree rather than to anyone's
//! memory: a gate whose script reads a compile-side golden is in the sweep.

use std::collections::BTreeSet;
use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

const SWEEP: &str = include_str!("../scripts/gates/all_compile.sh");

/// The names on the `gates="..."` line.
fn named() -> BTreeSet<String> {
    SWEEP
        .split_once("gates=\"")
        .expect("the sweep declares a gates list")
        .1
        .split_once('"')
        .expect("the gates list is closed")
        .0
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

/// Every gate under scripts/gates whose script reads a compile-side golden.
///
/// One is excluded for a reason that is not "it was inconvenient":
/// `build_benchmarks` is not a gate and says so in its own first line.
/// `all_compile` matches its own list and is not a gate; `all_counters` reads
/// the runtime side. `compile_ir_row` was a fourth exclusion until 2026-09-05,
/// when the per-chip table it read was retired under the one-row-one-value
/// ruling and the comparison moved back inside `compile_instructions`.
fn compile_gates_on_disk() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for entry in std::fs::read_dir(root().join("scripts/gates")).expect("the gates directory reads")
    {
        let path = entry.expect("a directory entry reads").path();
        let Some(name) = path.file_stem().and_then(|n| n.to_str()) else { continue };
        if path.extension().and_then(|e| e.to_str()) != Some("sh") {
            continue;
        }
        if matches!(name, "all_compile" | "all_counters" | "build_benchmarks") {
            continue;
        }
        let body = std::fs::read_to_string(&path).expect("a gate reads");
        let reads_compile_golden = body.lines().any(|l| {
            (l.contains("bench/compile_") && l.contains("golden"))
                || l.contains("bench/text_golden")
                || l.contains("bench/emitted_golden")
        });
        if reads_compile_golden {
            out.insert(name.to_string());
        }
    }
    out
}

#[test]
fn the_sweep_names_every_gate_that_reads_a_compile_golden() {
    let named = named();
    let on_disk = compile_gates_on_disk();
    assert!(!on_disk.is_empty(), "there is at least one compile gate to sweep");
    let missing: Vec<_> = on_disk.difference(&named).collect();
    assert!(
        missing.is_empty(),
        "scripts/gates/all_compile.sh has no entry for {missing:?} — a sweep that \
         names all but one looks like coverage and is not"
    );
    let extra: Vec<_> = named.difference(&on_disk).collect();
    assert!(extra.is_empty(), "the sweep names {extra:?}, which reads no compile golden");
}

/// And every name on the list is a gate that exists and can be run. The first
/// draft of the runtime sweep's twin asserted a directory that a fresh clone
/// does not carry; a name is checked against the tree rather than assumed.
#[test]
fn every_named_gate_is_a_script_on_disk() {
    for gate in named() {
        let path = root().join(format!("scripts/gates/{gate}.sh"));
        assert!(path.exists(), "the sweep names {gate}, which is not in scripts/gates");
    }
}

/// And CLAUDE.md's own list of them, which is the one a session actually reads
/// before touching lib/. It named five when there were six, in the same
/// paragraph that warns a count in prose goes stale — so it is pinned to the
/// tree rather than trusted, exactly as the cost-golden count is in
/// `every_counter_gate_is_in_the_sweep.rs`.
///
/// Scoped to the bullet, not the file: a name that happens to appear in some
/// distant sentence is not the instruction telling a session to run it.
#[test]
fn the_instructions_name_every_compile_gate() {
    let guidance =
        std::fs::read_to_string(root().join("CLAUDE.md")).expect("the instructions read");
    let bullet = guidance
        .split("\n- ")
        .find(|b| b.contains("are separate gates"))
        .expect("the instructions name the compile gates somewhere");
    let missing: Vec<_> = compile_gates_on_disk()
        .into_iter()
        .filter(|g| !bullet.contains(&format!("`{g}`")))
        .collect();
    assert!(
        missing.is_empty(),
        "CLAUDE.md tells a session which compile gates to run and does not name \
         {missing:?} — that list was short for as long as it was written down"
    );
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
    let orphans: Vec<_> = compile_goldens_on_disk()
        .into_iter()
        .filter(|g| !read.contains(g.as_str()))
        .collect();
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
    let built = SWEEP
        .find("scripts/gates/build_benchmarks.sh")
        .expect(
            "the sweep runs scripts/gates/build_benchmarks.sh — the gates read \
             *.ll and the linked binaries out of the working directory and none \
             of them produces those files",
        );
    let read = SWEEP
        .find("\"scripts/gates/$g.sh\"")
        .expect("the sweep runs each named gate by path");
    assert!(
        built < read,
        "the sweep runs a gate at byte {read} before building at byte {built} — \
         a gate reaching a stale artifact reports the last build's veins, and \
         the silent direction of that is a sweep that says nothing moved"
    );
}
