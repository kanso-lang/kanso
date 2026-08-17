use std::process::Command;

/// json decode and encode across a module boundary, both engines. The entry
/// decodes a document holding a nested list, a float, a bool and a null, then
/// a second module encodes it twice along with a map literal and counts the
/// characters.
///
/// It exercises the shape a real caller has — a program that imports std/json
/// and a helper module that imports it too — which is the diamond, and which
/// died with `no overload of loop/json/encode_onto matches these arguments`
/// until a module reached by two paths became one module. `null` was the only
/// value that failed, because every other arm of `encode_onto` matches a
/// literal or a builtin type and only `json_null` matches a type std/json
/// owns.
///
/// The expected output is the figure this fixture recorded when it was
/// written, months before the fork was noticed: `twice: 104`, byte for byte.
#[test]
fn a_document_reencodes_across_a_module_boundary() {
    let path = format!("{}/tests/golden/runtime/reencode", env!("CARGO_MANIFEST_DIR"));
    let expected = std::fs::read_to_string(format!("{path}.out")).expect("the golden reads");
    for engine in [&[][..], &["--interp"][..]] {
        let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("run")
            .arg(&path)
            .args(engine)
            .output()
            .expect("kanso binary runs");

        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            expected,
            "engine {engine:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
