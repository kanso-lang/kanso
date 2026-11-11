//! An entry with a `derive` line imports `std/derive` without writing the
//! import, and the module that declares the derivation imports it too. Two
//! imports of one module compile it once.
//!
//! The loader keeps a module from its first compile when it has counted two
//! imports of it, and it counts by reading the program's text before anything
//! compiles. The entry's import is not in the text, so the count read one,
//! the first compile was not kept, and `std/derive` was compiled a second
//! time for the entry (kanso#2442's last repeat).
//!
//! The fixture is the smallest program in the derive corpus: an entry, the
//! module that declares its derivation and the module it describes.

use std::process::Command;

fn loads_of(module: &str) -> usize {
    let entry = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/derive/a_derivation_declares_a_record_type/main.kso");
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("check")
        .arg(&entry)
        .env("KANSO_PHASES", "1")
        .output()
        .expect("kanso runs");
    let said = String::from_utf8_lossy(&done.stderr);
    assert!(done.status.success(), "the fixture should check clean:\n{said}");
    let wanted = format!("load {module}");
    said.lines().filter(|line| *line == wanted).count()
}

#[test]
fn an_entry_that_derives_compiles_derive_once() {
    assert_eq!(loads_of("std/derive"), 1, "std/derive was compiled more than once");
}

/// The count has something to read: with no line for a module the test above
/// would pass on a loader that printed nothing.
#[test]
fn the_loads_are_printed() {
    assert_eq!(loads_of("std/list"), 1, "the loader's lines changed, or std/list is not loaded");
}
