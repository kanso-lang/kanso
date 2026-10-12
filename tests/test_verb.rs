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
