//! A program that sequences effects one after another holds one frame, not one
//! per effect. The interpreter is the oracle, so a shape native runs in flat
//! memory must not cost the interpreter memory proportional to its length.
//!
//! Peak memory is the claim rather than finishing, because a chain that nests
//! still finishes — it just costs a few kilobytes a link until the stack gives
//! out. Only the footprint separates a loop from a recursion.
//!
//! The number comes from the kernel, which accounts every child's high-water
//! mark and hands it back at wait. time(1) used to supply it, and on a host
//! without /usr/bin/time — a minimal container is one — the missing stopwatch
//! was reported as the interpreter failing to run.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// `ru_maxrss` counts kilobytes on linux and bytes on the BSDs, and the same
/// claim has to hold on either host.
#[cfg(target_vendor = "apple")]
const RESIDENT_UNIT: u64 = 1;
#[cfg(not(target_vendor = "apple"))]
const RESIDENT_UNIT: u64 = 1024;

/// Peak resident bytes for one run of a program that prints `done`.
// wait4 below is the reaping call, and it is the one that also answers with
// the rusage this test exists to read. clippy watches for `Child::wait`.
#[allow(clippy::zombie_processes)]
fn peak_for_program(dir: &Path) -> u64 {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["play", "shape.kso", "--interp"])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .spawn()
        .expect("kanso runs");

    // Drained before the wait: a child whose pipe fills stops, and a wait on a
    // stopped child never returns.
    let mut said = String::new();
    child
        .stdout
        .take()
        .expect("the run's stdout is piped")
        .read_to_string(&mut said)
        .expect("the run's stdout reads");

    // wait4 reaps the child here, so nothing else may wait on it afterwards.
    let pid = child.id() as libc::pid_t;
    let mut status = 0;
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    let reaped = unsafe { libc::wait4(pid, &mut status, 0, &mut usage) };
    assert_eq!(reaped, pid, "the child is this test's to reap");
    assert!(said.contains("done"), "the program did not finish: {said:?}");

    usage.ru_maxrss as u64 * RESIDENT_UNIT
}

/// One written program in a directory named for its shape and its length, so
/// two shapes at the same length never share a path.
fn program_in(shape: &str, links: u64, source: String) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kanso-{shape}-{links}"));
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    std::fs::write(dir.join("shape.kso"), source).expect("the program writes");
    dir
}

/// The chain is right-nested through `.`, which is the shape a loop over work
/// takes: each step hands its result to a closure that decides the next step.
fn peak_for_chain(links: u64) -> u64 {
    let dir = program_in(
        "chain",
        links,
        format!(
            "import \"std/io\"\n\n\
             step 1\n\n\
             fn step n\n  going n (n > {links})\n\n\
             fn going _ true\n  io/write \"done\\n\"\n\n\
             fn going n false\n  io/write \"\" . (_ -> step (n + 1))\n"
        ),
    );
    let peak = peak_for_program(&dir);
    let _ = std::fs::remove_dir_all(&dir);
    peak
}

/// The same count of steps as a recursion that has work left to do when the
/// call below it comes back. Every link holds a frame until the bottom is
/// reached, which is the shape the chain must not be.
fn peak_for_nesting(links: u64) -> u64 {
    let dir = program_in(
        "nesting",
        links,
        format!(
            "import \"std/io\"\n\n\
             io/write \"done {{down {links}}}\\n\"\n\n\
             fn bump x\n  x + 1\n\n\
             fn down 0\n  0\n\n\
             fn down n\n  bump (down (n - 1))\n"
        ),
    );
    let peak = peak_for_program(&dir);
    let _ = std::fs::remove_dir_all(&dir);
    peak
}

/// Ten times the work over the same live set, so a chain that runs as a loop
/// stays flat while one that nests grows with its length.
#[test]
fn a_longer_chain_of_effects_does_not_need_more_memory() {
    let short = peak_for_chain(1_000);
    let long = peak_for_chain(10_000);

    assert!(
        long < short * 2,
        "a chain ten times longer took {long} bytes against {short} — it is nesting, not looping"
    );
}

/// The claim next door is worth only as much as the instrument that made it,
/// so the same instrument reads a shape that does nest, over the same ten-fold
/// ratio and against the same threshold. It answers in the other direction. A
/// nesting run cannot be asked for ten thousand links the way the chain is —
/// four thousand already costs four times what a chain of a million does, and
/// ten thousand runs out of stack.
#[test]
fn the_measurement_sees_a_shape_that_nests() {
    let short = peak_for_nesting(400);
    let long = peak_for_nesting(4_000);

    assert!(
        long > short * 2,
        "a nesting shape ten times deeper took {long} bytes against {short} — this test cannot see depth, so the flat reading beside it means nothing"
    );
}
