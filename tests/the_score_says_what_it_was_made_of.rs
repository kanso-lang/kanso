//! The objective can name the counter set it scored.
//!
//! The chart replay ruled on 2026-08-31 could not be computed, and the reason
//! was data rather than design. `perf_record` writes one history row per merged
//! commit from a hand-picked list of counters, and on 2026-09-03 the newest row
//! — commit a100f4f — held 12 of the 24 counters the formula reads.
//! `compile_instructions` was among the twelve missing. So "the replayed series
//! begins at the first commit for which every counter in the current formula
//! exists" named no commit at all, and the replayed line would have been empty.
//!
//! `welfare --counters` prints the set `score` was given, so a row can carry
//! exactly what the formula reads. Printing it here rather than assembling it
//! again in perf_record is the whole point: two lists drift the first time a
//! counter joins the model, and a chart replaying today's formula over
//! yesterday's counter set is wrong without saying so.

use std::process::Command;

fn counters() -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(root.join("scripts/welfare"))
        .arg("--")
        .arg("--counters")
        .current_dir(root)
        .output()
        .expect("welfare runs");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Every counter the model weighs, and nothing else. The list is written out
/// rather than derived, because a spec that recomputes what the tool computes
/// is asserting its own copy of the tool — and this list IS the thing the
/// replay depends on, so it is pinned where a reader can see it move.
#[test]
fn the_counter_set_is_the_one_the_formula_reads() {
    let said = counters();
    let want = [
        "decode_instructions",
        "encode_instructions",
        "oneshot_instructions",
        "basket_instructions",
        "wide_instructions",
        "deep_instructions",
        "pending_instructions",
        "digest_instructions",
        "scan_instructions",
        "escape_instructions",
        "index_instructions",
        "decode_peak_bytes",
        "decode_arena_blocks",
        "encode_peak_bytes",
        "encode_arena_blocks",
        "oneshot_peak_bytes",
        "basket_peak_bytes",
        "scan_arena_blocks",
        "scan_peak_bytes",
        "digest_peak_bytes",
        "digest_arena_blocks",
        "compile_instructions",
        "compile_allocs",
        "compile_peak_bytes",
    ];
    let named: Vec<&str> =
        said.lines().filter_map(|l| l.split('=').next()).filter(|l| !l.is_empty()).collect();
    assert_eq!(named.len(), want.len(), "one line per counter the model weighs: {said}");
    for c in want {
        assert!(named.contains(&c), "the set is missing {c}: {said}");
    }
}

/// Every line is `name=value` with a value that parses, because a row assembled
/// from this is read back as numbers. A counter printed with no value would
/// reach the history as a null and take the replayed line out silently.
#[test]
fn every_counter_carries_a_number() {
    for line in counters().lines() {
        let (name, value) = line.split_once('=').unwrap_or_else(|| panic!("name=value: {line}"));
        assert!(!name.is_empty(), "a named counter: {line}");
        value.parse::<u128>().unwrap_or_else(|_| panic!("a number for {name}: {line}"));
    }
}

/// The flag reports and does not ratchet. `--counters` must not be a path that
/// can move the floor, which is the one thing this file must never become a
/// second door to.
#[test]
fn asking_what_was_scored_does_not_move_the_floor() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let floor = root.join("bench/welfare_floor.json");
    let before = std::fs::read(&floor).expect("the floor reads");
    let _ = counters();
    let after = std::fs::read(&floor).expect("the floor reads");
    assert_eq!(before, after, "--counters is a report, not a ratchet");
}

/// The history row carries every counter the formula reads.
///
/// This is the property the chart replay rests on, and it was false until now:
/// on 2026-09-03 the newest row in the perf-history branch held 12 of the 24,
/// so "the first commit whose counter set is complete" named no commit and the
/// replayed line would have been empty.
///
/// It is asserted against `welfare --counters` rather than against a list
/// written here, because a list written here is a THIRD copy and would drift
/// the same way the second one did. The row and the objective have to agree;
/// what they agree ON is welfare's business.
#[test]
fn the_history_row_carries_the_whole_counter_set() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(root.join("scripts/perf_record"))
        .current_dir(root)
        .output()
        .expect("perf_record runs");
    let row = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(out.status.success(), "perf_record assembles a row: {row}");

    for line in counters().lines() {
        let name = line.split('=').next().expect("a named counter");
        assert!(
            row.contains(&format!("\"{name}\"")),
            "the row is missing {name}, which the formula weighs — the replayed \
             line cannot start while that is true"
        );
    }
}
