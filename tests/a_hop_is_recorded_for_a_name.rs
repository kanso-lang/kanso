//! An err passing into a NAMED function records a hop; into an anonymous one it
//! records none. Both engines agree on all three spellings.
//!
//! This is the ground the 2026-07-25 eta-reduction decline stood on, re-measured.
//! That entry declined `(a b -> f a b)` -> `f` because "an `err` records a hop
//! for every function it passes through, and the eta-expanded lambda is a
//! function": native printed `passed through greet` where the interpreter did
//! not, and the differential law does not permit that.
//!
//! The 2026-09-15 explicit-bind ruling moved the ground, in its own words: the
//! hop "now accrues at binds rather than at skipped calls". Measured on
//! 2026-09-17, three spellings of one call:
//!
//!     label "flan" seasonal                 passed through label
//!     lab "flan" seasonal, lab = eta of it  passed through lab
//!     (d c -> label d c) "flan" seasonal    no hop line at all
//!
//! **Both engines agree in every one**, so the divergence that killed the
//! optimization is gone. What stands in its place is different: eta-reducing
//! the third spelling to the first ADDS a line rather than dropping one, and
//! whether that is allowed is a question about what a hop means. The July entry
//! said that question "belongs to a gavel rather than to an optimization's side
//! effects", and it still does.
//!
//! The fixtures are here rather than in the commit message because a bug that
//! had no home in the corpus is a gap in the corpus.

use std::process::Command;

/// Run a fixture on one engine and answer the provenance lines it printed.
fn hops(fixture: &str, interpreted: bool) -> Vec<String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("tests/fixtures/provenance").join(format!("{fixture}.kso"));
    let mut run = Command::new(env!("CARGO_BIN_EXE_kanso"));
    run.arg("play").arg(&path);
    if interpreted {
        run.arg("--interp");
    }
    let done = run.output().expect("kanso runs");
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&done.stdout),
        String::from_utf8_lossy(&done.stderr),
    );
    assert!(said.contains("flan is seasonal"), "{fixture} did not reach the err at all:\n{said}",);
    said.lines()
        .map(str::trim)
        .filter(|l| l.starts_with("passed through "))
        .map(str::to_string)
        .collect()
}

/// The differential law, which is what the July decline turned on. Whatever the
/// hops are, the two engines say the same thing.
#[test]
fn both_engines_agree_on_every_spelling() {
    for fixture in ["hop_direct", "hop_named", "hop_lam"] {
        assert_eq!(
            hops(fixture, false),
            hops(fixture, true),
            "{fixture}: native and the oracle disagree about provenance",
        );
    }
}

/// A named function in the path is named in the trace.
#[test]
fn a_named_function_records_the_hop_it_takes() {
    assert_eq!(hops("hop_direct", false), ["passed through label"]);
    assert_eq!(hops("hop_named", false), ["passed through lab"]);
}

/// An anonymous one records nothing, which is the whole of what eta-reduction
/// would change. Without this the test above could pass on a compiler that
/// records a hop for everything.
#[test]
fn an_anonymous_function_records_no_hop() {
    let seen = hops("hop_lam", false);
    assert!(
        seen.is_empty(),
        "the lambda spelling recorded {seen:?}; if it names something now, the \
         eta-reduction question has moved again and design/compiler-log.md's \
         2026-09-17 entry is the one to correct",
    );
}
