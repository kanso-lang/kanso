//! A join a fiber reaches while it runs joins the scheduler that is already
//! running, so its members overlap the fibers beside them, and whatever is
//! bound after the join still waits for all of them (#2263). Before, such a
//! join started a second scheduler inside one step of the first, and every
//! other fiber waited until it finished; a server that forks per connection
//! is that shape, and it answered one request at a time.
//!
//! The scheduler orders by wake time and then by spawn order, so both
//! programs here have one transcript, and both engines must print it.
use std::path::PathBuf;
use std::process::Command;

fn played(name: &str, interp: bool) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/joins");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kanso"));
    cmd.arg("play").arg(root.join(format!("{name}.kso")));
    if interp {
        cmd.arg("--interp");
    }
    let done = cmd.output().expect("kanso runs");
    assert!(done.status.success(), "{name}: {}", String::from_utf8_lossy(&done.stderr));
    String::from_utf8(done.stdout).expect("utf-8 output")
}

fn golden(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/joins")
        .join(format!("{name}.stdout"));
    std::fs::read_to_string(path).expect("the golden reads")
}

#[test]
fn a_sleep_started_first_wakes_first_beside_a_join_a_fiber_reached() {
    let want = golden("staggered_sleeps");
    assert_eq!(played("staggered_sleeps", true), want, "interpreter");
    assert_eq!(played("staggered_sleeps", false), want, "native");
}

#[test]
fn what_follows_a_join_a_fiber_reached_waits_for_every_member() {
    let want = golden("what_follows_a_join_waits_for_it");
    assert_eq!(played("what_follows_a_join_waits_for_it", true), want, "interpreter");
    assert_eq!(played("what_follows_a_join_waits_for_it", false), want, "native");
}
