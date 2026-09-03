//! The front end's instruction count is keyed by silicon, and every way of
//! getting that wrong has to be a refusal.
//!
//! The vein it guards has a history of being confidently wrong. Four readings
//! were written up in one day as layout effects — +6,087, -416, +8,032,
//! -3,289 — and then a pull request that changed two documentation files and
//! nothing the compiler reads moved the same row 5,081. The pool is at least
//! four CPUs, glibc resolves memcpy and its neighbours by ifunc at load time,
//! and the compiler runs that code. So a single number for the row is a number
//! about the runner.
//!
//! A band would have been the cheap fix and it is the wrong one: held on one
//! chip with the binary's sha printed on both reads, kanso#1226's runtime
//! change moved this row -5,621, which is the same size as the chip effect. A
//! tolerance wide enough to swallow the noise swallows the signal.
//!
//! So the noise gets a key, and the four failures below are the whole of what
//! keying it means. The one that matters most is the third: an unrecorded chip
//! REFUSES. Letting it pass is exactly the design CI killed in two runs, where
//! three runs in four landed on an unrecorded cpu and would have waved a
//! regression through — and the ratchet's mutations redden these same gates,
//! so on those runs its rows would have gone blind.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A table's rows, wrapped in the header shape the real file has. The comment
/// lines are here because the script strips them, and a stripper that took the
/// first comment for a row would read `#` as a chip.
fn table_of(rows: &[(&str, &str)]) -> String {
    let mut held = String::from("# a table\n#\n# measured-on glibc=0.0 rustc=0.0.0\n#\n\n");
    for (key, value) in rows {
        held.push_str(&format!("{key} {value}\n"));
    }
    held
}

/// A golden carrying one bare `compile_instructions`, the way welfare, the
/// trend gate and golden_prose read it.
fn golden_of(value: &str) -> String {
    format!("# a golden\ncompile_instructions={value}\nfront_end_rounds=40\n")
}

struct Answer {
    code: i32,
    said: String,
}

impl Answer {
    fn refused(&self) -> bool {
        self.code == 1
    }
}

fn asked(name: &str, table: &str, golden: &str, key: &str, got: &str) -> Answer {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let stage: PathBuf = std::env::temp_dir().join(format!("kanso-ir-row-{name}"));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).expect("a staging directory");
    let table_at = stage.join("by_cpu.txt");
    let golden_at = stage.join("golden.txt");
    std::fs::write(&table_at, table).expect("the table writes");
    std::fs::write(&golden_at, golden).expect("the golden writes");

    let out = Command::new("sh")
        .arg(root.join("scripts/gates/compile_ir_row.sh"))
        .arg(&table_at)
        .arg(&golden_at)
        .arg(key)
        .arg(got)
        .output()
        .expect("the script runs");
    let mut said = String::from_utf8_lossy(&out.stdout).into_owned();
    said.push_str(&String::from_utf8_lossy(&out.stderr));
    Answer { code: out.status.code().expect("the script exits"), said }
}

/// The chip that counted the row counted this run too, and the golden agrees
/// with the reference row. Nothing to say.
#[test]
fn a_row_this_chip_landed_on_passes() {
    let answer = asked(
        "match",
        &table_of(&[("family0x6-model0x55", "41495096")]),
        &golden_of("41495096"),
        "family0x6-model0x55",
        "41495096",
    );
    assert_eq!(answer.code, 0, "an exact row on the right chip passes: {}", answer.said);
    assert!(
        answer.said.contains("41495096"),
        "and says what it matched, so a job log carries the value: {}",
        answer.said
    );
}

/// A later row is checked the same way. The first row is the reference series
/// only because welfare needs one number; it carries no authority over what a
/// second chip should read.
#[test]
fn a_chip_that_is_not_the_reference_is_read_against_its_own_row() {
    let table =
        table_of(&[("family0x6-model0x55", "41495096"), ("family0x19-model0x1", "41500974")]);
    let landed =
        asked("second-ok", &table, &golden_of("41495096"), "family0x19-model0x1", "41500974");
    assert_eq!(landed.code, 0, "the second chip's own row passes: {}", landed.said);

    let crossed =
        asked("second-crossed", &table, &golden_of("41495096"), "family0x19-model0x1", "41495096");
    assert!(
        crossed.refused(),
        "and the FIRST chip's value on the second chip does not: {}",
        crossed.said
    );
}

