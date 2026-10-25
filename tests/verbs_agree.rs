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
    String::from_utf8_lossy(&output.stdout).into_owned() + &String::from_utf8_lossy(&output.stderr)
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

const FIELD_LIBRARY: &str = r#"import "std/list"
import "std/text"

type slot
  path

fn path_key path
  text/join path "."

fn made target
  slot (text/slice target 1 (length target - 1))

pub fn spelled targets
  slots = list/map targets made
  list/to_list (list/map slots (s -> path_key s.path))
"#;

const FIELD_ENTRY: &str = r#"import "./lib"

print "{lib/spelled [["a" "b" "c"]]}"
"#;

fn kanso_in(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("kanso binary runs")
}

/// A field read hands a slice that can be a none to a function with no none
/// arm. The module route checks before field reads become getter calls and
/// the entry route after, and the none question answered differently on
/// each. Watched red: `check lib` said ok and `check main.kso` refused.
#[test]
fn check_gives_a_library_and_its_entry_one_verdict_on_a_field_read() {
    let dir = std::env::temp_dir().join("kanso-verbs-agree-field");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("lib")).expect("a directory to work in");
    std::fs::write(dir.join("lib/lib.kso"), FIELD_LIBRARY).expect("the library writes");
    std::fs::write(dir.join("main.kso"), FIELD_ENTRY).expect("the entry writes");

    let library = kanso_in(&dir, &["check", "lib"]);
    let entry = kanso_in(&dir, &["check", "main.kso"]);
    let run = kanso_in(&dir, &["run", "main.kso"]);

    assert_eq!(String::from_utf8_lossy(&library.stdout), "lib: ok\n");
    assert_eq!(
        String::from_utf8_lossy(&entry.stdout),
        "main.kso: ok\n",
        "{}",
        String::from_utf8_lossy(&entry.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "[\"a.b\"]\n");
}
