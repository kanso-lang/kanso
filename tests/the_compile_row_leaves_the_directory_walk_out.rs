//! The compile row subtracts the directory walk, and the golden says it does.
//!
//! `compile_instructions` alternated across four CI jobs on one tree whose
//! compiler source was identical — 35,544,159, 35,544,162, 35,544,159,
//! 35,544,162. Once kanso#1566 put every function table inside the readable
//! part of the job log, three independent pairs of jobs were diffed frame by
//! frame, and all three put the WHOLE delta, both signs, in one frame of
//! 1,336: `<std::sys::fs::unix::ReadDir as Iterator>::next`. The entry,
//! library, startup, emit and interp tables were byte-identical in all three.
//!
//! This gate hands `kanso check` a DIRECTORY, so the compile opens one and
//! walks it; the entry and library gates hand it single FILES and carry no
//! such frame at all, which is why this row alone drew.
//!
//! kanso#1569 established that it cannot be normalized: the entry SET is
//! identical on every host and the ORDER readdir returns it in is not. So the
//! term is excluded under the 2026-09-15 rule, which says exactly that — what
//! cannot be normalized is left out and the exclusion is named in the golden's
//! header.
//!
//! What this spec pins is that the subtraction and the header travel together.
//! A gate that stops subtracting goes back to drawing two faces; a header that
//! stops saying so leaves a reader with a row that does not match a profile and
//! nothing to explain it.

use std::path::Path;

fn gate() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/gates/compile_instructions.sh"),
    )
    .expect("the gate reads")
}

fn golden() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("bench/compile_instructions_golden.txt"),
    )
    .expect("the golden reads")
}

/// The frame whose subtree is the whole per-entry iteration. The OUTER one:
/// the sys-internal frame carries the drift, and this one contains it along
/// with the libc `readdir` beneath.
const WALK: &str = "<std::fs::ReadDir as core::iter::traits::iterator::Iterator>::next";

#[test]
fn the_gate_subtracts_the_walk_from_both_readings() {
    let g = gate();
    assert!(
        g.contains(WALK),
        "scripts/gates/compile_instructions.sh no longer names the directory \
         walk, so it is counting a term whose cost is the filesystem's answer. \
         Four CI jobs on one tree read two values three apart before this was \
         taken out."
    );
    // The gate reads the row twice and compares them; an exclusion applied to
    // one reading and not the other makes that comparison a lie.
    assert!(
        g.contains("own=$((own - printed - walked))"),
        "the first reading does not subtract the walk"
    );
    assert!(
        g.contains("again=$((again - again_printed - again_walked))"),
        "the SECOND reading does not subtract the walk, so the gate's own \
         two readings are taken on different terms and agreeing means nothing"
    );
}

#[test]
fn the_golden_header_names_the_exclusion() {
    let h = golden();
    assert!(
        h.contains("ReadDir"),
        "bench/compile_instructions_golden.txt does not say that the directory \
         walk is left out. The 2026-09-15 rule allows excluding a term that \
         cannot be normalized ONLY with the exclusion named in the header — a \
         term that is counted and explained, or excluded and not, is what it \
         forbids."
    );
}

#[test]
fn the_walk_is_taken_off_where_a_reader_can_see_it() {
    // The printed line's own cost is echoed for this reason already: a row
    // that drifts again is unanswerable from a number that only ever appears
    // subtracted.
    assert!(
        gate().contains("compile_walked="),
        "the walk's own cost is subtracted and never printed, so the next time \
         this row moves nobody can tell whether the excluded subtree moved with it"
    );
}
