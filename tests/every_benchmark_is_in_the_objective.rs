//! A benchmark can join the corpus and be weighed at nothing, and three did.
//!
//! `bench/scanbench` ran for weeks with two memory counters in the welfare
//! model and no instruction row anywhere: `scripts/gates/instructions.sh`
//! swept ten of the eleven benchmarks and nobody noticed which one was
//! missing. escapebench and indexbench had rows in the golden that
//! `scripts/welfare/welfare.kso` never read. A dimension a model leaves out it
//! weights at zero, so for those three a change that spent ten times the
//! instructions to save an allocation scored as a pure gain.
//!
//! Adding the fourth benchmark to a list of ten is the kind of edit that gets
//! forgotten, which is why this is a spec and not a convention. It reads the
//! three files rather than holding a list of its own: a list here would be the
//! twelfth place to forget.

use std::collections::BTreeSet;
use std::path::Path;

/// Every benchmark `build_benchmarks.sh` builds, by the directory it names.
fn built(script: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in script.lines() {
        let Some(rest) = line.split_once(" build bench/").map(|(_, r)| r) else { continue };
        let name = rest.split_whitespace().next().unwrap_or("");
        if !name.is_empty() {
            out.insert(name.to_string());
        }
    }
    out
}

/// Every benchmark the instructions gate actually measures. The loop spans two
/// lines with a backslash, so the words between `for b in` and `; do` are read
/// with the continuations folded out.
fn measured(gate: &str) -> BTreeSet<String> {
    let folded = gate.replace("\\\n", " ");
    let Some(open) = folded.find("for b in ") else { panic!("the gate no longer loops over `b`") };
    let tail = &folded[open + "for b in ".len()..];
    let Some(shut) = tail.find("; do") else {
        panic!("the gate's loop no longer ends with `; do`")
    };
    tail[..shut].split_whitespace().map(str::to_string).collect()
}

/// Every benchmark with a row in the instructions golden.
fn rowed(golden: &str) -> BTreeSet<String> {
    golden
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| l.split_whitespace().next())
        .map(str::to_string)
        .collect()
}

/// Every counter name welfare builds a row for in `worked`, which is where the
/// instruction terms are assembled from the gate's output.
fn weighed(welfare: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in welfare.lines() {
        let Some(open) = line.find("work[\"") else { continue };
        let rest = &line[open + "work[\"".len()..];
        let Some(shut) = rest.find('"') else { continue };
        out.insert(rest[..shut].to_string());
    }
    out
}

fn read(rel: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("{rel} reads: {e}"))
}

#[test]
fn every_built_benchmark_has_its_instructions_counted() {
    let built = built(&read("scripts/gates/build_benchmarks.sh"));
    let measured = measured(&read("scripts/gates/instructions.sh"));
    let missing: Vec<_> = built.difference(&measured).collect();
    assert!(
        missing.is_empty(),
        "these benchmarks are built and their instructions are never counted, so \
         every change to them scores zero on the one dimension no other vein sees: \
         {missing:?}. Add them to the loop in scripts/gates/instructions.sh and \
         harvest their rows from CI."
    );
}

#[test]
fn every_measured_benchmark_has_a_row() {
    let measured = measured(&read("scripts/gates/instructions.sh"));
    let rowed = rowed(&read("bench/instructions_golden.txt"));
    let missing: Vec<_> = measured.difference(&rowed).collect();
    assert!(
        missing.is_empty(),
        "the gate measures these and the golden has no row to compare against, so \
         the gate cannot fail on them: {missing:?}"
    );
}

#[test]
fn every_rowed_benchmark_is_weighed_by_the_objective() {
    let rowed = rowed(&read("bench/instructions_golden.txt"));
    let weighed = weighed(&read("scripts/welfare/welfare.kso"));
    let missing: Vec<_> = rowed.difference(&weighed).collect();
    assert!(
        missing.is_empty(),
        "these have an instruction row that the welfare model never reads, so their \
         runtime work is weighted at zero and a change that doubles it scores as \
         neutral: {missing:?}. Give each a counter in `worked` and a term in a \
         run-speed group."
    );
}

/// The machine-code and emitted-code gates loop the same way the instructions
/// gate does, so `measured` reads all three. Each list is derived from its
/// gate rather than written down here, for the reason the module comment
/// gives: a list here would be the next place to forget.
///
/// The machine-code gate's own comment records the last time one went stale —
/// "scanbench and digestbench joined the corpus after this gate was written
/// and nobody extended the list, so the two newest benchmarks were the two
/// this vein could not see." livebench joined on 2026-09-05 and was the third,
/// on both gates at once.
#[test]
fn every_built_benchmark_has_its_machine_code_sized() {
    let built = built(&read("scripts/gates/build_benchmarks.sh"));
    let sized = measured(&read("scripts/gates/machine_code.sh"));
    let missing: Vec<_> = built.difference(&sized).collect();
    assert!(
        missing.is_empty(),
        "these benchmarks are built and nothing measures how big their machine code \
         is: {missing:?}. That is the dimension the allocation counters and the \
         emitted-line count are both blind to. Add them to the loop in \
         scripts/gates/machine_code.sh and to bench/text_golden.txt."
    );
}

#[test]
fn every_sized_benchmark_has_a_text_row() {
    let sized = measured(&read("scripts/gates/machine_code.sh"));
    let rowed = rowed(&read("bench/text_golden.txt"));
    let missing: Vec<_> = sized.difference(&rowed).collect();
    assert!(
        missing.is_empty(),
        "the gate sizes these and bench/text_golden.txt has no row to compare \
         against, so the gate cannot fail on them: {missing:?}"
    );
}

/// The emitted-code gate counts the decoder separately, against
/// bench/emitted_golden.txt, so jsonbench is the one built benchmark its
/// `others` loop is allowed to lack.
#[test]
fn every_built_benchmark_has_its_emitted_code_counted() {
    let mut built = built(&read("scripts/gates/build_benchmarks.sh"));
    built.remove("jsonbench");
    let emitted = measured(&read("scripts/gates/emitted_code.sh"));
    let missing: Vec<_> = built.difference(&emitted).collect();
    assert!(
        missing.is_empty(),
        "these benchmarks are built and nothing counts what the compiler WROTE for \
         them: {missing:?}. The decoder gained 20% more calls over a fortnight with \
         every allocation counter byte-identical, which is what this vein is for. \
         Add them to the `others` loop in scripts/gates/emitted_code.sh and to \
         bench/emitted_golden_others.txt."
    );
}

#[test]
fn every_emitted_benchmark_has_a_row() {
    let emitted = measured(&read("scripts/gates/emitted_code.sh"));
    let rowed = rowed(&read("bench/emitted_golden_others.txt"));
    let missing: Vec<_> = emitted.difference(&rowed).collect();
    assert!(
        missing.is_empty(),
        "the gate counts these and bench/emitted_golden_others.txt has no row to \
         compare against, so the gate cannot fail on them: {missing:?}"
    );
}
