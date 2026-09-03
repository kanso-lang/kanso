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

/// The emitted model plus the fixed arithmetic reproduces the score.
///
/// This is the spec the chart rests on. numbers.html has to replay the current
/// formula over the stored rows, a static page computes in javascript, and two
/// things could therefore be copied onto the page and go stale: the numbers and
/// the arithmetic.
///
/// `welfare --model` removes the first hazard — weights, satiations and the
/// baseline come from the tool, so a gavel that moves a weight moves the chart
/// with it. This spec removes the second. It restates the arithmetic exactly as
/// a reader of the ruling would (saturate each counter, mean within a term,
/// weight, sum) and asserts the result against welfare's own banner, so the
/// restatement cannot drift from the tool without a red test.
///
/// It is deliberately NOT a copy of welfare's implementation. It is written
/// from the rule as stated — `r / (r + s)`, ruled 2026-08-29 to apply per
/// counter before the mean — which is what makes agreement evidence rather
/// than tautology.
#[test]
fn the_model_and_the_rule_reproduce_the_score() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let ask = |flag: Option<&str>| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kanso"));
        cmd.arg("run").arg(root.join("scripts/welfare"));
        if let Some(f) = flag {
            cmd.arg("--").arg(f);
        }
        let out = cmd.current_dir(root).output().expect("welfare runs");
        String::from_utf8_lossy(&out.stdout).into_owned()
    };

    let model = ask(Some("--model"));
    let now: std::collections::HashMap<String, f64> = ask(Some("--counters"))
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.to_string(), v.parse::<f64>().expect("a number")))
        .collect();

    // `base <counter>=<value>`
    let base: std::collections::HashMap<String, f64> = model
        .lines()
        .filter_map(|l| l.strip_prefix("base "))
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.to_string(), v.parse::<f64>().expect("a number")))
        .collect();
    assert!(!base.is_empty(), "the model carries a baseline: {model}");

    // A counter that reached zero is better than any baseline; welfare's own
    // note says dividing by it would say infinity rather than "as good as this
    // gets", so both sides read zero as a half.
    let live = |n: f64| if n == 0.0 { 0.5 } else { n };

    // `term <name>|<weight>|<satiation>|<counter,counter,...>`
    let mut total = 0.0;
    let mut seen = 0;
    for line in model.lines().filter_map(|l| l.strip_prefix("term ")) {
        let parts: Vec<&str> = line.split('|').collect();
        assert_eq!(parts.len(), 4, "a term is name, weight, satiation, counters: {line}");
        let weight: f64 = parts[1].parse().expect("a weight");
        let sat: f64 = parts[2].parse().expect("a satiation");
        let counters: Vec<&str> = parts[3].split(',').collect();
        let mut sum = 0.0;
        for c in &counters {
            let b = live(*base.get(*c).unwrap_or_else(|| panic!("a baseline for {c}")));
            let n = live(*now.get(*c).unwrap_or_else(|| panic!("a reading for {c}")));
            let ratio = b / n;
            sum += ratio / (ratio + sat);
        }
        total += sum / counters.len() as f64 * weight;
        seen += 1;
    }
    assert!(seen >= 2, "the model carries its terms: {model}");
    let mine = 100.0 * total;

    let said = ask(None);
    let banner: f64 =
        said.split_whitespace().nth(1).expect("welfare says a score").parse().expect("a number");

    assert!(
        (mine - banner).abs() < 0.005,
        "the rule applied to the emitted model gives {mine:.4}, welfare says {banner:.2}"
    );
}

/// The weights are a division of one whole. A term added without taking weight
/// from another silently reweighs every other term, which is a change to what
/// the project wants made by arithmetic rather than by a gavel.
#[test]
fn the_weights_divide_one_whole() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(root.join("scripts/welfare"))
        .arg("--")
        .arg("--model")
        .current_dir(root)
        .output()
        .expect("welfare runs");
    let model = String::from_utf8_lossy(&out.stdout);
    let sum: f64 = model
        .lines()
        .filter_map(|l| l.strip_prefix("term "))
        .map(|l| l.split('|').nth(1).expect("a weight").parse::<f64>().expect("a number"))
        .sum();
    assert!((sum - 1.0).abs() < 1e-9, "the weights sum to one, not {sum}");
}
