//! A benchmark that joins the work vein must have a direction in the trend
//! gate, or the gate cannot see it get worse.
//!
//! digestbench landed in the vein on 2026-08-31 and in the gate's direction
//! table on 2026-09-01. In between, `work_digestbench 152,573,619 ->
//! 999,999,999` printed as UNCLASSIFIED drift and the gate exited green: the
//! pure-regression rule counts only classified moves, so a 6.5x regression
//! counted toward neither side of it.
//!
//! Every row of `bench/instructions_golden.txt` is one benchmark's retired
//! instructions, and fewer is better in all of them, so the vein's entry in
//! the gate's `bench_goldens` carries `"lower"` for every row it holds, and a
//! benchmark added to `bench/benchmarks.txt` needs no entry of its own. This
//! asserts the entry still says so.

#[test]
fn the_work_vein_carries_its_direction_into_the_trend_gate() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let gate = std::fs::read_to_string(root.join("scripts/trend_gate/trend_gate.kso"))
        .expect("the trend gate reads");
    let entry = gate
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("[\"bench/instructions_golden.txt\""))
        .expect("the trend gate walks bench/instructions_golden.txt");
    let fields: Vec<&str> = entry.split('"').skip(1).step_by(2).collect();
    assert_eq!(
        fields,
        ["bench/instructions_golden.txt", "work_", "lower"],
        "the work vein's entry in `bench_goldens` names no direction, so a rise \
         in any benchmark reads as UNCLASSIFIED drift and the trend gate exits green"
    );
}
