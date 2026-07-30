use std::process::Command;

/// A package may not turn its own err into a value — the two-universe rule —
/// and a library that cannot assert its own failures cannot be trusted. The
/// way out is not an exemption: the harness is a foreign party, so a builtin
/// doing the reading is a stranger rescuing, which the rule already permits.
///
/// Watched red twice, each guard alone: with the file gate off, the
/// production fixture in the error corpus stops being refused; with the hole
/// in infectiousness closed, the err propagates past `failed?` and both of
/// these fail instead of passing.
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
        stdout.contains("2 passed, 0 failed"),
        "{stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success());
}

/// And `check` accepts the same module, because the surface is gated by the
/// file that declares the caller rather than by the verb — a name that
/// existed under one verb and not another would be worse to explain.
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
