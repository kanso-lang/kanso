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
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "test_greet ... ok\n1 passed, 0 failed\n"
    );
}
