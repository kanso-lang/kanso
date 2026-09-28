//! An annotation may name a type any file of its module declares.
//!
//! A module's files share their declarations: a function one file declares is
//! called from the next, and a record one file declares is built there. Until
//! 2026-09-28 an annotation was the exception. The checker asked only the
//! annotating file's own types, so `p:pt` with `pt` declared in the file beside
//! it was refused as naming no type, on every engine, because every engine
//! runs the same checker. Found by the generated-program differential, which
//! splits a program across two files of one module.

use std::process::Command;

fn run(fixture: &str, engine: &[&str]) -> (String, String) {
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(format!("tests/golden/sibling_types/{fixture}"))
        .args(engine)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("kanso runs");
    (
        String::from_utf8_lossy(&done.stdout).into_owned(),
        String::from_utf8_lossy(&done.stderr).into_owned(),
    )
}

const ENGINES: [&[&str]; 2] = [&[], &["--interp"]];

#[test]
fn an_annotation_names_a_type_the_next_file_declares() {
    for engine in ENGINES {
        let (out, err) = run("annotates", engine);
        assert_eq!(
            out, "[\"tag 2\" \"pt 3\" \"other\"]\n",
            "{engine:?} did not dispatch on the sibling file's types: {err}"
        );
    }
}
