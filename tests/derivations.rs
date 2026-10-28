//! Derivations (#2186): an entry's `derive` lines, run while it compiles.
//!
//! Each case in tests/golden/derive is a program directory: an entry, the
//! module holding the convention, and the modules it describes. A case with a
//! `.stdout` golden compiles and prints that; a case with a `.stderr` golden is
//! refused with exactly that text. Both engines run every case, because the
//! derivation itself always runs on the interpreter, and what it declares has
//! to mean the same thing to native code as to the interpreter.
//!
//! Run from the manifest directory against a relative path, so a refusal
//! quotes the line it points at and the golden names the corpus, not the
//! clone.

use std::process::Command;

fn cases() -> Vec<String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/derive");
    let mut found: Vec<String> = std::fs::read_dir(&root)
        .expect("the derive corpus reads")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .map(|p| p.file_name().expect("a case is named").to_string_lossy().into_owned())
        .collect();
    found.sort();
    found
}

#[test]
fn every_derivation_case_says_what_its_golden_says_on_both_engines() {
    let cases = cases();
    assert!(cases.len() >= 6, "the derive corpus moved: {cases:?}");
    for name in cases {
        let case = format!("tests/golden/derive/{name}");
        let stdout = std::fs::read_to_string(format!("{case}.stdout")).ok();
        let stderr = std::fs::read_to_string(format!("{case}.stderr")).ok();
        for engine in [&[][..], &["--interp"][..]] {
            let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
                .arg("run")
                .arg(&case)
                .args(engine)
                .current_dir(env!("CARGO_MANIFEST_DIR"))
                .output()
                .expect("kanso runs");
            let out = String::from_utf8_lossy(&done.stdout);
            let err = String::from_utf8_lossy(&done.stderr);
            match (&stdout, &stderr) {
                (Some(want), None) => {
                    assert_eq!(err, "", "{name} {engine:?} was refused");
                    assert_eq!(&out, want, "{name} {engine:?}");
                }
                (None, Some(want)) => {
                    assert_eq!(&err, want, "{name} {engine:?}");
                    assert_eq!(done.status.code(), Some(2), "{name} {engine:?} exits 2");
                }
                _ => panic!("{name} needs exactly one of a .stdout and a .stderr golden"),
            }
        }
    }
}

/// `kanso derived` prints each constant a build derived, with a function
/// spelled by its name and the derivation that declared it, the way
/// `rails routes` prints a route table.
#[test]
fn the_derived_verb_prints_what_a_derivation_declared() {
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("derived")
        .arg("tests/golden/derive/a_route_table_names_real_functions")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("kanso runs");
    assert_eq!(String::from_utf8_lossy(&done.stderr), "");
    assert_eq!(
        String::from_utf8_lossy(&done.stdout),
        "routes = { \"/posts/index\":posts/index \"/posts/show\":posts/show }    \
         # conventions/routing\n"
    );
}
