use std::process::Command;

/// A module that declares `id_of` and imports a module that exports one
/// calls its own when it writes `id_of` bare: its own arms stand ahead of the
/// import's in the group, and its own `fn id_of n` takes every value. The
/// check read the group with the import's arm first, so the call answered
/// the none `lib/id_of` can return, and `wanted`, which has no none arm, was
/// refused on both engines for a value it is never handed (#2473). Watched
/// failing before the fix.
#[test]
fn a_module_s_own_arms_answer_its_bare_call() {
    let path = format!("{}/tests/golden/own_arms_first", env!("CARGO_MANIFEST_DIR"));
    for engine in [&[][..], &["--interp"][..]] {
        let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("run")
            .arg(&path)
            .args(engine)
            .output()
            .expect("kanso binary runs");

        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "id 7\nid 0\n",
            "engine {engine:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
