//! A loop threading a map must not hold more arena the longer it runs.
//!
//! The live set is one map with one key whatever the iteration count, and the
//! string bound each time round is dead before the next one begins. So peak
//! arena is a property of the program rather than of `n`, and asserting the
//! ratio between two sizes says that where a single number would only say what
//! today's constant happens to be.
//!
//! tests/golden/mem/an_accumulator_loop_keeps_its_garbage.mem covers the same
//! program and cannot see this: at twenty thousand iterations the loop fits in
//! one arena block either way, and one block is the floor. The gap only opens
//! at a size that golden does not reach, which is the same lesson the welfare
//! basket taught one corpus over.

use std::process::Command;

/// Peak arena bytes for a map-threading loop of `n` iterations.
fn peak_bytes(n: u64) -> u64 {
    let dir = std::env::temp_dir().join(format!("kanso-map-peak-{n}"));
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    // `junk` is bound, measured and dropped: it is the per-iteration garbage
    // the rewind exists to reclaim. The map holds one key throughout.
    let program = format!(
        "fn go 0 m\n  print (length (entries m))\n\n\
         fn go n m\n  junk = \"k{{n % 17}}\"\n  \
         go (n - 1) (put m \"k1\" (m[\"k1\"]! + length junk))\n\n\
         pub play = go {n} {{ \"k1\":0 }}\n"
    );
    std::fs::write(dir.join("run.kso"), program).expect("the program writes");

    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg("run.kso")
        .env("KANSO_COUNTERS", "1")
        .current_dir(&dir)
        .output()
        .expect("kanso runs");
    let said = String::from_utf8_lossy(&done.stderr).into_owned();
    let _ = std::fs::remove_dir_all(&dir);

    said.lines()
        .find_map(|l| l.trim().strip_prefix("arena_peak_bytes=")?.parse().ok())
        .unwrap_or_else(|| panic!("no arena_peak_bytes in:\n{said}"))
}

/// Ten times the iterations, the same live set, so the same peak. Before the
/// map accumulator's pairs moved out of the arena this read 1,048,576 against
/// 9,437,184 — nine blocks held for a map with one key in it.
#[test]
fn a_map_threading_loop_holds_no_more_arena_the_longer_it_runs() {
    let short = peak_bytes(20_000);
    let long = peak_bytes(200_000);

    assert!(
        long <= short * 2,
        "ten times the iterations cost more than twice the arena: {short} bytes \
         at 20,000 against {long} at 200,000, a factor of {:.1}",
        long as f64 / short as f64
    );
}
