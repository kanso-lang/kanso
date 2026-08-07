//! The three verbs describe the same file the same way.
//!
//! A file holding declarations *and* top-level statements is a library with
//! statements it cannot run — `kanso play` is the relaxed form that takes
//! both. `run` said so already. `check` complained that blank lines may not
//! appear inside a body, pointing at a line that is not in a body, and
//! removing the blank moved it to "a top-level line must begin with `fn`" —
//! two diagnostics, neither naming the rule or the verb that does the job.

use std::process::Command;

const DECLARATION_AND_STATEMENT: &str = r#"fn greeting who
  "hello {who}"

print (greeting "world")
"#;

fn verb(verb: &str, dir: &std::path::Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args([verb, "one.kso"])
        .current_dir(dir)
        .output()
        .expect("kanso binary runs");
    String::from_utf8_lossy(&output.stdout).into_owned()
        + &String::from_utf8_lossy(&output.stderr)
}

/// Watched red: check answered `error[formatting]: blank lines may not appear
/// inside a body` for line 4, which is a top-level statement.
#[test]
fn check_names_the_verb_that_runs_a_declaration_beside_a_statement() {
    let dir = std::env::temp_dir().join("kanso-verbs-agree");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a directory to work in");
    std::fs::write(dir.join("one.kso"), DECLARATION_AND_STATEMENT).expect("the file writes");

    let said = verb("check", &dir);

    assert!(said.contains("kanso play"), "check did not name the verb: {said}");
    assert!(!said.contains("blank lines"), "check blamed the blank line: {said}");
}

/// And `play` is that verb, so the advice is followed by something that works.
#[test]
fn play_runs_what_check_points_at() {
    let dir = std::env::temp_dir().join("kanso-verbs-agree-play");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a directory to work in");
    std::fs::write(dir.join("one.kso"), DECLARATION_AND_STATEMENT).expect("the file writes");

    assert_eq!(verb("play", &dir), "hello world\n");
}