/// An empty table cannot answer, and says so with the line to paste. This is
/// the state the vein starts in, because only CI may write these numbers —
/// they belong to its toolchain — and it writes one chip per run.
#[test]
fn a_table_with_no_rows_refuses_and_says_what_to_add() {
    let answer =
        asked("empty", &table_of(&[]), &golden_of("41495096"), "family0x6-model0x55", "41502222");
    assert!(answer.refused(), "an empty table refuses: {}", answer.said);
    assert!(
        answer.said.contains("family0x6-model0x55 41502222"),
        "and prints the row to add, so nobody has to guess the key's shape: {}",
        answer.said
    );
}

/// The chip is not in the table. This is the answer the whole design turns on:
/// it is a refusal, not a skip.
#[test]
fn an_unrecorded_chip_refuses_rather_than_passing() {
    let answer = asked(
        "unknown",
        &table_of(&[("family0x6-model0x55", "41495096")]),
        &golden_of("41495096"),
        "family0x19-model0x1",
        "41500974",
    );
    assert!(
        answer.refused(),
        "an unrecorded chip is an unsat row and does not pass: {}",
        answer.said
    );
    assert!(
        answer.said.contains("family0x19-model0x1 41500974"),
        "and it prints the sitting to add: {}",
        answer.said
    );
}

/// The golden's bare line and the reference row are the same quantity, so they
/// have to hold the same number. Nothing else in the tree can see this: welfare
/// reads only the golden and this gate reads only the table, so a re-sitting
/// that updated one and not the other would leave the objective tracking a
/// number no chip ever counted, silently.
#[test]
fn a_golden_that_drifted_from_the_reference_row_refuses() {
    let answer = asked(
        "drifted",
        &table_of(&[("family0x6-model0x55", "41495096")]),
        &golden_of("41500974"),
        "family0x6-model0x55",
        "41495096",
    );
    assert!(answer.refused(), "the two files must agree: {}", answer.said);
    assert!(
        answer.said.contains("41500974") && answer.said.contains("41495096"),
        "and the refusal names both numbers: {}",
        answer.said
    );
}

/// The drift check runs first, so it is not hidden by a chip the table has
/// never seen. A branch re-sitting a new chip is exactly when the golden is
/// most likely to be left behind.
#[test]
fn the_drift_is_caught_even_on_a_chip_with_no_row() {
    let answer = asked(
        "drifted-unknown",
        &table_of(&[("family0x6-model0x55", "41495096")]),
        &golden_of("41500974"),
        "family0x19-model0x1",
        "41500974",
    );
    assert!(answer.refused(), "still a refusal: {}", answer.said);
    assert!(
        answer.said.contains("welfare"),
        "and it is the drift that is named, not the missing chip: {}",
        answer.said
    );
}

/// The row moved on the chip that counted it. This is the regression the vein
/// exists for, and the message may not offer the runner as an excuse, because
/// on a keyed row the runner is not available as one.
#[test]
fn a_row_that_moved_on_its_own_chip_is_the_front_end() {
    let answer = asked(
        "moved",
        &table_of(&[("family0x6-model0x55", "41495096")]),
        &golden_of("41495096"),
        "family0x6-model0x55",
        "41489475",
    );
    assert!(answer.refused(), "a moved row refuses: {}", answer.said);
    assert!(answer.said.contains("FRONT END"), "and names the front end: {}", answer.said);
    assert!(
        answer.said.contains("fall is a win to bank"),
        "and says a fall is banked rather than only warning about rises: {}",
        answer.said
    );
}

/// Called wrong, it says so and exits 2 — distinct from a refusal, so a gate
/// that lost an argument cannot read as a passing row or as a regression.
#[test]
fn a_missing_argument_is_neither_a_pass_nor_a_regression() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Command::new("sh")
        .arg(root.join("scripts/gates/compile_ir_row.sh"))
        .arg("/nonexistent/table.txt")
        .output()
        .expect("the script runs");
    assert_eq!(out.status.code(), Some(2), "usage is its own answer");
}
