//! The objective can name the counter set it scored.
//!
//! The welfare column is written over the stored rows under one formula —
//! ruled 2026-08-31, restated 2026-09-03 as one line rewritten in place — and
//! a row can only be scored on the counters it carries. `perf_record` used to
//! write one history row per merged commit from a hand-picked list, and on
//! 2026-09-03 the newest row — commit a100f4f — held 12 of the 24 counters the
//! formula reads, `compile_instructions` among them.
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
/// The column carries four places and welfare's banner two, so the column is
/// rounded to the banner rather than compared inside a tolerance. A tolerance
/// is a guess that stays green through exactly the drift this is here for.
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
    let theirs: f64 = column.parse().expect("a score");

    let banner = ask(root, &[]);
    let ours = banner.split_whitespace().nth(1).expect("welfare says a score").to_string();

    assert_eq!(format!("{theirs:.2}"), ours, "the rescorer writes {column}");
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
