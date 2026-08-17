use std::process::Command;

/// Two directories both named `shape`, so both modules qualify their
/// declarations under the same prefix and both `blank` types answer to
/// `shape/blank`. The compiler gave each its own type id and wrote both into
/// the same switch, which clang refused: `duplicate case value in switch`.
/// Nothing built, on a program with no overlapping function to blame.
///
/// One module means one `shape/blank`, and the two arrivals collapse. The
/// names were already indistinguishable to every consumer — what changed is
/// that the emitter stops writing two cases for one name. Watched failing on
/// main before this landed.
#[test]
fn two_modules_of_one_name_emit_one_type() {
    let path = format!("{}/tests/golden/samename", env!("CARGO_MANIFEST_DIR"));
    for engine in [&[][..], &["--interp"][..]] {
        let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("run")
            .arg(&path)
            .args(engine)
            .output()
            .expect("kanso binary runs");

        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "A\nB\n",
            "engine {engine:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
