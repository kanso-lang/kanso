//! How many times the front end rewrites the whole program.
//!
//! bench/compile_golden_modules.txt runs this exact sample through
//! `compile_entry`, so the entry path is already in a golden's workload. What
//! that golden counts is rounds and visits — the inference fixpoint's, which a
//! rewrite pass does not touch — and the emitted text, which is byte-identical
//! whether a pass runs once or twice, because the passes are idempotent on
//! their own output. `compile_parsed_entry` ran four of them twice for as long
//! as anyone had looked and every gate in the repository stayed green.
//!
//! This is `tests/inference_passes.rs` pointed at the other half of the front
//! end, for the same reason and with the same discipline: a watched trend, not
//! a floor. A pass added for a good reason is fine; saying which one and why
//! in design/compiler-log.md is the price of moving the number.

/// The same committed directory the compile golden measures, so the module
/// loader is on the path being counted and the sample cannot go missing.
fn passes_for_the_sample() -> u64 {
    let entry = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/compile/module/main.kso");
    let source = std::fs::read_to_string(&entry).expect("the entry reads");
    kanso::rewrite::reset();
    let _ = kanso::compile_entry(&entry.to_string_lossy(), &source).expect("the sample compiles");
    kanso::rewrite::passes()
}

#[test]
fn the_front_end_rewrites_the_whole_program_twenty_times() {
    assert_eq!(
        passes_for_the_sample(),
        20,
        "the whole-program rewrite pass count moved; if that is intended, \
         name the pass and the reason in design/compiler-log.md and update \
         this number"
    );
}
