//! The objective can name the counter set it scored.
//!
//! The welfare column is written over the stored rows under one formula —
//! ruled 2026-08-31, restated 2026-09-03 as one line rewritten in place — and
//! a row can only be scored on the counters it carries. `perf_record` used to
//! write one history row per merged commit from a hand-picked list, and on
//! 2026-09-03 the newest row — commit a100f4f — held 12 of the 24 counters the
//! formula reads, `compile_instructions` among them.
//!
//! The set is five now rather than twenty-eight, and the reason it is worth
//! pinning has not changed with the size: a row scored on yesterday's counter
//! set is wrong without saying so, whichever direction the set moved.
//!
//! `welfare --counters` prints the set `score` was given, so a row can carry
//! exactly what the formula reads. Printing it here rather than assembling it
//! again in perf_record is the whole point: two lists drift the first time a
//! counter joins the model, and a row scored on yesterday's counter set is
//! wrong without saying so.

use std::process::Command;

fn counters() -> String {
    ask(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), &["--", "--counters"])
}

fn ask(root: &std::path::Path, args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(root.join("scripts/welfare"))
        .args(args)
        .current_dir(root)
        .output()
        .expect("welfare runs");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Every counter the model weighs, and nothing else. The list is written out
/// rather than derived, because a spec that recomputes what the tool computes
/// is asserting its own copy of the tool — and this list IS what a row has to
/// carry to be scored, so it is pinned where a reader can see it move.
#[test]
fn the_counter_set_is_the_one_the_formula_reads() {
    let said = counters();
    // Five since the 2026-09-06 gavel put the run side on one consolidated
    // program. It was twenty-eight: thirteen work rows and twelve memory rows
    // over thirteen benchmarks, plus the three compile rows. Every one of those
    // goldens still exists and still fails CI when it moves; they stopped being
    // objective INPUTS.
    let want = [
        "run_instructions",
        "run_peak_bytes",
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
/// reach the history as a null and drop its term out of the row's score.
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

/// TWO IMPLEMENTATIONS OF ONE FORMULA, on one set of readings. `welfare`
/// scores the goldens directly; `welfare_rescore` scores a history row, and
/// the column it writes is what the chart draws. They are separate code and
/// they must not drift, so this builds the row the objective would record
/// today — `--counters` is that half of a row exactly — and requires the
/// rescorer's column to be the number welfare reports.
///
/// COMPARED AT ONE PRECISION, because rounding a rounded number is wrong at a
/// boundary. This read the banner's two places and rounded the column's four
/// down to them, and on kanso#1372 the two implementations agreed exactly at
/// 67.54499292290286 and were reported as disagreeing: four places make that
/// 67.5450, rounding 67.5450 gives 67.55, and the banner says 67.54. Anything
/// in [67.5445, 67.5450) reads that way. `welfare --score` prints the column's
/// own precision, so the two are compared as they are written — which is also
/// a hundred times tighter than the old comparison, and still not a tolerance.
#[test]
fn the_rescored_column_is_the_score_the_tool_reports() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let row: String = counters()
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| format!("{k:?}:{v},"))
        .collect();

    let stage = std::env::temp_dir().join("kanso-rescore-agrees");
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).expect("a staging directory");
    let history = stage.join("history.jsonl");
    std::fs::write(&history, format!("{{{row}\"commit\":\"staged\"}}\n")).expect("the row writes");

    let model = ask(root, &["--", "--model"]);
    let mut child = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(root.join("scripts/welfare_rescore"))
        .arg("--")
        .arg(&history)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the rescorer starts");
    {
        use std::io::Write;
        child
            .stdin
            .take()
            .expect("a pipe to write the model down")
            .write_all(model.as_bytes())
            .expect("the model writes");
    }
    let done = child.wait_with_output().expect("the rescorer finishes");
    let _ = std::fs::remove_dir_all(&stage);
    assert!(
        done.status.success(),
        "the rescorer refused: {}",
        String::from_utf8_lossy(&done.stderr)
    );
    let said = String::from_utf8_lossy(&done.stdout);
    let at = said.find("\"welfare\":\"").expect("a welfare column") + 11;
    let rest = &said[at..];
    let column = &rest[..rest.find('"').expect("the column ends")];

    let ours = ask(root, &["--", "--score"]).trim().to_string();

    assert_eq!(column, ours, "the rescorer writes {column}, welfare scores {ours}");
}

/// The model dates the formula, because that date is what the rescorer stamps
/// on every row as `scored_by`. Without it the rescorer refuses rather than
/// guessing, and five hundred rows would go untagged.
#[test]
fn the_model_dates_the_formula_it_describes() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let model = ask(root, &["--", "--model"]);
    let dated = model
        .lines()
        .find_map(|l| l.strip_prefix("formula "))
        .expect("the model dates the formula")
        .to_string();
    assert!(dated.len() == 10 && dated.starts_with("20"), "a date: {dated}");
}
