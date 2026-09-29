//! `if` held as a value, `&if` or `&if c`, picks a branch when it is called
//! with all three arguments.
//!
//! The interpreter counts a held function's arguments before it calls it, and
//! it read a builtin's count from the checker's table, where `if` has no entry
//! on purpose: its count is checked where its branches are. So `&if` answered
//! no count at all, three arguments grew the partial instead of calling it, and
//! `"{g true 1 2}"` printed `<fn>`. Native declines the form by name, which is
//! the one way the differential law lets it live on fewer engines, so the
//! oracle's answer is the only one there is and it has to be the branch.

use std::path::PathBuf;
use std::process::Command;

const HELD: &str = "fn pick c
  g = &if
  h = &if c
  \"{g c 1 2} {h 3 4}\"

print \"{pick true} {pick false}\"
";

fn written() -> PathBuf {
    let dir = std::env::temp_dir().join("kanso-held-if-test");
    std::fs::create_dir_all(&dir).expect("temp work dir");
    let file = dir.join("held_if.kso");
    std::fs::write(&file, HELD).expect("program writes");
    file
}

fn run(extra: &[&str]) -> std::process::Output {
    let path = written();
    let mut args = vec!["play", path.to_str().expect("utf-8")];
    args.extend_from_slice(extra);
    Command::new(env!("CARGO_BIN_EXE_kanso")).args(args).output().expect("kanso runs")
}

#[test]
fn a_held_if_answers_the_branch_it_picks() {
    let out = run(&["--interp"]);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "1 3 2 4",
        "the oracle did not call the held if: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn native_names_its_limit_for_a_held_if() {
    let said = String::from_utf8_lossy(&run(&[]).stderr).into_owned();
    assert!(
        said.contains("`if` as a bare value is not yet supported"),
        "the backend did not name its own limit: {said}"
    );
}
