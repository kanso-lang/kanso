//! The three compile rows do not count the line the run prints.
//!
//! `kanso check` prints one line when it finishes and `kanso::main` inclusive
//! counts it. Under that line `LineWriter` runs `core::slice::memchr::memrchr`
//! over the formatted bytes to find the last newline, and what that frame
//! costs moves with the binary's layout. Five CI builds on 2026-09-17, across
//! trees whose compiler source was identical, gave two faces thirteen apart on
//! all three rows — while `startup_instructions`, whose gate prints nothing,
//! gave one value on every one of them.
//!
//! Not printing does not help: an environment variable asking for quiet costs
//! 11,606 instructions against the 1,116 the quiet saves, because the compiler
//! asks getenv about seven thousand times and each ask walks the block, and an
//! argv entry costs about 2,047. Both move the initial process layout, which
//! is the same class of thing the thirteen is.
//!
//! So the term is excluded, per the 2026-09-15 rule — what cannot be
//! normalized is left out and the exclusion is named in the golden's header.
//! Each gate subtracts `std::io::stdio::_print` inclusive from the anchored
//! reading, on BOTH of its two measured runs, and a gate that stopped doing it
//! would go back to pinning a coin with nothing to say so.

use std::path::Path;

const GATES: [&str; 3] =
    ["compile_instructions.sh", "entry_instructions.sh", "library_instructions.sh"];

/// Every line that takes a reading from a profile, paired with whether the
/// print subtree is taken off it.
fn readings(script: &str) -> Vec<(usize, bool)> {
    let mut found = Vec::new();
    let lines: Vec<&str> = script.lines().collect();
    for (at, line) in lines.iter().enumerate() {
        // The two that become the row. The gate also annotates a listing for
        // the job log and reads the print's own cost through a helper, and
        // neither of those is a reading of the row.
        let takes_the_row = line.starts_with("own=$(callgrind_annotate")
            || line.starts_with("again=$(callgrind_annotate");
        if !takes_the_row {
            continue;
        }
        let subtracted = lines[at..]
            .iter()
            .take(6)
            .any(|near| near.contains("printed_cost") || near.contains("- printed"));
        found.push((at + 1, subtracted));
    }
    found
}

#[test]
fn every_reading_in_the_three_gates_takes_the_printed_line_off() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/gates");
    let mut blind = Vec::new();
    let mut counted = 0;
    for gate in GATES {
        let path = root.join(gate);
        let script = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{} is read: {e}", path.display()));
        let found = readings(&script);
        assert_eq!(
            found.len(),
            2,
            "{gate} takes {} anchored readings; each gate measures twice",
            found.len()
        );
        for (at, subtracted) in found {
            counted += 1;
            if !subtracted {
                blind.push(format!("{gate}:{at}"));
            }
        }
    }
    assert_eq!(counted, 6, "three gates, two readings each");
    assert!(
        blind.is_empty(),
        "{} of {counted} readings count the line the run prints, so the row \
         carries a frame whose cost moves with the binary's layout: {}",
        blind.len(),
        blind.join(", ")
    );
}

#[test]
fn the_helper_names_the_frame_it_subtracts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/gates");
    for gate in GATES {
        let script = std::fs::read_to_string(root.join(gate)).expect("the gate is read");
        assert!(
            script.contains("std::io::stdio::_print"),
            "{gate} subtracts a cost without naming the frame it comes from"
        );
    }
}
