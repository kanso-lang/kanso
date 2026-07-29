use std::process::Command;

fn run(dir: &str, extra: &[&str]) -> std::process::Output {
    let path = format!("{}/tests/golden/shadowing/{dir}", env!("CARGO_MANIFEST_DIR"));
    let mut command = Command::new(env!("CARGO_BIN_EXE_kanso"));
    command.arg("run").arg(&path).args(extra);
    command.output().expect("kanso binary runs")
}

/// A parameter shadowing a bare-enrolled import is the parameter. Loading a
/// dependency qualifies every identifier the module declares, and enrollment
/// puts its own imports among those — so `foo` inside `bar` was rewritten
/// into `middle/foo` and `length` was handed a function instead of the list.
/// Watched red on main, which the shadow ban does not catch because an
/// enrolled name is deliberately shadowable.
#[test]
fn a_parameter_shadowing_an_enrolled_import_is_the_parameter() {
    for engine in [&[][..], &["--interp"][..]] {
        let output = run("app", engine);

        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "3\n",
            "engine {engine:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
