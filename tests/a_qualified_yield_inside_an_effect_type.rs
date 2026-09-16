use std::process::Command;

/// A TYPE NAME INSIDE A SHELL IS SEVERAL NAMES. The import check asks each
/// file which module qualifiers it uses, and it asked by splitting a name at
/// its first slash — so `<g/cell>effect` answered `<g`, a qualifier no import
/// can match. The file was refused for borrowing an import it had written,
/// and in the same run that import read as unused: two diagnostics, both
/// wrong, for a program that compiles.
///
/// `map[string g/cell]` splits the same way and is fixed by the same scan:
/// every run of name characters is a name, and the ones holding a slash are
/// the qualified ones.
///
/// Watched red on the compiler before the fix, on both engines:
/// `error[import]: `<g` is not imported here`.
#[test]
fn a_qualified_yield_is_the_module_its_qualifier_names() {
    let path = format!("{}/tests/golden/qualified_yield", env!("CARGO_MANIFEST_DIR"));
    for engine in [&[][..], &["--interp"][..]] {
        let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("run")
            .arg(&path)
            .args(engine)
            .output()
            .expect("kanso binary runs");

        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "cell 7\n",
            "engine {engine:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
