//! Every instruction gate consults the recorded silicon when a row has moved.
//!
//! `scripts/gates/dispatch.sh` has carried a `differs` verb since it was
//! written, `bench/dispatch.txt` was never recorded, and no gate called either.
//! What the gates did call is `dispatch.sh name`, which prints the basic family
//! and model and stops — the two rows most likely to read the same on two
//! different runners.
//!
//! They are not the same. A sweep of ninety-odd cost-goldens job logs on
//! 2026-09-17 found SEVEN distinct feature blocks, identical within any one job
//! and differing across jobs in 57 rows. The basic family itself takes three
//! values, 0x19, 0x1a and 0x6, and the last of those is Intel. Level-3 cache
//! spans 32 MB to 480 MB, and `Fast_Unaligned_Load`, `Prefer_No_AVX512` and
//! `Prefer_PMINUB_for_stringop` flip between them — three of the switches
//! glibc's ifunc resolvers read when they pick `memcpy` and its neighbours.
//!
//! The gates pin the cache-derived thresholds through `GLIBC_TUNABLES`, which
//! is why the rows hold as well as they do. What the tunables do not reach is
//! which implementation the resolver picks.
//!
//! So a row that moved now gets told whether the silicon moved with it. The
//! consult sits in the disagreement path and never decides the gate's exit: a
//! resolver difference is a candidate explanation, not a verdict.

const GATES: [&str; 5] = [
    "interp_instructions",
    "compile_instructions",
    "entry_instructions",
    "library_instructions",
    "startup_instructions",
];

fn gate(name: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(root.join("scripts/gates").join(format!("{name}.sh")))
        .unwrap_or_else(|e| panic!("scripts/gates/{name}.sh reads: {e}"))
}

/// The consult exists, and it is in the path a moved row takes rather than
/// somewhere the gate reaches on every run.
#[test]
fn every_instruction_gate_consults_the_block_when_a_row_moved() {
    let mut missing = Vec::new();
    for name in GATES {
        let text = gate(name);
        let consults = text.contains("dispatch.sh differs");
        let anchor = format!("::error::{name} counted ");
        let at_move = match (text.find("dispatch.sh differs"), text.find(&anchor)) {
            (Some(d), Some(a)) => d < a,
            _ => false,
        };
        if !consults || !at_move {
            missing.push(format!(
                "  {name}.sh: consults={consults} before-the-verdict={at_move}"
            ));
        }
    }
    assert!(
        missing.is_empty(),
        "a gate reports a moved row without saying what the silicon did:\n{}",
        missing.join("\n"),
    );
}

/// And the block it consults is on disk with rows in it. `differs` answers
/// "cannot tell" when the file is absent, which is what it answered for as
/// long as the file was absent.
#[test]
fn the_block_is_recorded_and_carries_its_rows() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(root.join("bench/dispatch.txt"))
        .expect("bench/dispatch.txt reads");
    let rows: Vec<&str> = text.lines().filter(|l| l.starts_with("x86.")).collect();
    assert!(
        rows.len() > 100,
        "bench/dispatch.txt carries {} feature rows, which is not a block",
        rows.len(),
    );
    let mut sorted = rows.clone();
    sorted.sort_unstable();
    assert_eq!(rows, sorted, "the block is not sorted, so a diff of it will not read line by line");
    for row in ["x86.cpu_features.basic.family", "x86.cpu_features.preferred.Fast_Unaligned_Load"] {
        assert!(
            rows.iter().any(|l| l.starts_with(&format!("{row}="))),
            "the block does not record {row}, which is one of the rows that moves",
        );
    }
}
