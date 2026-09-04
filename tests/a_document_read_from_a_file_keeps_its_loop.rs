//! A document read from a file threads through a loop that allocates, and the
//! loop brackets every iteration.
//!
//! This is the property the io half of the 2026-09-03 gavel took away and
//! gave back. `os/read_file` answers `text | file_not_found`, and a builtin
//! cannot name a type declared in kanso, so the builtin answers the text or
//! `none` and the wrapper names the second — the shape `os/env` already uses.
//! It answered a two-element list for a day, and the list cost the caller the
//! string's type: inference gives a list index the top set, `beat.rs` reads
//! that set to decide whether a slot may be carried across a rewind, and an
//! untyped slot keeps the grow-only arena. jsonbench decoded the same bytes
//! with 248 arena blocks instead of 2 and a 260 MB peak instead of 2 MB.
//!
//! Watched red with the list put back in `read_value` (src/eval.rs) and
//! `found` back to `if r[1]! r[2]!`: both programs report `beat_iters=0`,
//! the loop never bracketing at all.
//!
//! `beat_iters` and not `arena_blocks`, which is the counter the same failure
//! moves on jsonbench. These programs are small enough to fit one block
//! either way — measured, both shapes report 1 — so asserting the block count
//! here would be a check that cannot fail. The block count is pinned where it
//! is sensitive, in bench/cost_golden.txt.
//!
//! `reading_insisted.kso` is the third program and the bang's own case. It
//! read `beat_iters=1` against the other two's 201 and 801: `desc_yield` was
//! a table keyed on a chain head's BARE name, `os/read_file` hit it because
//! the name collides with the builtin's, and `os/read_file!` — same body, one
//! character more — missed and fell to the top set. Eight std wrappers missed
//! the same way. The yield is carried per declaration now, so the table is not
//! what answers and there is no list to keep up to date.

use std::process::Command;

/// The loop brackets rather than running once: `beat_iters` is one per
/// iteration plus one for the program. Measured at 0 when the loop keeps the
/// grow-only arena: nothing brackets, so nothing counts.
#[test]
fn the_loop_brackets_every_iteration() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/read_beat");
    for (file, want) in [
        ("reading.kso", "beat_iters=201"),
        ("reading_long.kso", "beat_iters=801"),
        ("reading_insisted.kso", "beat_iters=201"),
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("play")
            .arg(file)
            .env("KANSO_COUNTERS", "1")
            .current_dir(dir)
            .output()
            .expect("kanso runs");
        let said = String::from_utf8_lossy(&out.stderr);
        assert!(said.lines().any(|l| l == want), "{file} wanted {want}: {said}");
    }
}

/// And the read still answers what it says it answers, on both engines: the
/// text when the file is there, `file_not_found` when it is not.
#[test]
fn a_missing_file_reaches_the_arm_that_names_it() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/read_beat");
    for interp in [false, true] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kanso"));
        cmd.arg("play").arg("absent.kso");
        if interp {
            cmd.arg("--interp");
        }
        let out = cmd.current_dir(dir).output().expect("kanso runs");
        let said = String::from_utf8_lossy(&out.stdout);
        assert_eq!(said, "absent: nowhere.txt\n", "interp={interp}: {said}");
    }
}
