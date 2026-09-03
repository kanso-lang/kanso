//! The welfare column is rewritten under one formula, and a row that predates
//! a counter is scored on the counters it carries.
//!
//! DIRECTIVE (Clay, 2026-09-03): "Rewrite history.jsonl's welfare column once,
//! in place, under the current formula on the counters each row carries (terms
//! average over present counters, absent terms dropped and weights
//! renormalized per row, row tagged with the scoring formula). One line."
//!
//! What replaced a two-line chart: one line drawn from a replay the browser
//! ran at view time and one line of whatever each commit shipped with. The
//! replay could score two rows of five hundred, because it refused any row
//! missing a counter and eighteen of the objective's twenty-four counters were
//! first recorded on 2026-09-02.
//!
//! Five rows, and the arithmetic is done by hand in the comments below rather
//! than read off the tool. The fixture's baselines are its own round numbers,
//! so none of these move when a benchmark does.

use std::io::Write;
use std::process::{Command, Stdio};

const MODEL: &str = "\
formula 2026-09-02
term run|0.75|2.0|a,b
term compile|0.25|0.5|c
base a=100
base b=100
base c=100
";

/// Every counter present and every ratio one. The run term saturates each
/// counter at 1/(1+2.0) and averages them, so it contributes 0.75/3; compile
/// saturates at 1/(1+0.5) and contributes 0.25 * 2/3. 100 * (0.25 + 1/6).
const WHOLE: &str = "{\"a\":100,\"b\":100,\"c\":100,\"commit\":\"r1\"}";

/// One counter of the run term's two. The term keeps its whole weight on the
/// one it has, so the score does not move; only the coverage does, from 1.00
/// to 0.75 * 1/2 + 0.25.
const HALF_A_TERM: &str = "{\"a\":100,\"c\":100,\"commit\":\"r2\"}";

/// No counter the compile term reads. The term is dropped and its 0.25 leaves
/// the denominator with it, so the row reads what it would read if the
/// objective were the run term alone: 100 * 0.25 / 0.75.
const NO_COMPILE_TERM: &str = "{\"a\":100,\"b\":100,\"commit\":\"r3\"}";

/// Twice as good as baseline everywhere: run 2/(2+2) each, compile 2/(2+0.5).
/// 100 * (0.75 * 0.5 + 0.25 * 0.8).
const TWICE_AS_GOOD: &str = "{\"a\":50,\"b\":50,\"c\":50,\"commit\":\"r4\"}";

/// A counter that reached zero. Dividing by it would answer infinity rather
/// than "as good as this gets", so both sides read a zero as a half and the
/// ratio is 200: 200/202 for a, 1/3 for b, averaged, then 2/3 for compile.
const ONE_AT_ZERO: &str = "{\"a\":0,\"b\":100,\"c\":100,\"commit\":\"r5\"}";

fn rescored(key: &str, rows: &[&str]) -> Vec<String> {
    let (code, out, said) = ran(key, MODEL, rows);
    assert!(code, "the rescorer refused: {said}");
    out
}

/// The exit, the stdout lines and the stderr, for the two callers that need
/// to see a refusal rather than assert there was none.
fn ran(key: &str, model: &str, rows: &[&str]) -> (bool, Vec<String>, String) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let stage = std::env::temp_dir().join(format!("kanso-rescore-{key}"));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).expect("a staging directory");
    let history = stage.join("history.jsonl");
    std::fs::write(&history, format!("{}\n", rows.join("\n"))).expect("the history writes");

    let mut child = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(root.join("scripts/welfare_rescore"))
        .arg("--")
        .arg(&history)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the rescorer starts");
    child
        .stdin
        .take()
        .expect("a pipe to write the model down")
        .write_all(model.as_bytes())
        .expect("the model writes");
    let done = child.wait_with_output().expect("the rescorer finishes");
    let _ = std::fs::remove_dir_all(&stage);
    (
        done.status.success(),
        String::from_utf8_lossy(&done.stdout).lines().map(str::to_string).collect(),
        String::from_utf8_lossy(&done.stderr).into_owned(),
    )
}

#[test]
fn every_counter_present_scores_the_whole_formula() {
    assert_eq!(
        rescored("whole", &[WHOLE]),
        vec![concat!(
            "{\"a\":100,\"b\":100,\"c\":100,\"commit\":\"r1\",",
            "\"scored_by\":\"2026-09-02\",\"scored_weight\":\"1.00\",",
            "\"welfare\":\"41.6667\"}"
        )]
    );
}

