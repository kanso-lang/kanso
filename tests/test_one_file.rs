//! Testing one file of a module tests it inside its module.
//!
//! A module's files share their declarations, so a test file naming a function
//! its sibling declares is the ordinary case — `lib/list/list_test.kso` calls
//! `find` and `sorted`, which live in `lib/list/list.kso`. Compiling the file
//! alone answers `unknown name find`, which reads as a broken test rather than
//! as a verb that took the wrong target.

use std::process::Command;

fn tested(target: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["test", target])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("kanso binary runs")
}

/// Watched red: `unknown name find`, `unknown name sorted`, exit 2.
#[test]
fn one_file_of_a_module_sees_its_siblings() {
    let output = tested("lib/list/list_test.kso");

    assert!(
        output.status.success(),
        "testing one file of a module failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("0 failed"),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

/// And it runs that file's tests rather than the whole module's — the
/// directory is how you ask for all of them.
#[test]
fn a_file_runs_its_own_tests_and_the_directory_runs_them_all() {
    let one = tested("lib/json/json_test.kso");
    let all = tested("lib/json");

    let count = |out: &std::process::Output| {
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|line| line.ends_with(" ... ok"))
            .count()
    };
    assert!(one.status.success(), "{}", String::from_utf8_lossy(&one.stderr));
    assert!(all.status.success(), "{}", String::from_utf8_lossy(&all.stderr));
    assert!(count(&one) > 0, "the file ran no tests");
    assert!(
        count(&one) <= count(&all),
        "one file ran {} tests where the module ran {}",
        count(&one),
        count(&all)
    );
}
