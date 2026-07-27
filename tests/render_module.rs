use std::process::Command;

fn run_module(dir: &str, extra: &[&str]) -> std::process::Output {
    let path = format!("{}/tests/golden/render_module/{dir}", env!("CARGO_MANIFEST_DIR"));
    let mut command = Command::new(env!("CARGO_BIN_EXE_kanso"));
    command.arg("run").arg(&path).args(extra);
    command.output().expect("kanso binary runs")
}

/// A directory module's to_string arm joins the render group even when
/// the type lives in a different file than the arm. Watched red: before
/// the module path called the ambient merge, both engines printed the
/// structural `money 350`.
#[test]
fn a_module_spread_over_files_renders_through_its_own_arm() {
    for engine in [&[][..], &["--interp"][..]] {
        let output = run_module("money", engine);

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout,
            "$3.50\n",
            "engine {engine:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.status.success());
    }
}

/// The ownership rule holds in directory modules: an arm on a sentinel
/// is rejected at the definition site, not silently outranked.
#[test]
fn a_module_arm_on_a_sentinel_is_rejected() {
    let output = run_module("orphan", &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "expected a compile error: {stderr}");
    assert!(stderr.contains("error[ownership]"), "wrong diagnostic: {stderr}");
}
