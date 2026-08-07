//! A map's cached sorted view is malloc'd, and nothing gives it back.
//!
//! The view is built the first time a map is read — `entries` needs the sorted
//! deduped pairs, `length` needs the deduped count — and cached on the map so
//! the second read is free. Its storage is malloc'd deliberately, so that a
//! below-mark map header can never hold a pointer a rewind would free.
//!
//! The price of that choice is ownership: the arena has no notion of an object
//! dying, so when the map becomes garbage its view is simply orphaned. Carried
//! in the source as "a transient map's view leaks with the map", which reads
//! like a rounding error and is not one. On a loop that builds one map per
//! iteration it was measured at 76,800,048 bytes over 1.6 million iterations —
//! forty-eight bytes each, unbounded in the iteration count.
//!
//! No counter could see it: the arena counters measure one allocator and the
//! view is not in it, so `arena_peak_bytes` reads flat while the process grows.
//! `view_allocs` and `view_frees` exist so the gap between them is a diffed
//! fact, and this asserts the gap stays closed.

use std::process::Command;

/// Views allocated and views returned, for a program that builds `n` transient
/// maps and reads each one.
fn views(n: u64) -> (u64, u64) {
    let dir = std::env::temp_dir().join(format!("kanso-view-{n}"));
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    // A map literal's keys must be literals, so a transient map is built with
    // `put`. Reading its length is what forces the view.
    let program = format!(
        "fn go 0 seen\n  seen\n\n\
         fn go n seen\n  fresh = put {{}} \"k\" n\n  \
         go (n - 1) (seen + length fresh)\n\n\
         print \"{{go {n} 0}}\"\n"
    );
    std::fs::write(dir.join("run.kso"), program).expect("the program writes");

    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("play")
        .arg("run.kso")
        .env("KANSO_COUNTERS", "1")
        .current_dir(&dir)
        .output()
        .expect("kanso runs");
    let said = String::from_utf8_lossy(&done.stderr).into_owned();
    let _ = std::fs::remove_dir_all(&dir);

    let read = |key: &str| -> u64 {
        said.lines()
            .find_map(|l| l.trim().strip_prefix(key)?.strip_prefix('=')?.trim().parse().ok())
            .unwrap_or_else(|| panic!("no {key} in:\n{said}"))
    };
    (read("view_allocs"), read("view_frees"))
}

/// Every view a program builds is one it must give back. A handful may still be
/// live at exit — the maps still reachable from the answer — so the assertion
/// is a small constant, not equality.
///
/// Ignored because it fails: it is the acceptance criterion for an ownership
/// rule that does not exist yet, not a regression guard for one that does. Run
/// it with `cargo test -- --ignored` to see the current gap, and delete this
/// attribute in the change that gives the view an owner.
#[test]
#[ignore = "the view has no owner yet; see design/compiler-log.md"]
fn a_transient_maps_view_goes_back_to_the_allocator() {
    let (built, returned) = views(20_000);

    assert!(
        built.saturating_sub(returned) < 64,
        "views are being orphaned: {built} built, {returned} returned, \
         {} never given back",
        built - returned
    );
}

/// The shape rather than the constant: ten times the transient maps must not
/// leave ten times the views outstanding. This is the one that catches a leak
/// growing with the workload, which is how the 76.8 MB arose.
#[test]
#[ignore = "the view has no owner yet; see design/compiler-log.md"]
fn what_is_never_returned_does_not_grow_with_the_loop() {
    let (small_built, small_back) = views(2_000);
    let (large_built, large_back) = views(20_000);
    let small = small_built - small_back;
    let large = large_built - large_back;

    assert!(
        large <= small * 2,
        "the orphaned views grow with the iteration count: {small} outstanding \
         at 2,000 maps against {large} at 20,000"
    );
}
