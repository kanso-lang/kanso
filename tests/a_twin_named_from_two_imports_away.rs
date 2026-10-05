use std::process::Command;

/// An import's type twin keeps the declaration it was cloned from when the
/// module holding it is imported in turn.
///
/// `steps` imports std/list, so it holds a twin of `list/step` that remembers
/// `list/step` as its origin. When `kinds` imports `steps`, the loader renames
/// that twin `steps/step`. Its origin is already qualified and must stay
/// `list/step`: prefixed again it reads `steps/list/step`, a type nothing
/// declares. The interpreter then lets a step fall through `_:steps/step` to
/// the next arm, and native refuses the build with `unknown type`.
///
/// The micro fixture an_imported_record_keeps_its_name used to reach this
/// path by building a record through its short name. Since that construction
/// is refused (kanso#1762), the micro fixture no longer reaches the renamed
/// twin, and the ratchet row naming this mutation stood blind from 2026-10-01.
/// An ascription two imports away is the path left. Watched red with the
/// filter removed.
#[test]
fn a_twin_named_from_two_imports_away_is_the_declaration() {
    let path = format!("{}/tests/golden/twin_origin", env!("CARGO_MANIFEST_DIR"));
    for engine in [&[][..], &["--interp"][..]] {
        let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("run")
            .arg(&path)
            .args(engine)
            .output()
            .expect("kanso binary runs");

        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "a step\nnot a step: 4\n",
            "engine {engine:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
