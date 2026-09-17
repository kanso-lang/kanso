//! Everything a gate prints ABOUT ITS BINARY is a notice, not plain stdout.
//!
//! Plain stdout lands only in the job log, and the blob host GitHub redirects a
//! log fetch to answers from here with `CONNECT tunnel failed, response 403`.
//! So a property printed to stdout is a property nobody can read when two
//! sittings need comparing -- which is exactly when it is wanted.
//!
//! On 2026-09-17 that cost the ±13 hunt a round. Two attempts of run
//! 35192857066's successor, one commit and two builds, printed sha256
//! `fde1fb87…` and `13e6cf22…` and read the three compile rows byte-identical;
//! main's build of a tree that cannot change the compiler read thirteen away.
//! Two builds agreeing while a third disagrees is the shape of a layout
//! difference between builds, and `.text`/`.data`/`.bss` are what would settle
//! it. All three were printed on all three runs, and not one was readable.
//!
//! The sha became a notice in kanso#1479. The sections are the same argument
//! and this spec holds them to it.

use std::path::Path;

const GATES: [&str; 3] = [
    "scripts/gates/compile_instructions.sh",
    "scripts/gates/entry_instructions.sh",
    "scripts/gates/library_instructions.sh",
];

fn read(gate: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(gate);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{gate} is on disk: {e}"))
}

#[test]
fn every_compile_gate_reports_its_sections_as_a_notice() {
    for gate in GATES {
        let s = read(gate);
        assert!(
            s.contains("size --format=sysv"),
            "{gate} does not read its binary's sections at all"
        );
        let notices: Vec<&str> =
            s.lines().filter(|l| l.contains("::notice::") && l.contains("sections")).collect();
        assert_eq!(
            notices.len(),
            1,
            "{gate} should emit its section sizes as exactly one notice; it \
             emits {}:\n{notices:#?}",
            notices.len()
        );
    }
}

#[test]
fn the_sections_notice_carries_all_three() {
    // One notice rather than three, because GitHub keeps fifty annotations per
    // check run and this job already spends most of them on verdicts.
    for gate in GATES {
        let s = read(gate);
        let line = s
            .lines()
            .find(|l| l.contains("::notice::") && l.contains("sections"))
            .unwrap_or_else(|| panic!("{gate} emits no sections notice"));
        let file = line
            .split_once("$(cat ")
            .and_then(|(_, r)| r.split_once(')'))
            .map(|(f, _)| f.trim())
            .unwrap_or_else(|| panic!("{gate}'s sections notice reads no file: {line}"));
        let writer = s
            .lines()
            .find(|l| l.contains("> ") && l.contains(file))
            .unwrap_or_else(|| panic!("{gate} never writes {file}"));
        for want in ["text", "data", "bss"] {
            assert!(
                writer.contains(want),
                "{gate} leaves .{want} out of the line it notices: {writer}"
            );
        }
    }
}
