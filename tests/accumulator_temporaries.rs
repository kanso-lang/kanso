//! A growing accumulator must cost what it holds, and nothing else.
//!
//! tests/accumulator_growth.rs already pins the fixed-size case: a loop whose
//! live set is one map with one key must not need more memory to run longer.
//! A list being built is different — it genuinely holds n elements, so its
//! peak must grow with n and asserting otherwise would be asserting a lie.
//!
//! What must not grow is everything else. A loop that builds a list while
//! making and dropping a temporary each iteration has the same live set as one
//! that builds the same list without the temporary: the temporary is dead the
//! moment the iteration ends. Today both loops keep it, because a loop
//! carrying a heap accumulator emits no beat bracket at all and so has no
//! rewind point — measured at 71.9 MB against 1.5 MB over 1.6M iterations for
//! the threading case.
//!
//! So the claim is a ratio between two programs rather than a number: the loop
//! that discards a temporary may not cost materially more than the loop that
//! never made one. That is falsifiable, it is the observable end rather than a
//! verdict inside the compiler, and no counter asserts it — three bugs in this
//! seam shipped with their counters green.

use std::process::Command;

/// Peak arena bytes for a loop building `n` elements, with or without a
/// per-iteration temporary that is used and then dropped.
fn peak_bytes(n: u64, temporary: bool) -> u64 {
    let name = if temporary { "temp" } else { "bare" };
    let dir = std::env::temp_dir().join(format!("kanso-acc-temp-{name}-{n}"));
    std::fs::create_dir_all(&dir).expect("a directory to run in");

    // Both loops push exactly the same values, so the list they build is
    // byte-identical. The only difference is that one names a string first,
    // measures it, and lets it go.
    let body = if temporary {
        "fn go 0 xs\n  xs\n\nfn go n xs\n  junk = \"t{n % 13}\"\n  \
         go (n - 1) (push xs (length junk))\n\n"
    } else {
        "fn go 0 xs\n  xs\n\nfn go n xs\n  go (n - 1) (push xs 2)\n\n"
    };
    let program = format!("{body}pub play = print (length (go {n} []))\n");
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

/// The temporary is dead at the end of its iteration, so the two loops have
/// the same live set and the one that makes it should not pay for keeping it.
/// A quarter is slack for block granularity — the arena rounds up to whole
/// blocks, so the two can differ by one without anything being retained. Every
/// temporary is kept today: 12.58 MB against 19.92 MB at 200k, which is the
/// 200,000 dead strings themselves, and the arena grows from four blocks to
/// eleven to hold them.
///
#[test]
fn a_discarded_temporary_does_not_ride_along_with_the_accumulator() {
    let bare = peak_bytes(200_000, false);
    let temp = peak_bytes(200_000, true);

    assert!(
        temp * 4 <= bare * 5,
        "a temporary that is dropped each iteration is being retained: the \
         loop that discards one peaks at {temp} bytes against {bare} for the \
         same list built without it, a factor of {:.1}",
        temp as f64 / bare as f64
    );
}

/// The accumulator's own growth is legitimate, so this pins the shape rather
/// than a constant: ten times the elements may cost about ten times the
/// memory, and the sibling above catches what rides along with them.
#[test]
fn the_accumulator_costs_what_it_holds() {
    let small = peak_bytes(20_000, true);
    let large = peak_bytes(200_000, true);

    assert!(
        large <= small * 20,
        "ten times the elements cost more than twenty times the memory: \
         {small} bytes at 20k against {large} at 200k, a factor of {:.1}",
        large as f64 / small as f64
    );
}
