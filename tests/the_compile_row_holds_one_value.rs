//! One row, one value. Ruled 2026-09-05.
//!
//! The front end's instruction count spent a fortnight being keyed. First by
//! silicon — `bench/compile_instructions_by_cpu.txt`, one row per chip, because
//! glibc resolves memcpy and its neighbours by ifunc and the runner pool is at
//! least four CPUs. Then a row was allowed to pin TWO values, because one
//! binary on one chip had been seen to count 41,831,767 and 41,832,275 and
//! neither named suspect explained the 508.
//!
//! Both are retired. The evidence that retired them is eight within-binary
//! sittings across two binaries, two vendors and four CPU generations, agreeing
//! to the instruction and none disagreeing; the 508 was two binaries read as
//! one, and the earlier drift was Rust's stack guard parsing /proc/self/maps at
//! startup, which the gate now anchors past by counting `kanso::main` inclusive.
//! Consistency is verified by reproduction: the same build on any runner counts
//! the same number.
//!
//! What that leaves is a rule about a file, and the rule is what this pins. A
//! run that disagrees is a REPRODUCTION FAILURE — it halts the vein and is
//! hunted to its source. The tempting repair, both times it was reached for
//! before, was to write the second number down beside the first and move on,
//! which is how a row stops being able to fail. A band is the same move with
//! arithmetic: wide enough to hold 508 is wide enough to hold kanso#1226's
//! -5,621, which was a real change to the compiler.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

const GOLDEN: &str = "bench/compile_instructions_golden.txt";

/// The golden's value lines, comments and blanks dropped.
fn value_lines(body: &str) -> Vec<&str> {
    body.lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

#[test]
fn the_golden_pins_exactly_one_value() {
    let body = std::fs::read_to_string(root().join(GOLDEN)).expect("the compile golden reads");
    let lines = value_lines(&body);
    assert_eq!(
        lines.len(),
        1,
        "{GOLDEN} carries {} value lines and the row is one row with one \
         value; the extras are {:?}",
        lines.len(),
        &lines[1.min(lines.len())..]
    );
    let line = lines[0];
    let value = line.strip_prefix("compile_instructions=").unwrap_or_else(|| {
        panic!("{GOLDEN}'s value line reads {line:?}, not compile_instructions=")
    });
    assert!(
        !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()),
        "{GOLDEN} pins {value:?}. One row, one value: a second number beside \
         the first, or a range around it, records a reproduction failure as a \
         mode instead of hunting it, and that is what the 2026-09-05 ruling \
         forbids."
    );
}

#[test]
fn nothing_keys_the_row_by_chip_any_more() {
    let table = root().join("bench/compile_instructions_by_cpu.txt");
    assert!(
        !table.exists(),
        "bench/compile_instructions_by_cpu.txt is back. The row is not keyed \
         by silicon: eight sittings across two vendors and four CPU \
         generations agreed to the instruction, and a per-chip table hides the \
         disagreement it was built to record."
    );
}

#[test]
fn no_gate_reads_a_chip_keyed_table() {
    let gates = root().join("scripts/gates");
    let mut naming = Vec::new();
    for entry in std::fs::read_dir(&gates).expect("the gates directory reads") {
        let path = entry.expect("a directory entry reads").path();
        if path.extension().and_then(|e| e.to_str()) != Some("sh") {
            continue;
        }
        let body = std::fs::read_to_string(&path).expect("a gate reads");
        if body.contains("compile_instructions_by_cpu") {
            naming.push(path.file_name().and_then(|n| n.to_str()).unwrap().to_string());
        }
    }
    assert!(
        naming.is_empty(),
        "these gates still read a chip-keyed compile row: {naming:?}. The \
         table is retired and the gate compares against {GOLDEN} exactly."
    );
}
