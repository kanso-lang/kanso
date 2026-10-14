use std::path::PathBuf;
use std::process::Command;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/test_verb")
}

#[test]
fn test_runs_a_lone_library_files_tests() {
    let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("test")
        .arg("greet_test.kso")
        .current_dir(fixture_dir())
        .output()
        .expect("kanso binary runs");

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "test_greet ... ok\n1 passed, 0 failed\n");
}

/// A test that recurses past the depth limit fails with the interpreter's
/// diagnostic, and the runner goes on to report the rest. On the main
/// thread's stack Rust overflowed first and aborted the process.
#[test]
fn a_test_that_runs_out_of_stack_fails_alone() {
    let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("test")
        .arg("deep_test.kso")
        .current_dir(fixture_dir())
        .output()
        .expect("kanso binary runs");

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "test_shallow ... ok\ntest_deep ... FAILED\n1 passed, 1 failed\n"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("the program ran out of stack"), "stderr: {stderr}");
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
}

fn in_fixtures(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(args)
        .current_dir(fixture_dir())
        .output()
        .expect("kanso binary runs")
}

/// A test file is an outside consumer of the module beside it (the 2026-10-12
/// gavel, #1883): it imports the module and sees its pub names only. Watched
/// red on the shared-namespace runner, which passed both tests.
#[test]
fn a_test_file_reading_a_private_name_is_refused() {
    let output = in_fixtures(&["test", "peeks"]);
    let expected = std::fs::read_to_string(fixture_dir().join("peeks.stderr")).expect("golden");

    assert_eq!(String::from_utf8_lossy(&output.stderr), expected);
    assert_eq!(output.status.code(), Some(2));
}

/// The test's helper and the module's private function share the name
/// `label`, and each file keeps its own. Watched red as an overlapping
/// overload of `label`.
#[test]
fn a_test_helper_sharing_a_private_name_does_not_collide() {
    let output = in_fixtures(&["test", "shared_name"]);
    let expected = std::fs::read_to_string(fixture_dir().join("shared_name.out")).expect("golden");

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        expected,
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.code(), Some(0));
}

/// `kanso test .` from inside the module tests the same module as naming it
/// from the directory above. The test file reaches its module as `../<name>`,
/// and stepping up from `.` must leave the module's directory rather than
/// look for `./<name>` inside it. No file in the fixture shares the
/// directory's name, so nothing inside can answer by accident. Watched red as
/// "cannot resolve import \"../tested_as_dot\"", the error kanso-json's CI
/// hit running its tests this way.
#[test]
fn a_module_tested_from_inside_as_dot_imports_itself() {
    let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["test", "."])
        .current_dir(fixture_dir().join("tested_as_dot"))
        .output()
        .expect("kanso binary runs");
    let expected =
        std::fs::read_to_string(fixture_dir().join("tested_as_dot.out")).expect("golden");

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        expected,
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.code(), Some(0));
}

/// Only `kanso test` reads a test file, so one that does not parse leaves the
/// program beside it running on both engines. Watched red as a syntax error
/// from `run`.
#[test]
fn a_malformed_test_file_does_not_stop_the_program_beside_it() {
    for engine in [&["run", "runs_beside"][..], &["run", "runs_beside", "--interp"][..]] {
        let output = in_fixtures(engine);
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "hello, kanso\n",
            "{engine:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.status.code(), Some(0));
    }
    let tested = in_fixtures(&["test", "runs_beside"]);
    assert!(
        String::from_utf8_lossy(&tested.stderr)
            .contains("runs_beside/runs_beside/greet_test.kso:3"),
        "{}",
        String::from_utf8_lossy(&tested.stderr)
    );
    assert_eq!(tested.status.code(), Some(2));
}
