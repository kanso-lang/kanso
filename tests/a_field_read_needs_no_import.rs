//! A field is read without importing the type that owns it (ch03, and the
//! 2026-10-12 gavel on #1894). regex's port read `h.start` in a module that
//! imports nothing, on a value another module built, and every engine refused
//! it at check with "no record type has a field `start`". A library checked
//! before anything imports it now leaves such a field to the program, which
//! refuses it there when no type in the program declares it.

use std::process::Command;

fn kanso(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(args)
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/field_reads"))
        .output()
        .expect("kanso runs")
}

/// Watched red: "no record type has a field `start` (module u)" on both
/// engines.
#[test]
fn a_module_reads_a_field_of_a_type_it_never_imports() {
    for engine in [&["run", "runs/main.kso"][..], &["run", "runs/main.kso", "--interp"][..]] {
        let done = kanso(engine);
        assert_eq!(
            String::from_utf8_lossy(&done.stdout),
            "2..5\n",
            "{engine:?}: {}",
            String::from_utf8_lossy(&done.stderr)
        );
    }
}

/// The whole program still answers for a field no type in it declares.
#[test]
fn a_field_no_type_in_the_program_declares_is_refused() {
    let done = kanso(&["play", "typo/typo.kso"]);
    let expected = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/golden/field_reads/typo.stderr"
    ))
    .expect("golden");
    assert_eq!(String::from_utf8_lossy(&done.stderr), expected);
    assert_eq!(done.status.code(), Some(2));
}
