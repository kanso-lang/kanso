//! An import reaches only the file that writes it (the 2026-10-12 gavel). A
//! name a module declares and an import also exports forms one overload group
//! in a file that imports it, as the 2026-08-29 ruling says. xsv's module had
//! a private `matches` in one file and std/regexp imported by another, and
//! std/regexp's arms joined the private group in the file that never imported
//! it: the check refused `matches index k != []` because one of regexp's arms
//! can answer an err.

use std::process::Command;

/// Watched red: "this can be an err and `!=` wants a value" in m/join.kso,
/// on both engines.
#[test]
fn a_private_function_keeps_its_arms_beside_a_siblings_import() {
    for engine in [&["run", "main.kso"][..], &["run", "main.kso", "--interp"][..]] {
        let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .args(engine)
            .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/import_scope/runs"))
            .output()
            .expect("kanso runs");
        assert_eq!(
            String::from_utf8_lossy(&done.stdout),
            "true true [regexp/hit 1 2 []]\n",
            "{engine:?}: {}",
            String::from_utf8_lossy(&done.stderr)
        );
    }
}
