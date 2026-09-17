//! The cached innermost beat mark agrees with the depth it is cached from.
//!
//! `k_beat_iter` used to derive `&k_beat_stack[k_beat_depth - 1]` on every
//! iteration -- a load, a decrement, a range test and three address
//! instructions to arrive at a value that cannot change for the life of the
//! loop. `k_beat_top` holds it instead, and every write to the depth goes
//! through `k_beat_set_depth` so the two stay together.
//!
//! That is an invariant a future edit can break silently, and breaking it is
//! not a crash: a stale top rewinds the arena to an OUTER loop's mark, which
//! frees memory the inner loop is still reading, and what a reader sees is a
//! wrong answer or nothing at all. So the counting build asks, at every
//! iteration, whether the cached pointer is the one the depth names, and dies
//! by name when it is not.
//!
//! This spec runs a program whose beats nest three deep under `--counters`,
//! which is what makes the question get asked. The accumulators are SCALARS on
//! purpose, and the laps ALLOCATE on purpose. Both are needed. An accumulator
//! rebuilt each lap compiles to a CARRY beat, which computes its own mark and
//! never reads the cache: a first draft built strings, emitted
//! `k_beat_iter_carry` four times and no `k_beat_iter` at all, and passed with
//! the maintenance removed. A loop that carries a scalar and allocates nothing
//! emits no beat at all, because a beat exists to reclaim the arena. So each
//! lap builds a padded string, keeps only its length, and drops the string.
//!
//! Watched red by dropping the `k_beat_top` maintenance from `k_beat_pop`: the
//! program died with "the cached beat top and the beat depth disagree"
//! instead of printing its line.
//!
//! The shipped binary compiles the check out, so nothing here is a cost the
//! gates or a user pays.

use std::process::Command;

const BEATS_THREE_DEEP: &str = r#"import "std/io"

pad = "0123456789012345678901234567890123456789012345678901234567890123"

fn innermost acc 0
  acc

fn innermost acc k
  s = "{pad}-{k}-{pad}"
  innermost (acc + length s) (k - 1)

fn middle acc 0
  acc

fn middle acc j
  got = innermost 0 6
  middle (acc + got) (j - 1)

fn outer acc 0
  acc

fn outer acc i
  got = middle 0 5
  outer (acc + got) (i - 1)

pub play =
  total = outer 0 40
  io/write "{total}\n"
"#;

const ENTRY: &str = r#"import "./lib"

lib/play
"#;

fn stage(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    std::fs::write(dir.join("lib.kso"), BEATS_THREE_DEEP).expect("the library writes");
    std::fs::write(dir.join("main.kso"), ENTRY).expect("the entry writes");
    dir
}

#[test]
fn the_counting_build_finds_the_cached_top_where_the_depth_says() {
    let dir = stage("kanso-beat-top-counting");

    // --counters is what turns the check on. A build under an inherited
    // KANSO_COUNTERS would keep the gates whatever this arm asked for, so the
    // variable is removed rather than trusted.
    let built = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("build")
        .arg(&dir)
        .arg("--release")
        .arg("--counters")
        .env_remove("KANSO_COUNTERS")
        .current_dir(&dir)
        .output()
        .expect("kanso binary runs");
    assert!(
        built.status.success(),
        "the counting build failed: {}",
        String::from_utf8_lossy(&built.stderr)
    );

    // `build` names the binary for the program, which is the directory.
    let name = dir.file_name().expect("the stage has a name").to_owned();
    let out =
        Command::new(dir.join(name)).current_dir(&dir).output().expect("the binary runs");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        !err.contains("the cached beat top and the beat depth disagree"),
        "a beat iteration read a cached mark the depth does not name: {err}"
    );

    // Each lap builds a 64-64 padded string and keeps only its length, so the
    // arena grows and the carried value is a scalar -- which is what a PLAIN
    // beat is. 1,200 innermost laps at 130 or 131 bytes: 157,200. Pinned.
    assert_eq!(String::from_utf8_lossy(&out.stdout), "157200\n", "{err}");
}

#[test]
fn the_oracle_reads_the_same_program_the_same_way() {
    let dir = stage("kanso-beat-top-oracle");
    let oracle = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(&dir)
        .arg("--interp")
        .output()
        .expect("kanso binary runs");
    assert_eq!(
        String::from_utf8_lossy(&oracle.stdout),
        "157200\n",
        "{}",
        String::from_utf8_lossy(&oracle.stderr)
    );
}
