//! `kanso play <file> --plan` prints what a play file would do without doing
//! it, the way `run --plan` does for an entry. A program small enough for one
//! file is how the book and the playground teach, and three of the book's
//! panels are about what a plan shows (#2204).

use std::process::Command;

#[test]
fn a_play_file_shows_its_plan() {
    let fixture = "tests/golden/play_plan/shout.kso";
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["play", fixture, "--plan"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("kanso runs");
    let want = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/play_plan/shout.plan"),
    )
    .expect("the golden reads");
    assert_eq!(String::from_utf8_lossy(&done.stderr), "");
    assert_eq!(String::from_utf8_lossy(&done.stdout), want);
    assert_eq!(done.status.code(), Some(0));
}
