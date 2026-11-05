//! A field read through a name annotated with its type answers that type's
//! field (kanso#2350). Two record types may share a field name, and every
//! read of it is one getter group with an arm per type, so inference used to
//! answer each read with the join of every arm: `r.body` in `fn size r:letter`
//! carried the none a `note` body can hold, and the none check refused
//! `length (as_bytes r.body)` for a letter whose body is always a string.

use std::process::Command;

fn kanso(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(args)
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/field_reads"))
        .output()
        .expect("kanso runs")
}

/// Watched red before the fix, on both engines: "this can be a none and
/// `length` does not take one" at `as_bytes r.body`.
#[test]
fn a_letters_body_is_not_a_notes_none() {
    for engine in [&[][..], &["--interp"][..]] {
        let mut args = vec!["run", "its_own_type/main.kso"];
        args.extend_from_slice(engine);
        let done = kanso(&args);
        assert_eq!(
            String::from_utf8_lossy(&done.stdout),
            "8\n7\n",
            "{engine:?}: {}",
            String::from_utf8_lossy(&done.stderr)
        );
    }
}

/// The none a note's body can hold is still refused where a note is read.
#[test]
fn a_notes_body_keeps_its_none() {
    let expected = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/golden/field_reads/another_types_none.stderr"
    ))
    .expect("golden");
    for engine in [&[][..], &["--interp"][..]] {
        let mut args = vec!["run", "another_types_none/main.kso"];
        args.extend_from_slice(engine);
        let done = kanso(&args);
        assert_eq!(String::from_utf8_lossy(&done.stderr), expected, "{engine:?}");
        assert_eq!(done.status.code(), Some(2), "{engine:?}");
    }
}
