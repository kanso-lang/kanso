//! The run program's peak and its work are both terms in the objective.
//!
//! On 2026-08-31 a change took an 8 KB digest from 79,691,776 arena bytes to
//! 1,048,576 and scored exactly zero against welfare, because no counter in
//! the model measured whether a peak grows with the input. The same change
//! was 52x slower and that scored zero too. Both halves went into the model,
//! and these are the two fixtures that go red if either leaves it.
//!
//! Pricing only the peak would be worse than pricing neither: it would rank
//! any change that reclaims per block above one that does not, however long
//! it takes, which is exactly the trade the 52x slowdown was.
//!
//! The rows moved on 2026-09-06. Clay's gavel put the objective's run side on
//! one consolidated program, and the digest is a phase inside it carrying 5.05%
//! of the work rather than a term of its own — so `digest_peak_bytes` and
//! `digest_instructions` are diagnostics now and doctoring them scores nothing.
//! The property is the same one and it is asserted where the objective reads:
//! `bench/cost_golden_run.txt` for the peak, `bench/instructions_golden.txt`'s
//! runbench row for the work. digestbench keeps its own golden and its own gate,
//! so a digest regression still turns CI red; what this file watches is whether
//! the INDEX can see both dimensions of one workload.

use std::process::Command;

/// Run welfare against a staged `bench/` with one row of one golden replaced.
/// Answers the exit status and what it printed.
///
/// The staging directory is keyed per call — six tests in this repository
/// have staged into one path and torn each other down mid-run.
fn scored(key: &str, golden: &str, row: &str, replacement: &str) -> (bool, String) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let stage = std::env::temp_dir().join(format!("kanso-digest-priced-{key}"));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(stage.join("bench")).expect("a staging directory");
    for entry in std::fs::read_dir(root.join("bench")).expect("bench is readable") {
        let path = entry.expect("directory entry").path();
        if path.is_file() {
            let landing = stage.join("bench").join(path.file_name().expect("named"));
            std::fs::copy(&path, &landing).expect("the golden copies");
        }
    }

    if !row.is_empty() {
        let held = stage.join(golden);
        let text = std::fs::read_to_string(&held).expect("the golden reads");
        let mut out = String::new();
        let mut hit = false;
        for line in text.lines() {
            if line.starts_with(row) {
                out.push_str(replacement);
                hit = true;
            } else {
                out.push_str(line);
            }
            out.push('\n');
        }
        assert!(hit, "no row of {golden} starts with `{row}`");
        std::fs::write(&held, out).expect("the golden writes");
    }

    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(root.join("scripts/welfare"))
        .current_dir(&stage)
        .output()
        .expect("welfare runs");
    let said = String::from_utf8_lossy(&done.stdout).into_owned();
    let _ = std::fs::remove_dir_all(&stage);
    (done.status.success(), said)
}

/// Nothing doctored, so nothing may be red. Without this the two fixtures
/// below could both pass on a welfare that fails on every input.
#[test]
fn the_undoctored_goldens_hold_the_floor() {
    let (ok, said) = scored("control", "", "", "");
    assert!(ok, "welfare is red before anything is doctored:\n{said}");
}

/// The dimension the digest thread was about: a peak that grows with the input
/// rather than being reclaimed per block. Ten times the arena bytes.
#[test]
fn a_run_peak_that_grew_costs_welfare() {
    let (ok, said) = scored(
        "peak",
        "bench/cost_golden_run.txt",
        "arena_peak_bytes=",
        "arena_peak_bytes=1560450080",
    );
    assert!(!ok, "a tenfold run peak scored nothing:\n{said}");
    assert!(said.contains("run_peak_bytes"), "the fall names no run row:\n{said}");
}

/// The other half. A peak-only term would rank a change that reclaims per
/// block above one that does not however long it takes.
#[test]
fn a_run_that_got_slower_costs_welfare_too() {
    let (ok, said) =
        scored("work", "bench/instructions_golden.txt", "runbench ", "runbench 30437427340");
    assert!(!ok, "a tenfold run instruction count scored nothing:\n{said}");
    assert!(said.contains("run_instructions"), "the fall names no run row:\n{said}");
}
