//! `>>` sequences effects, and says so where the operand is written.
//!
//! `1 >> 2` passed `kanso check` clean and died at run time. Nothing about it
//! is dynamic — both operands are integer literals, and the wall's own rule
//! says it sequences two effect descriptions. A call to a function that can
//! never answer an effect is the same case one step out, and the fixpoint
//! already knows which calls those are.
//!
//! The third example is the one that must not move: a wall between two real
//! effects is the ordinary case, and refusing it would be far worse than the
//! bug.

use std::process::Command;

fn played(case: &str, source: &str) -> String {
    let dir = std::env::temp_dir().join(format!("kanso-wall-{case}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a directory to work in");
    std::fs::write(dir.join("one.kso"), source).expect("the file writes");

    let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["play", "one.kso"])
        .current_dir(&dir)
        .output()
        .expect("kanso binary runs");
    String::from_utf8_lossy(&output.stdout).into_owned() + &String::from_utf8_lossy(&output.stderr)
}

/// Watched red: `one.kso: ok` from check, then the runtime refusal.
#[test]
fn a_wall_between_two_literals_is_refused_where_it_is_written() {
    let said = played("lits", "1 >> 2\n");

    assert!(said.contains("sequences"), "not refused: {said}");
    assert!(!said.contains("error[runtime]"), "refused at run time: {said}");
}

/// The same case one step out, which needs the fixpoint rather than the shape.
#[test]
fn a_wall_between_two_pure_calls_is_refused() {
    let said =
        played("calls", "fn pure_one _\n  1\n\nfn pure_two _\n  2\n\npure_one 0 >> pure_two 0\n");

    assert!(said.contains("sequences"), "not refused: {said}");
    assert!(!said.contains("error[runtime]"), "refused at run time: {said}");
}

/// And the ordinary wall keeps working.
#[test]
fn a_wall_between_two_effects_still_runs() {
    let said = played("effects", "print \"a\" >> print \"b\"\n");

    assert_eq!(said, "a\nb\n", "an ordinary wall was refused: {said}");
}
