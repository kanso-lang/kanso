//! A failure the compiler can work out is not a runtime failure.
//!
//! `kanso check` answers `ok` for `3 / 0`, for `text/to_int "two"`, and for an
//! index past the end of a literal list — then every one of them dies when the
//! program runs. Nothing here is dynamic: the operands are written in the
//! source, and the answer is decidable by reading it.
//!
//! Clay put it plainly while arguing that err should come only from io: a
//! conversion failure on a literal "would fail to compile". It does not.
//!
//! Consequence recorded with the task: docs/book/samples/ch04/teas.kso is built
//! on two of these, and folding them makes the book's own demonstration of the
//! err merge a pair of compile errors. That is the thesis arriving rather than
//! a cost — the two most obvious "fails at runtime" examples turned out to be
//! things a compiler should have caught.

use std::process::Command;

/// `play` rather than `check`: a decidable failure should be refused before
/// anything runs, and `play` is the verb a person reaches for on one file.
///
/// The directory is per-case because cargo runs these in parallel and a shared
/// path lets one case overwrite another's program — which is how the socket
/// test wedged the suite (#152), arrived at twice in one afternoon.
fn checked(case: &str, source: &str) -> String {
    let dir = std::env::temp_dir().join(format!("kanso-decidable-{case}"));
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

/// Watched red: `one.kso: ok`, then `error[endpoint]: … "division by zero"`.
#[test]
fn a_literal_division_by_zero_is_refused() {
    let said = checked("div", "print \"{3 / 0}\"\n");

    assert!(said.contains("division by zero"), "not refused: {said}");
    assert!(!said.contains("endpoint"), "refused at run time, not compile time: {said}");
}

/// Watched red: `one.kso: ok`, then the same at run time.
#[test]
fn a_literal_modulo_by_zero_is_refused() {
    let said = checked("mod", "print \"{3 % 0}\"\n");

    assert!(said.contains("modulo by zero"), "not refused: {said}");
    assert!(!said.contains("endpoint"), "refused at run time, not compile time: {said}");
}

/// Watched red: `one.kso: ok`, then `"two" is not an integer` at run time.
#[test]
fn a_literal_that_is_not_a_number_is_refused() {
    let said = checked("toint", "import \"std/text\"\n\nprint \"{text/to_int \"two\"}\"\n");

    assert!(said.contains("not an integer"), "not refused: {said}");
    assert!(!said.contains("endpoint"), "refused at run time, not compile time: {said}");
}

/// And the ordinary cases stay quiet — a divisor that is not a literal zero
/// cannot be decided by reading, and must not be refused.
#[test]
fn a_divisor_the_compiler_cannot_read_is_left_alone() {
    let said = checked("sound", "fn half n\n  n / 2\n\nprint \"{half 8}\"\n");

    assert_eq!(said, "4\n", "a sound program was refused: {said}");
}
