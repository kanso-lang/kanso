//! The counter sweep's lazy-tier step must run the ONE test that reads the
//! `.mem` vein, not the whole `golden` binary.
//!
//! `cargo test --test golden` builds ten tests. The sweep read the exit status
//! of all ten and reported any failure among them as `counters moved: mem`,
//! which names the one vein that had not moved and sends the reader to the
//! wrong file. Two of those ten are the micro corpus, which is also where the
//! time goes: the named test finishes in 3.35 seconds against 158 for the
//! binary, measured 2026-09-06 on this container.
//!
//! Which test is the right one is DERIVED here rather than written down.
//! `KANSO_REGEN_MEM_GOLDEN` is what regenerates the vein, so the test that
//! reads that variable is the test that owns it; if a second test ever starts
//! reading it, this goes red and the sweep needs a decision rather than a
//! quiet extra name.
use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    let path = manifest_dir().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} reads: {e}", path.display()))
}

/// Every `#[test]` in tests/golden.rs whose body mentions the regeneration
/// variable. A test's body runs from its `fn` line to the next `#[test]`.
fn tests_reading_the_regen_flag(golden: &str) -> Vec<String> {
    let mut out = Vec::new();
    for block in golden.split("#[test]").skip(1) {
        if !block.contains("KANSO_REGEN_MEM_GOLDEN") {
            continue;
        }
        let Some(at) = block.find("fn ") else { continue };
        let rest = &block[at + 3..];
        let Some(shut) = rest.find('(') else { continue };
        out.push(rest[..shut].trim().to_string());
    }
    out
}

#[test]
fn one_test_owns_the_mem_vein_and_the_sweep_names_it() {
    let owners = tests_reading_the_regen_flag(&read("tests/golden.rs"));
    assert_eq!(
        owners.len(),
        1,
        "exactly one test in tests/golden.rs may read KANSO_REGEN_MEM_GOLDEN; \
         found {owners:?}. If the vein has genuinely gained a second owner, the \
         sweep has to name both -- decide that here rather than letting \
         all_counters.sh fall back to the whole binary."
    );
    let owner = &owners[0];
    let sweep = read("scripts/gates/all_counters.sh");
    assert!(
        sweep.contains(&format!("mem_test={owner}")),
        "scripts/gates/all_counters.sh must run `{owner}` by name. Without the \
         name it reads the exit status of all ten tests in the golden binary \
         and reports any of them as `counters moved: mem`, which is a \
         diagnosis pointing at the vein that did not move."
    );
}

/// The name alone is not enough: both branches of the step have to carry it.
/// The read branch reporting the wrong vein costs a reader their time; the
/// `--write` branch reporting a failed regeneration for a micro-corpus
/// mismatch would send them to regenerate a vein that is already correct.
#[test]
fn both_branches_of_the_lazy_tier_step_run_the_named_test() {
    let sweep = read("scripts/gates/all_counters.sh");
    // Invocations, not prose: the step's own comment names `--test golden`
    // while explaining why the name is there, and counting it read as a third
    // branch the first time this ran.
    let runs: Vec<&str> = sweep
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.starts_with('#'))
        .filter(|l| l.contains("--test golden"))
        .collect();
    assert_eq!(
        runs.len(),
        2,
        "the lazy-tier step has a read branch and a --write branch, and both \
         invoke the golden binary. Found {runs:?}"
    );
    for line in &runs {
        assert!(
            line.contains("\"$mem_test\""),
            "`{line}` runs the whole golden binary. Both branches pass \
             \"$mem_test\" so a micro-corpus failure is never reported as a \
             .mem divergence."
        );
    }
}
