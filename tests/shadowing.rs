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

/// A dependency's local in a list is the local. A module's dependencies are
/// walked with the module's own table of functions, so a local `both` in one
/// file was looked up among the functions of the file that imports it, and
/// its list was refused as a call missing its parentheses (#2466). A bind
/// below a guard is the case: one above it is in the walk's bound names.
#[test]
fn a_local_in_a_dependencys_list_is_not_the_importers_function() {
    for engine in [&[][..], &["--interp"][..]] {
        let output = run("held", engine);

        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "[\"one\" \"xx\" \"\"]\n",
            "engine {engine:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// The refusal still stands where the call is written: a dependency's own
/// function in its own list, with no parentheses, is named in its own file.
#[test]
fn a_dependencys_own_bare_call_in_a_list_is_still_refused() {
    let output = run("called", &[]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "a bare call in a list ran: {stderr}");
    assert!(stderr.contains("`twice` takes 1 argument(s)"), "{stderr}");
    assert!(stderr.contains("called/pair.kso:6:10"), "{stderr}");
}
