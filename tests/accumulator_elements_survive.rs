//! The boundary a rewind license must not cross.
//!
//! A loop threading a heap accumulator emits no beat bracket today, so nothing
//! is reclaimed and the program is correct but wasteful — that is what
//! tests/accumulator_growth.rs measures. The fix is a license admitting the
//! cluster, and the danger is admitting one whose elements point into the
//! region the rewind frees.
//!
//! `cluster_edges_ok` names the invariant its bytes license depends on:
//! "header below the entry mark, growth outside the arena, no pointers
//! inside". A map's values are KValues, and a value built this iteration
//! lives in the arena above the mark. Rewinding frees it while the map still
//! points at it.
//!
//! So this holds a map whose values are strings built per iteration and reads
//! them back at the end. It passes today because the cluster is refused. It
//! fails the moment a widening admits it without keeping the values alive —
//! which is the failure #57, #92 and #95 all were, and which every counter
//! stayed green through.

use std::process::Command;

fn ran(program: &str, name: &str) -> (String, String, Option<i32>) {
    let dir = std::env::temp_dir().join(name);
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    let file = dir.join("run.kso");
    std::fs::write(&file, program).expect("the program writes");
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("play")
        .arg("run.kso")
        .current_dir(&dir)
        .output()
        .expect("kanso runs");
    let _ = std::fs::remove_dir_all(&dir);
    (
        String::from_utf8_lossy(&done.stdout).into_owned(),
        String::from_utf8_lossy(&done.stderr).into_owned(),
        done.status.code(),
    )
}

/// The value stored each iteration is a fresh string, so it is arena memory
/// born above any rewind mark. Reading it back after the loop is what a
/// premature rewind corrupts.
///
/// The loop counts DOWN, so the surviving value for each key is the one
/// written at the smallest matching n: k0 at n=3, k1 at n=1, k2 at n=2, and
/// `"v{n % 7}"` makes those v3, v1 and v2.
#[test]
fn a_map_value_built_each_iteration_is_still_readable_after_the_loop() {
    let program = "fn go 0 m\n  m\n\n\
                   fn go n m\n  fresh = \"v{n % 7}\"\n  \
                   go (n - 1) (put m \"k{n % 3}\" fresh)\n\n\
                   built = go 6000 { \"k0\":\"\" }\n\n\
                   print \"{built[\"k0\"]!} {built[\"k1\"]!} {built[\"k2\"]!}\"\n";

    let (out, err, code) = ran(program, "kanso-acc-elements");

    assert_eq!(code, Some(0), "the program failed: {err}");
    assert_eq!(
        out, "v3 v1 v2\n",
        "a map value built during the loop did not survive it — a rewind freed \
         memory the accumulator still points at:\n{err}"
    );
}

/// The same shape one container over: a list whose elements are built per
/// iteration, read back after the loop. Pushes happen from n=4000 down, so
/// element 1 is n=4000 ("e0") and element 2000 is n=2001 ("e1").
#[test]
fn a_list_element_built_each_iteration_is_still_readable_after_the_loop() {
    let program = "fn go 0 xs\n  xs\n\n\
                   fn go n xs\n  fresh = \"e{n % 5}\"\n  \
                   go (n - 1) (push xs fresh)\n\n\
                   built = go 4000 []\n\n\
                   print \"{built[1]!} {built[2000]!} {length built}\"\n";

    let (out, err, code) = ran(program, "kanso-acc-list-elements");

    assert_eq!(code, Some(0), "the program failed: {err}");
    assert_eq!(
        out, "e0 e1 4000\n",
        "a list element built during the loop did not survive it — a rewind \
         freed memory the accumulator still points at:\n{err}"
    );
}
