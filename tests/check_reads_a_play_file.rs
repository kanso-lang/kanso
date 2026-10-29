//! `kanso check` on a play file reports what `kanso play` would refuse
//! (#2205). A play file holds declarations beside statements, and `check` used
//! to send any file with a declaration to the library compile, which refused
//! the first statement and checked nothing else. The book's `_check` samples
//! are play files, so they had no check at all.

use std::process::Command;

fn check(fixture: &str) -> (String, String, Option<i32>) {
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["check", &format!("tests/golden/play_check/{fixture}")])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("kanso runs");
    (
        String::from_utf8_lossy(&done.stdout).into_owned(),
        String::from_utf8_lossy(&done.stderr).into_owned(),
        done.status.code(),
    )
}

#[test]
fn a_play_file_that_would_run_checks_clean() {
    let (out, err, code) = check("fine.kso");
    assert_eq!(err, "");
    assert_eq!(out, "tests/golden/play_check/fine.kso: ok\n");
    assert_eq!(code, Some(0));
}

#[test]
fn a_play_file_is_refused_for_what_play_would_refuse() {
    let (out, err, code) = check("unknown.kso");
    let want = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/golden/play_check/unknown.stderr"),
    )
    .expect("the golden reads");
    assert_eq!(err, want);
    assert_eq!(out, "");
    assert_eq!(code, Some(2));
}
