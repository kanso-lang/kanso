//! How many times the front end infers the whole program.
//!
//! bench/compile_golden.txt counts what one inference pass costs — rounds and
//! visits — by resetting the counter and running exactly one pass of its own.
//! That leaves it blind to how many passes the front end asks for, so a new
//! diagnostic that calls `infer` for itself raises the real cost without
//! moving either number. One did, and every gate in the repository stayed
//! green.
//!
//! Pinned here as a watched trend, not a floor. A pass added for a good
//! reason is fine; saying which one and why in design/compiler-log.md is the
//! price of moving the number.

/// The same committed directory the compile golden measures, so the module
/// loader is on the path being counted and the sample cannot go missing —
/// bench/jsonbench is generated rather than checked in, and a test that
/// reaches for it passes on the machine that just built it and nowhere else.
fn passes_for_the_sample() -> u64 {
    let entry = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/compile/module/main.kso");
    let source = std::fs::read_to_string(&entry).expect("the entry reads");
    kanso::infer::work::reset();
    let _ = kanso::compile_entry(&entry.to_string_lossy(), &source).expect("the sample compiles");
    kanso::infer::work::passes()
}

#[test]
fn the_front_end_infers_the_whole_program_four_times() {
    assert_eq!(
        passes_for_the_sample(),
        4,
        "the whole-program inference pass count moved; if that is intended, \
         name the pass and the reason in design/compiler-log.md and update \
         this number"
    );
}
