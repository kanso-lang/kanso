//! A benchmark can join the corpus and be weighed at nothing, and three did.
//!
//! `bench/scanbench` ran for weeks with two memory counters in the welfare
//! model and no instruction row anywhere, because the gates each held their
//! own list of benchmarks and one of them was short. The gates now read
//! `bench/benchmarks.txt` through `scripts/gates/benchmarks.sh`, so a
//! benchmark added there is built, measured and sized everywhere at once.
//!
//! What the one list cannot promise is checked here: that each name is a
//! program under bench/, that each golden the gates diff has a row for every
//! name in the list's order, and that the objective weighs only a listed
//! benchmark.

use std::collections::BTreeSet;
use std::path::Path;

fn read(rel: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("{rel} reads: {e}"))
}

/// The benchmarks in `bench/benchmarks.txt`, in order.
fn listed() -> Vec<String> {
    read("bench/benchmarks.txt")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| l.split_whitespace().next())
        .map(str::to_string)
        .collect()
}

/// The first word of each row of a golden, in order.
fn rowed(golden: &str) -> Vec<String> {
    read(golden)
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| l.split_whitespace().next())
        .map(str::to_string)
        .collect()
}

/// Every counter name welfare builds a row for in `worked`, which is where the
/// instruction terms are assembled from the gate's output.
///
/// EVERY occurrence, not the first on each line: with one read per line, two
/// `work["..."]` reads on one line left this spec green naming one benchmark.
fn weighed(welfare: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for piece in welfare.split("work[\"").skip(1) {
        let Some(shut) = piece.find('"') else { continue };
        out.insert(piece[..shut].to_string());
    }
    out
}

/// jsonbench is written by `bench/make_jsonbench`, which build_benchmarks.sh
/// runs first, so a benchmark may be a program under bench/ or the output of a
/// `make_` program beside it.
#[test]
fn every_listed_benchmark_is_a_program_under_bench() {
    let bench = Path::new(env!("CARGO_MANIFEST_DIR")).join("bench");
    let missing: Vec<_> = listed()
        .into_iter()
        .filter(|b| !bench.join(b).is_dir() && !bench.join(format!("make_{b}")).is_dir())
        .collect();
    assert!(
        missing.is_empty(),
        "bench/benchmarks.txt lists these and bench/ has no such program: {missing:?}"
    );
}

/// The gates print their rows in the list's order and diff them against these
/// goldens line by line, so a golden in another order fails every run.
#[test]
fn every_golden_the_gates_diff_rows_the_list_in_order() {
    let all = listed();
    let others: Vec<String> = all.iter().filter(|b| *b != "jsonbench").cloned().collect();
    for (golden, want) in [
        ("bench/instructions_golden.txt", &all),
        ("bench/text_golden.txt", &all),
        // The emitted-code gate counts the decoder against
        // bench/emitted_golden.txt, so jsonbench is the one benchmark this
        // golden lacks.
        ("bench/emitted_golden_others.txt", &others),
    ] {
        assert_eq!(
            &rowed(golden),
            want,
            "{golden} does not row bench/benchmarks.txt in its order. A benchmark \
             added to the list needs its rows here, harvested from CI."
        );
    }
}

/// The 2026-09-06 gavel put the objective's run side on one consolidated
/// program, so the other benchmarks have rows nothing in welfare reads, on
/// purpose: they say where a move came from. What is left to protect is that
/// the one it weighs is built: a term naming a benchmark nothing builds would
/// read the same number forever while looking like a measurement.
#[test]
fn the_benchmark_the_objective_weighs_is_listed() {
    let weighed = weighed(&read("scripts/welfare/welfare.kso"));
    assert_eq!(
        weighed.len(),
        1,
        "the 2026-09-06 gavel put the objective's run side on one consolidated \
         program. welfare reads {weighed:?}. If that is a deliberate change to \
         what the project measures, it is a gavel and this number moves with it."
    );
    let listed: BTreeSet<String> = listed().into_iter().collect();
    let missing: Vec<_> = weighed.difference(&listed).collect();
    assert!(
        missing.is_empty(),
        "the objective weighs these and bench/benchmarks.txt does not list them, so \
         no gate builds or measures them: {missing:?}"
    );
}
