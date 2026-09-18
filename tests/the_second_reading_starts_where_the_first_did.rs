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

/// Both tiers are warmed on every staging, and the test RUNS the function
/// rather than reading it.
///
/// It used to count the literal `./kanso build` lines, and went red the day the
/// two became a loop over the two flags -- the property held and the spelling
/// moved, which is a spec pinned to the wrong thing. So `stage_and_warm` is
/// extracted, the box staging is stubbed out, a fake `kanso` on PATH records
/// what it was called with, and the assertion is on what ran.
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

    let dir = std::env::temp_dir().join(format!("kanso_warm_probe_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the probe directory is made");
    let log = dir.join("calls");
    std::fs::write(
        dir.join("kanso"),
        format!("#!/bin/sh\nprintf '%s\\n' \"$*\" >> {}\n", log.display()),
    )
    .expect("the fake kanso writes");
    let mut perm = std::fs::metadata(dir.join("kanso")).expect("stat").permissions();
    {
        use std::os::unix::fs::PermissionsExt;
        perm.set_mode(0o755);
    }
    std::fs::set_permissions(dir.join("kanso"), perm).expect("chmod");

    // The box staging builds the compiler; it is not what this is testing.
    let body = body.replace("sh scripts/gates/codegen_box.sh", ":");
    // `stage_and_warm` calls the gate's own `clear_output`, which empties the
    // output path before each warm build -- `ld` reads whatever is already at
    // `-o` and what it finds there is worth 2,354 instructions. Extracting one
    // function and running it standalone leaves the helpers it calls undefined,
    // so the helper is carried across too rather than stubbed: a stub would let
    // the two drift apart, and this harness exists to run the real body.
    let helper = s
        .split_once("clear_output() {")
        .expect("the gate defines clear_output")
        .1
        .split_once("\n}\n")
        .expect("clear_output closes on a line of its own")
        .0;
    let script = format!(
        "set -e\ntune=t\nbox={dir}\nclear_output() {{\n{helper}\n}}\n\
         stage_and_warm() {{\n{body}\n}}\nstage_and_warm\n",
        dir = dir.display(),
    );
    let out = std::process::Command::new("sh")
        .arg("-c")
        .arg(&script)
        .env("PATH", format!("{}:/usr/bin:/bin", dir.display()))
        .output()
        .expect("sh runs");
    assert!(
        out.status.success(),
        "stage_and_warm did not run: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let calls = std::fs::read_to_string(&log).unwrap_or_default();
    let tiers: std::collections::BTreeSet<String> = calls
        .lines()
        .filter(|l| l.contains("build"))
        .map(|l| if l.contains("--release") { "release" } else { "dev" }.to_string())
        .collect();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        tiers.len(),
        2,
        "both tiers are warmed on every staging, or the reading that follows \
         counts a runtime.c compile the other one does not -- which is the \
         start-up row's kanso#1461 bug, one binary reading 6,018,427 and then \
         4,869,632. The warm-up ran: {calls:?}"
    );
}

/// And the warm-up runs the measurement's own command in the measurement's own
/// environment.
///
/// Re-staging the box was half the fix and the sitting said so: the next run
/// still read `again_procs=5 first_procs=6`. The profiles named the missing
/// process. `cached_runtime_object` keys runtime.c's object on profile and
/// runtime hash and keeps it in `std::env::temp_dir()`, which is not in the
/// box and which reads TMPDIR -- so a warm-up under the job's environment
/// filled a different directory from the one the measurement reads, the first
/// measured run paid for runtime.c and the second found it cached.
#[test]
fn the_warm_up_runs_under_the_measurements_environment() {
    let s = gate();
    let body = s
        .split_once("stage_and_warm() {")
        .expect("the staging step is a shell function named stage_and_warm")
        .1
        .split_once("\n}\n")
        .expect("stage_and_warm closes on a line of its own")
        .0;
    let (first, _) = readings(&s);
    let measured = s[..first]
        .rsplit_once("env -i ")
        .map(|(_, r)| r)
        .unwrap_or_else(|| panic!("the first measured run does not empty its environment"));
    // The environment the measurement sets, up to the callgrind invocation.
    let wanted: Vec<&str> = measured.split_whitespace().take_while(|t| *t != "valgrind").collect();
    assert!(
        !wanted.is_empty(),
        "the measured run names no environment between `env -i` and valgrind"
    );
    assert!(
        body.contains("env -i "),
        "the warm-up runs under the job's environment while the measurement \
         runs under `env -i`, so the two fill different caches"
    );
    for t in wanted {
        assert!(
            body.contains(t),
            "the measurement sets {t} and the warm-up does not, so the warm-up \
             is not the run it is warming"
        );
    }
}
