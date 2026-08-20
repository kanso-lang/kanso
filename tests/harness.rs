use std::process::Command;

/// A library that cannot assert its own failures cannot be trusted, and the
/// testing hako is how it does: every tested package's err is foreign to the
/// hako, so its arms are licensed by the rules as they stand. The module
/// proves both halves — a test file reading a failure, and a production file
/// reading its own through the same door.
#[test]
fn a_package_can_assert_its_own_failures() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/harness/pkg");
    let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["test", "."])
        .current_dir(dir)
        .output()
        .expect("kanso runs");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("3 passed, 0 failed"),
        "{stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success());
}

/// And `check` accepts the same module, because the harness surface is
/// ordinary library code — no verb carries a vocabulary another lacks.
#[test]
fn check_accepts_a_module_whose_tests_read_failures() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/harness/pkg");
    let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["check", "."])
        .current_dir(dir)
        .output()
        .expect("kanso runs");

    assert!(
        output.status.success(),
        "check refused a module whose tests read failures: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A deliberate exit is an err carrying a status, so it propagates by the rule
/// that already exists and the endpoint reads its code instead of reporting an
/// unhandled failure. Both engines, because an exit status is program output.
#[test]
fn a_deliberate_exit_sets_the_status_and_says_nothing() {
    let dir = std::env::temp_dir().join("kanso-exit-status");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp work dir");
    std::fs::write(
        dir.join("main.kso"),
        "import \"std/io\"\n\nio/write \"before\\n\" . (_ -> io/exit 3)\n",
    )
    .expect("the entry writes");

    for engine in [&[][..], &["--interp"][..]] {
        let out = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("run")
            .arg(".")
            .args(engine)
            .current_dir(&dir)
            .output()
            .expect("kanso runs");

        assert_eq!(out.status.code(), Some(3), "engine {engine:?} lost the status");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "before\n",
            "the effect before the exit did not run under {engine:?}"
        );
        assert!(
            String::from_utf8_lossy(&out.stderr).is_empty(),
            "a deliberate exit reported itself as a failure: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
