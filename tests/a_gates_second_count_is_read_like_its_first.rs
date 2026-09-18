//! A gate that counts twice reads the second count the way it read the first.
//!
//! Seven instruction gates take a SECOND callgrind pass when a row disagrees
//! with its golden, and the whole point of it is to separate "this binary
//! counted two numbers" from "this binary disagrees with the golden". Those
//! two are settled differently: the first halts the vein and is hunted, the
//! second is an ordinary ratchet. So the second reading is only worth taking
//! if it is the same quantity as the first.
//!
//! Four of the gates subtract a printed line from the row, because what
//! `std::io::stdio::_print` costs moves with the binary's layout rather than
//! with anything the code does, and the 2026-09-15 rule excludes what cannot
//! be normalized. For one day `interp_instructions.sh` took it off the first
//! reading and not off the second, so a stable binary was reported as having
//! counted two numbers exactly `interp_printed` apart. kanso#1502 and
//! kanso#1504 both read 1,138,001,437 and then 1,138,004,452 on 2026-09-18 --
//! a gap of 3,015, with `interp_printed=3015` in both job logs. Two branches,
//! two runners, the same pair, and the gate said in words that the binary was
//! unstable when it had counted the same number twice.
//!
//! That defect lives on the FAILURE path, which is the only path the second
//! count exists on, so it fired exactly when a reader most needed the answer
//! and never otherwise. Nothing in the tree could see it: the row gates are
//! CI-only, and the job log's verdict was a sentence rather than a number.
//!
//! So the property is pinned here instead of being remembered. A gate that
//! subtracts a printed cost from its first reading subtracts one from its
//! second. The three that were already right -- compile, entry, library --
//! hold it by construction, because kanso#1487 gave all three the reader as a
//! function and called it twice. kanso#1505 brought the subtraction to the
//! interpreted row the same day and open-coded the pipeline instead, reaching
//! the first reading and not the second. That is why this spec reads for the
//! CALL rather than for the text of the pipeline: the defect is not a missing
//! subtraction anybody could see, it is one call site out of two.
//!
//! THE OTHER SIX GATES WERE SWEPT BY HAND when this was written, for the wider
//! property this spec only partly reaches: that the second reading measures
//! the same quantity as the first. All five that anchor an inclusive frame
//! anchor the SAME frame in both readings. `emit_instructions.sh` reads both
//! through one `emit_cost()`. `codegen_instructions.sh` sums the child tree
//! rather than anchoring, and counts the processes each reading saw, because a
//! second reading that sees fewer of them is not measuring the same thing --
//! which is this defect's shape, guarded against before it had a name. So the
//! printed line was the only asymmetry in the tree on 2026-09-18, and a gate
//! that grows a new one grows it in a form this spec cannot see.

use std::path::{Path, PathBuf};

fn gates() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/gates");
    let mut out: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("scripts/gates is readable")
        .map(|e| e.expect("a directory entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "sh"))
        .collect();
    out.sort();
    out
}

/// The gates that take a second callgrind pass. `again=` is how every one of
/// them spells the second reading, and it is assigned once.
fn takes_a_second_count(body: &str) -> bool {
    body.lines().any(|l| l.trim_start().starts_with("again="))
}

/// Whether the row this gate pins has a printed line taken off it.
///
/// The marker is the READ of the frame out of a profile, not the name of the
/// frame, and the difference is not pedantry: every gate that is exempt
/// explains the exemption in prose that names `std::io::stdio::_print`, so a
/// looser match makes `startup_instructions.sh` look like it subtracts a line
/// its program never prints. This is the same constant
/// `every_anchored_gate_answers_for_the_printed_line.rs` reads by, and the
/// two specs agree by construction because they agree on what the read is.
fn subtracts_a_printed_cost(body: &str) -> bool {
    body.contains(r":std::io::stdio::_print \[")
}

#[test]
fn a_gate_that_counts_twice_subtracts_the_printed_line_from_both() {
    let mut checked = Vec::new();
    for path in gates() {
        let body = std::fs::read_to_string(&path).expect("a gate is readable");
        if !takes_a_second_count(&body) || !subtracts_a_printed_cost(&body) {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().into_owned();

        // The reader is named once and called twice: once for the first
        // profile and once for the second. Counting the CALLS is what
        // distinguishes a gate that reads both from one that reads one and
        // open-codes the other, which is the shape the defect took.
        assert!(
            body.contains("printed_cost() {"),
            "{name} takes a printed line off its row but does not name the \
             reader as a function. Both readings must come from one reader, \
             or they drift apart the way interp_instructions.sh did: the \
             first reading minus the printed line, the second raw, and a \
             stable binary reported as unstable by exactly that many \
             instructions."
        );
        let calls = body.matches("printed_cost /tmp/").count();
        assert_eq!(
            calls, 2,
            "{name} calls printed_cost {calls} time(s). It takes two \
             callgrind passes and both rows carry the printed line, so both \
             must have it taken off. One call means the second reading is the \
             raw frame and every comparison against the first is short by \
             whatever the printed line cost."
        );
        checked.push(name);
    }

    // A spec that silently matched nothing would pass forever. The four are
    // named so that a gate leaving this set is a deliberate edit here.
    assert_eq!(
        checked,
        vec![
            "compile_instructions.sh",
            "entry_instructions.sh",
            "interp_instructions.sh",
            "library_instructions.sh",
        ],
        "the gates that subtract a printed line and count twice have changed"
    );
}
