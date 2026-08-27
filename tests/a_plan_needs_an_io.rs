// `kanso run <file> --plan` prints what a program would do without doing it.
// When main answers a value rather than an io there is nothing to render, and
// the driver says so. The message is written as plain `error:` text and
// printed by src/main.rs, not built with `Diagnostic::new`, and no golden
// corpus reads the stderr of a --plan run — so the pin lives here, and
// tests/golden/unpinned_diagnostics.txt names this file as its coverage.
use std::process::Command;

#[test]
fn a_plan_needs_an_io() {
    let dir = std::env::temp_dir().join("kanso-plan-needs-io");
    let _ = std::fs::create_dir_all(&dir);
    let file = dir.join("a_value_not_an_io.kso");
    std::fs::write(&file, "x = 2\n\nx\n").expect("fixture writes");
    let out = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(&file)
        .arg("--plan")
        .output()
        .expect("kanso runs");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("main is not an io; there is no plan to show"),
        "expected the driver's refusal, got: {err}"
    );
    assert!(!out.status.success(), "a program with no plan must not exit zero");
}
