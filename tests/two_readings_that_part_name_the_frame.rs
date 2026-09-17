//! When one binary counts two numbers, the diff says which frame moved.
//!
//! The three compile gates read a single figure out of a callgrind profile:
//! `kanso::main` inclusive. When a second reading of the same binary on the
//! same machine disagrees with the first, that figure says how much and
//! nothing at all about where, and every hunt through 2026-09-16 had to guess
//! from the size of the move. On kanso#1464 all three rows read exactly 13
//! below goldens taken from an earlier run -- one constant across three
//! unrelated compiles, which is a fact about a per-process term rather than
//! about the compiler, and there was no instrument that could say which term.
//!
//! `scripts/gates/profile_diff.sh` is that instrument, and this is the spec
//! that keeps it honest. Two profiles that differ in one function by thirteen
//! instructions: the diff must NAME that function and report the net. The
//! thirteen is the size the hunt actually has to see, and the trap the spec
//! exists for is a thresholded read -- `callgrind_annotate` hides everything
//! under its default threshold, which is every frame this instrument is for.
//! A diff written that way passes a test built from a hot function and says
//! "the two profiles are identical" on the real one.

use std::process::Command;

/// The smallest thing callgrind_annotate will read: one file, two functions,
/// a total.
fn profile(beta: u32) -> String {
    format!(
        "version: 1\ncreator: callgrind-3.22.0\ncmd: ./probe\npart: 1\n\n\
         positions: line\nevents: Ir\n\n\
         fl=probe.c\nfn=alpha\n1 100000000\n\nfn=beta\n2 {beta}\n\ntotals: {}\n",
        100_000_000 + beta
    )
}

#[test]
fn a_thirteen_instruction_move_is_named() {
    if Command::new("callgrind_annotate").arg("--version").output().is_err() {
        // The gates that call this script already refuse without the tool, and
        // say so; there is nothing for the spec to add on a host without it.
        return;
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let stage = std::env::temp_dir().join("kanso-two-readings-part");
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).expect("a staging directory");
    let first = stage.join("cg.first");
    let second = stage.join("cg.second");
    // `beta` is a hair over one ten-millionth of the profile, which is where
    // the real move sits: 13 in 132 million. A threshold that keeps the
    // default 99% loses it, and that is the failure this spec catches.
    std::fs::write(&first, profile(1_000)).expect("the first profile");
    std::fs::write(&second, profile(1_013)).expect("the second profile");

    let out = Command::new("sh")
        .arg(root.join("scripts/gates/profile_diff.sh"))
        .arg(&first)
        .arg(&second)
        .output()
        .expect("profile_diff runs");
    let said = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(out.status.success(), "profile_diff exited {:?}: {said}", out.status.code());
    assert!(
        said.contains("probe.c:beta"),
        "the diff must name the frame that moved, and said:\n{said}"
    );
    assert!(
        said.contains("13 instructions net"),
        "the diff must report the net move, and said:\n{said}"
    );
    assert!(
        !said.contains("probe.c:alpha"),
        "the diff must leave the frame that held still out of it, and said:\n{said}"
    );
}

#[test]
fn two_readings_that_agree_say_so() {
    if Command::new("callgrind_annotate").arg("--version").output().is_err() {
        return;
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let stage = std::env::temp_dir().join("kanso-two-readings-agree");
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).expect("a staging directory");
    let only = stage.join("cg.only");
    std::fs::write(&only, profile(1_000)).expect("the profile");

    let out = Command::new("sh")
        .arg(root.join("scripts/gates/profile_diff.sh"))
        .arg(&only)
        .arg(&only)
        .output()
        .expect("profile_diff runs");
    let said = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(out.status.success(), "profile_diff exited {:?}: {said}", out.status.code());
    assert!(
        said.contains("every function agrees"),
        "two identical profiles must read as identical, and said:\n{said}"
    );
}
