//! Every gate that counts a run does it from the same directory, and that
//! directory's path length is pinned.
//!
//! The counted rows move with the working directory's path. Measured
//! 2026-09-18 with ONE binary, one corpus and nothing else varying, counting
//! `kanso run interp_corpus --interp` from directories that differ only in
//! name and length:
//!
//! ```text
//!   /tmp/p                             len  6   1,007,027,010
//!   /tmp/pathaaaa                      len 13   1,007,027,010
//!   /tmp/pathbbbb                      len 13   1,007,027,010
//!   /tmp/pathaaaaaaaa                  len 17   1,007,027,010
//!   /tmp/pathaaaaaaaaa                 len 18   1,007,027,010
//!   /tmp/pathaaaaaaaaaa                len 19   1,007,027,010
//!   /tmp/pathaaaaaaaaaaa               len 20   1,007,027,010
//!   /tmp/pathaaaaaaaaaaaa              len 21   1,007,004,925
//!   /tmp/pathaaaaaaaaaaaaaaaa          len 25   1,007,004,925
//!   /tmp/pathaaaaaaaaaaaaaaaaaaaaaaaa  len 33   1,007,004,925
//! ```
//!
//! Two values, 22,085 apart, with one step between 20 and 21. Equal-length
//! directories with different names agree, which rules the NAME out. The
//! longer path reads FEWER instructions, so this is not a cost that grows
//! with the string.
//!
//! The gates are already normalized against it -- each `cd`s into a fixed box
//! and runs under `env -i` -- so no CI row is in question. What is in question
//! is how narrow the margin is. There are TWO boxes, and the first draft of
//! this spec asserted there was one; it went red naming the second, which is
//! how the pair below got measured rather than assumed:
//!
//! ```text
//!   /tmp/kanso-compile-ir   21   compile, entry, library, interp, start-up
//!   /tmp/kanso-codegen      18   codegen, emit
//! ```
//!
//! 21 is one character past the step and 18 is three short of it, so the two
//! boxes sit on OPPOSITE sides of it. Renaming either re-bases its rows with
//! no compiler change behind them, and renaming the first by one character
//! does it by 22,085. That is the 2026-09-15 normalization ruling's case, and
//! a comment saying "keep this name" is the kind of thing that goes stale.
//!
//! The step was measured on the interpreted corpus. Whether the codegen
//! corpus steps in the same place is NOT measured here, and this spec does
//! not need it to: it pins each box at the length its own goldens were
//! measured at.

use std::path::Path;

/// The two boxes a counted run is taken from, each with the path length its
/// goldens were measured at. The NAME is arbitrary; the LENGTH is load-bearing.
const BOXES: &[(&str, usize)] = &[("/tmp/kanso-compile-ir", 21), ("/tmp/kanso-codegen", 18)];

/// Which box each gate that counts a run must use.
const GATES: &[(&str, &str)] = &[
    ("compile_instructions.sh", "/tmp/kanso-compile-ir"),
    ("entry_instructions.sh", "/tmp/kanso-compile-ir"),
    ("interp_instructions.sh", "/tmp/kanso-compile-ir"),
    ("library_box.sh", "/tmp/kanso-compile-ir"),
    ("library_instructions.sh", "/tmp/kanso-compile-ir"),
    ("startup_instructions.sh", "/tmp/kanso-compile-ir"),
    ("codegen_box.sh", "/tmp/kanso-codegen"),
    ("codegen_instructions.sh", "/tmp/kanso-codegen"),
    ("emit_instructions.sh", "/tmp/kanso-codegen"),
];

fn gates_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/gates")
}

/// Every `box=` assignment in the gates directory, as (gate, value).
fn declared() -> Vec<(String, String)> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(gates_dir()).expect("the gates directory is readable") {
        let path = entry.expect("a readable entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("sh") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("a readable gate");
        for line in text.lines() {
            if let Some(rest) = line.trim().strip_prefix("box=") {
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                found.push((name, rest.to_string()));
            }
        }
    }
    found
}

#[test]
fn every_gate_that_counts_a_run_uses_its_own_box() {
    let found = declared();
    let mut wrong = Vec::new();
    for (gate, expected) in GATES {
        match found.iter().find(|(name, _)| name == gate) {
            Some((_, value)) if value == expected => {}
            Some((_, value)) => wrong.push(format!("{gate}: box={value}, expected {expected}")),
            None => wrong.push(format!("{gate}: declares no box")),
        }
    }
    assert!(
        wrong.is_empty(),
        "a gate counting a run from a different path reads a different number \
         for the same binary: {wrong:?}"
    );
}

#[test]
fn no_gate_declares_a_box_outside_the_known_two() {
    let known: Vec<&str> = BOXES.iter().map(|(p, _)| *p).collect();
    let strays: Vec<String> = declared()
        .into_iter()
        .filter(|(_, value)| !known.contains(&value.as_str()))
        .map(|(gate, value)| format!("{gate}: box={value}"))
        .collect();
    assert!(
        strays.is_empty(),
        "a third box is a third path length, and no golden on disk was \
         measured at it: {strays:?}"
    );
}

#[test]
fn each_box_is_the_length_its_rows_were_measured_at() {
    for (path, len) in BOXES {
        assert_eq!(
            path.len(),
            *len,
            "{path} was measured at {len} characters. The interpreted row steps \
             by 22,085 between length 20 and 21, so changing this re-bases rows \
             with no compiler change behind them."
        );
    }
}
