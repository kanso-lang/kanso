//! A gate that measures a command which WRITES takes its second reading from a
//! box in the same state the first one started in.
//!
//! `codegen_instructions.sh` counts `kanso build`, and a build writes its
//! output beside itself. So the second reading -- the one kanso#1463 added to
//! separate "this binary is stable" from "this vein is halted" -- was asking a
//! different question of a different box. On 2026-09-16 it counted five
//! processes where the first counted six, and the dev row read 9,273,832,919
//! and then 1,071,604,124: an incremental build that skipped the work, printed
//! as a REPRODUCTION FAILURE about the compiler.
//!
//! The project's rule is that external state is normalized before it is
//! measured. Here that means re-staging: `codegen_box.sh` begins `rm -rf`, so a
//! call to it puts the box back to bytes the gate chose rather than bytes the
//! last measurement left.
//!
//! The other three compile gates run `kanso check`, which writes nothing, which
//! is why this spec names the build gate alone rather than sweeping them.

use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

const GATE: &str = "scripts/gates/codegen_instructions.sh";
const BOX: &str = "scripts/gates/codegen_box.sh";

fn gate() -> String {
    std::fs::read_to_string(root().join(GATE)).expect("the codegen gate is on disk")
}

/// Byte offsets of the two measured runs, named by the profile prefix each one
/// writes. Reading them off the script rather than off a line number is what
/// keeps this spec pointed at the runs when the script around them moves.
fn readings(s: &str) -> (usize, usize) {
    let first = s
        .find("--callgrind-out-file=/tmp/cg.codegen.$tier.%p")
        .expect("the first measured run writes /tmp/cg.codegen.$tier.%p");
    let second = s
        .find("--callgrind-out-file=/tmp/cg.codegen.${tier}b.%p")
        .expect("the second measured run writes /tmp/cg.codegen.${tier}b.%p");
    assert!(
        first < second,
        "the b-suffixed profiles are the SECOND reading; if they now come first \
         this spec is reading the script backwards"
    );
    (first, second)
}

#[test]
fn the_box_is_restaged_between_the_two_readings() {
    let s = gate();
    let (first, second) = readings(&s);
    let between = &s[first..second];
    assert!(
        between.contains("stage_and_warm"),
        "nothing re-stages the box between the first measured run and the \
         second, so the second counts a build the first one already did. The \
         text between them was:\n{between}"
    );
}

#[test]
fn the_first_reading_is_staged_too() {
    let s = gate();
    let (first, _) = readings(&s);
    let before = &s[..first];
    assert!(
        before.contains("\nstage_and_warm\n"),
        "the first measured run is not preceded by a staging call, so the two \
         readings do not start from the same box even with the second one fixed"
    );
}

#[test]
fn staging_clears_the_box_rather_than_writing_over_it() {
    let staging = std::fs::read_to_string(root().join(BOX)).expect("the box script is on disk");
    assert!(
        staging.contains("rm -rf \"$box\""),
        "re-staging only normalizes the box if it CLEARS it first; copying over \
         a directory the last build wrote to leaves that build's output in place"
    );
}

#[test]
fn staging_warms_both_tiers_every_time() {
    let s = gate();
    let body = s
        .split_once("stage_and_warm() {")
        .expect("the staging step is a shell function named stage_and_warm")
        .1
        .split_once("\n}\n")
        .expect("stage_and_warm closes on a line of its own")
        .0;
    assert!(
        body.contains("sh scripts/gates/codegen_box.sh"),
        "stage_and_warm does not re-stage the box"
    );
    let warms = body.matches("./kanso build pkg/codegen_corpus").count();
    assert_eq!(
        warms, 2,
        "both tiers are warmed on every staging, or the reading that follows \
         counts a runtime.c compile the other one does not -- which is the \
         start-up row's kanso#1461 bug, one binary reading 6,018,427 and then \
         4,869,632"
    );
}
