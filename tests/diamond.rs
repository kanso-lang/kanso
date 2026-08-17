use std::process::Command;

/// GAVEL 51, Clay 2026-08-17: "one module". `main` imports `shape` directly
/// and imports `mid`, which imports `shape` too — a diamond, and the ordinary
/// shape of any program with a dependency.
///
/// Both routes used to build their own `shape`: the .ll carried
/// `mid/shape/describe` beside `shape/describe`, and `mid/shape/blank` beside
/// `shape/blank`. A value the entry built matched no arm the middle module
/// compiled, so `mid/via b` died with `no overload of mid/shape/describe
/// matches these arguments` while the identical call from the entry answered.
/// Watched red before the loader collapsed the diamond.
///
/// The fielded case is here because the first diagnosis blamed bare nullary
/// types — `filled 3` has a field and failed the same way, which is what
/// moved the finding from "about null" to "about every user type".
#[test]
fn a_module_reached_by_two_paths_is_one_module() {
    let path = format!("{}/tests/golden/diamond", env!("CARGO_MANIFEST_DIR"));
    for engine in [&[][..], &["--interp"][..]] {
        let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("run")
            .arg(&path)
            .args(engine)
            .output()
            .expect("kanso binary runs");

        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "entry:    blank\nimported: blank\nfielded:  filled 3\n",
            "engine {engine:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
