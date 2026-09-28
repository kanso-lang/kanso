//! An accumulator whose seed the caller still reads is copied once, not once
//! a lap.
//!
//! `linear::in_place_pushes` proves a write may go in place when every caller
//! hands the accumulator a value nobody else holds. A loop seeded from a name
//! the caller prints afterwards fails that proof for every lap, not only the
//! first, so the interpreter copied the whole accumulator on each one: 40,000
//! pushes onto a shared seed took 8.4 seconds interpreted and 5 milliseconds
//! compiled. The compiled engine never had the problem, because its buffers
//! carry the length of their newest owner and a push checks that at run time.
//!
//! The interpreter's check is the holder count. `linear::moved_writes` proves
//! the weaker, local half: the write's argument is a name its function reads
//! once. That name's binding is one holder and the argument is another, so a
//! count of two at the write means nobody else holds the value. The first lap
//! sees three, because the caller's `seed` holds it too, and copies; every lap
//! after that sees two.
//!
//! Found by a generated program on 2026-09-28, which timed out interpreted
//! where both native builds answered at once.
//!
//! WHAT IS PINNED is the same kind of number as in
//! `a_unique_container_is_extended_in_place`: what 300 extra laps cost in
//! allocations, taken as a difference of two runs from entry files of the
//! same name length, so every fixed allocation cancels. Copying every lap
//! read 161,356 for the extra laps, and the count grew with the square of
//! the laps, because each copy is as long as the accumulator. Copying once
//! reads 3,647.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Three builders, one per writing builtin, each seeded from a name that
/// `run` reads again after the loop. The seeds must print unchanged: that is
/// the answer a write through a shared value would get wrong.
const LIBRARY: &str = r#"import "std/text"

fn grow acc 0
  acc

fn grow acc n
  grow (push acc n) (n - 1)

fn file m 0
  m

fn file m n
  file (put m "k{n}" n) (n - 1)

fn spell b 0
  b

fn spell b n
  spell (text/append b "ab") (n - 1)

pub fn run rounds
  seed = [0]
  book = { "k0":0 }
  page = text/bytes "x"
  grown = length (grow seed rounds)
  filed = length (file book rounds)
  spelt = length (spell page rounds)
  print "{grown} {filed} {spelt} {seed} {book} {length page}"
"#;

/// What 300 extra laps of the three builders cost, fixed allocations
/// cancelled by the subtraction.
const PER_EXTRA_ROUND: u64 = 3_647;

fn kanso() -> PathBuf {
    let mut exe = std::env::current_exe().expect("the test binary has a path");
    exe.pop();
    if exe.ends_with("deps") {
        exe.pop();
    }
    exe.join("kanso")
}

/// Each test stages its own tree, for the reason
/// `a_unique_container_is_extended_in_place` gives at length: cargo runs a
/// binary's tests on parallel threads, and a library one thread is writing is
/// empty for the other. The tags are the same length so the paths cost the
/// same.
fn staged(tag: &str) -> PathBuf {
    let stage = std::env::temp_dir().join(format!("kanso-shared-seed-{tag}"));
    std::fs::create_dir_all(&stage).expect("a staging directory");
    std::fs::write(stage.join("builders.kso"), LIBRARY).expect("the library writes");
    stage
}

fn ran(stage: &Path, rounds: u32) -> (String, u64) {
    let entry = stage.join(format!("run_{rounds}.kso"));
    std::fs::write(&entry, format!("import \"./builders\"\n\nbuilders/run {rounds}\n"))
        .expect("the entry writes");
    let out = Command::new(kanso())
        .arg("run")
        .arg(&entry)
        .arg("--interp")
        .env("KANSO_COUNTERS", "1")
        .output()
        .expect("kanso runs");
    assert!(
        out.status.success(),
        "the interpreted run failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let allocs = String::from_utf8_lossy(&out.stderr)
        .lines()
        .find_map(|line| line.strip_prefix("interp_allocs=")?.trim().parse().ok())
        .expect("the run counts interp_allocs");
    (String::from_utf8_lossy(&out.stdout).to_string(), allocs)
}

#[test]
fn the_seeds_print_unchanged() {
    let stage = staged("answer");
    assert_eq!(ran(&stage, 300).0, "301 301 601 [0] { \"k0\":0 } 1\n");
    assert_eq!(ran(&stage, 600).0, "601 601 1201 [0] { \"k0\":0 } 1\n");
}

#[test]
fn a_shared_seed_is_copied_on_the_first_lap_only() {
    let stage = staged("allocs");
    let small = ran(&stage, 300).1;
    let large = ran(&stage, 600).1;
    assert_eq!(
        large - small,
        PER_EXTRA_ROUND,
        "300 more laps of three builders whose seeds the caller still reads \
         cost {} allocations; copying the accumulator once and extending it in \
         place after that costs {PER_EXTRA_ROUND}. Copying it on every lap read \
         161,356. Read {small} at 300 laps and {large} at 600.",
        large - small
    );
}
