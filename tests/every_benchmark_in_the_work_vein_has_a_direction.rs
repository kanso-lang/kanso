//! A benchmark that joins the work vein without joining the trend gate's
//! direction table is invisible to the gate that watches the work vein.
//!
//! digestbench landed in the vein on 2026-08-31 and in this table on
//! 2026-09-01. In between, `work_digestbench 152,573,619 -> 999,999,999`
//! printed as UNCLASSIFIED drift and the gate exited green: the pure-regression
//! rule counts only classified moves, so a 6.5x regression counted toward
//! neither side of it.
//!
//! The other veins hold counters left unclassified on purpose — a fast-path
//! presence counter and its slow-path twin move together and neither direction
//! means anything alone. `bench/instructions_golden.txt` is not like that.
//! Every row is one benchmark's retired instructions, and fewer is better in
//! all of them, so "every row has a direction" is an invariant here and this
//! asserts it rather than the eleven names that satisfy it today.

/// The counter names the trend gate's `lower_*` bindings list. Read out of the
/// gate's source because the gate is written in kanso and there is no other way
/// in; a restructuring of those bindings breaks this loudly, which is the
/// intended failure.
fn classified_lower(gate: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in gate.lines() {
        let Some(rest) = line.strip_prefix("lower_") else { continue };
        let Some(open) = rest.find('[') else { continue };
        let Some(shut) = rest.find(']') else { continue };
        for word in rest[open + 1..shut].split('"') {
            let word = word.trim();
            if !word.is_empty() {
                out.push(word.to_string());
            }
        }
    }
    out
}

#[test]
fn every_row_of_the_work_vein_is_named_by_a_direction_table() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let vein = std::fs::read_to_string(root.join("bench/instructions_golden.txt"))
        .expect("the work vein reads");
    let gate = std::fs::read_to_string(root.join("scripts/trend_gate/trend_gate.kso"))
        .expect("the trend gate reads");
    let named = classified_lower(&gate);
    assert!(
        named.len() > 20,
        "the lower_* bindings parsed to {} names, which means the shape moved \
         and this spec is reading nothing",
        named.len()
    );

    let mut adrift = Vec::new();
    let mut rows = 0;
    for line in vein.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let name = line.split_whitespace().next().expect("a row names a benchmark");
        rows += 1;
        if !named.iter().any(|n| n == name) {
            adrift.push(name.to_string());
        }
    }
    assert!(rows >= 10, "the work vein parsed to {rows} rows, so this spec is reading nothing");
    assert!(
        adrift.is_empty(),
        "these benchmarks are in bench/instructions_golden.txt and in no direction \
         table, so a rise in any of them reads as UNCLASSIFIED drift and the trend \
         gate exits green: {adrift:?}"
    );
}
