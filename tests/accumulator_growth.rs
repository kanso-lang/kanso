//! Peak memory is the only honest measure of a rewind, because every counter
//! this could assert instead has been green while memory was corrupt. Three
//! bugs in this seam passed their counters: a bind chain rewinding an arena a
//! live frame pointed into, a record held across an io walk, and a thunk
//! memoising a result the loop's rewind freed.
//!
//! So the claim here is the one a person would make: a loop whose live set is
//! one map with one key must not need more memory to run four times as long.

use std::process::Command;

fn peak_at(iterations: u64) -> (u64, u64) {
    let dir = std::env::temp_dir().join(format!("kanso-acc-{iterations}"));
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    let program = dir.join("accumulate.kso");
    std::fs::write(
        &program,
        format!(
            "fn go 0 m\n  print (length (entries m))\n\n\
             fn go n m\n  junk = \"k{{n % 17}}\"\n  \
             go (n - 1) (put m \"k1\" (m[\"k1\"]! + length junk))\n\n\
             go {iterations} {{ \"k1\":0 }}\n"
        ),
    )
    .expect("the program writes");

    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("play")
        .arg("accumulate.kso")
        .env("KANSO_COUNTERS", "1")
        .current_dir(&dir)
        .output()
        .expect("kanso runs");
    let said = String::from_utf8_lossy(&done.stderr).into_owned();
    let _ = std::fs::remove_dir_all(&dir);

    let read = |key: &str| -> u64 {
        said.lines()
            .find_map(|l| l.trim().strip_prefix(key)?.strip_prefix('='))
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or_else(|| panic!("no `{key}` in counters:\n{said}"))
    };
    (read("arena_peak_bytes"), read("allocs"))
}

/// The live set does not grow, so neither should the arena.
///
/// Written to fail, and ignored, as the acceptance criterion for a rewind that
/// did not exist. It exists: this loop is self-recursive and threads a map,
/// which `is_scalar_map_chain` licenses, so the group brackets and rewinds
/// every iteration. The arena peak holds at 1,048,576 bytes from five thousand
/// iterations to 1,280,000 — flat across 256x — with `beat_iters` tracking the
/// iteration count all the way. A regression guard now.
#[test]
fn a_longer_loop_over_the_same_live_set_does_not_need_more_memory() {
    let (small_peak, small_allocs) = peak_at(5_000);
    let (large_peak, large_allocs) = peak_at(80_000);

    assert_eq!(
        large_peak, small_peak,
        "sixteen times the iterations took more arena: {small_peak} -> {large_peak} bytes \
         (allocations {small_allocs} -> {large_allocs}), for a live set of one map with one key"
    );
}