#[test]
fn a_term_scored_on_half_its_counters_keeps_its_weight() {
    assert_eq!(
        rescored("half", &[HALF_A_TERM]),
        vec![concat!(
            "{\"a\":100,\"c\":100,\"commit\":\"r2\",",
            "\"scored_by\":\"2026-09-02\",\"scored_weight\":\"0.63\",",
            "\"welfare\":\"41.6667\"}"
        )]
    );
}

/// The renormalization, which is the half of the rule a reader is most likely
/// to get wrong: the dropped term is not scored zero, it is not scored at all.
/// Scoring it zero would answer 25.00 here.
#[test]
fn a_term_with_no_counters_leaves_the_denominator() {
    assert_eq!(
        rescored("dropped", &[NO_COMPILE_TERM]),
        vec![concat!(
            "{\"a\":100,\"b\":100,\"commit\":\"r3\",",
            "\"scored_by\":\"2026-09-02\",\"scored_weight\":\"0.75\",",
            "\"welfare\":\"33.3333\"}"
        )]
    );
}

#[test]
fn a_better_row_scores_higher_on_the_same_counters() {
    assert_eq!(
        rescored("twice", &[TWICE_AS_GOOD]),
        vec![concat!(
            "{\"a\":50,\"b\":50,\"c\":50,\"commit\":\"r4\",",
            "\"scored_by\":\"2026-09-02\",\"scored_weight\":\"1.00\",",
            "\"welfare\":\"57.5000\"}"
        )]
    );
}

#[test]
fn a_counter_at_zero_reads_as_a_half() {
    assert_eq!(
        rescored("zero", &[ONE_AT_ZERO]),
        vec![concat!(
            "{\"a\":0,\"b\":100,\"c\":100,\"commit\":\"r5\",",
            "\"scored_by\":\"2026-09-02\",\"scored_weight\":\"1.00\",",
            "\"welfare\":\"66.2954\"}"
        )]
    );
}

/// Every row in one run, in the order the file holds them, because the column
/// is rewritten in place and a rewrite that reordered five hundred rows would
/// be a different file with the same numbers in it.
#[test]
fn the_file_keeps_its_order_and_every_row_gets_a_column() {
    let said =
        rescored("order", &[WHOLE, HALF_A_TERM, NO_COMPILE_TERM, TWICE_AS_GOOD, ONE_AT_ZERO]);
    let commits: Vec<&str> = said
        .iter()
        .map(|l| {
            let at = l.find("\"commit\":\"").expect("a commit") + "\"commit\":\"".len();
            &l[at..at + 2]
        })
        .collect();
    assert_eq!(commits, vec!["r1", "r2", "r3", "r4", "r5"]);
    let scores: Vec<&str> = said
        .iter()
        .map(|l| {
            let at = l.find("\"welfare\":\"").expect("a score") + "\"welfare\":\"".len();
            let rest = &l[at..];
            &rest[..rest.find('"').expect("the score ends")]
        })
        .collect();
    assert_eq!(scores, vec!["41.6667", "41.6667", "33.3333", "57.5000", "66.2954"]);
}

/// The tag is read off the model, not held here. A copy of the date in this
/// tool would keep its old value through the gavel that moved the formula,
/// and every row would claim a definition that no longer exists.
#[test]
fn the_tag_is_whatever_the_model_says_it_is() {
    let model = MODEL.replace("2026-09-02", "1999-12-31");
    let (ok, out, said) = ran("tag", &model, &[WHOLE]);
    assert!(ok, "the rescorer refused: {said}");
    assert!(out[0].contains("\"scored_by\":\"1999-12-31\""), "{}", out[0]);
}

/// A model with no formula line is an older welfare than this tool. Guessing
/// a date would tag every row with a definition nobody can check them
/// against, so it refuses and says what to run.
#[test]
fn a_model_with_no_formula_line_is_refused() {
    let model: String = MODEL.lines().skip(1).map(|l| format!("{l}\n")).collect();
    let (ok, out, said) = ran("noformula", &model, &[WHOLE]);
    assert!(!ok, "it scored five hundred rows against nothing: {out:?}");
    assert!(said.contains("the model has no formula line"), "{said}");
}
