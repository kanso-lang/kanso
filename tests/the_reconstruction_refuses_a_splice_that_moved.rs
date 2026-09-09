//! `scripts/welfare_rescore` rebuilds `run_instructions` for the history rows
//! measured before runbench existed, and it splices that rebuild onto the
//! first row that carries the real counter.
//!
//! The rebuild is a RATIO to an anchor, scaled so the anchor equals the
//! measured row. That is only continuous if the two rows do the same runtime
//! work, and they are different commits. If they do not, every rebuilt row
//! slides by the difference — and it slides smoothly, so the result looks like
//! history rather than like a bug. Nothing downstream could catch it: the
//! welfare column would still be monotone, still plausible, still wrong.
//!
//! So the tool refuses rather than splicing, and this holds that refusal red.
//! Two fixtures differing in one counter: one where the splice rows agree, and
//! one where the measured row's `encode_instructions` moved. The first must
//! rebuild and the second must fail.
//!
//! THE NUMBERS ARE PINNED, NOT BANDED. The older row carries every phase at
//! half the anchor's value, so its share-weighted ratio is exactly 0.5 whatever
//! the shares are, and against a measured 1,000 it must read 500. The anchor
//! rebuilds to 1,000 itself, which is the splice.
//!
//! AND THE TOOL READS ITS OWN OUTPUT, every run. ci.yml takes
//! `origin/perf-history:history.jsonl` — the previous run's output — appends
//! one row and rescores the lot, so a rebuilt `run_instructions` written into
//! a row comes back as an input. On 2026-09-09 that turned main red: the
//! forty-eight all-phase rows gained a rebuilt count, the next run read them
//! as MEASURED rows, and the oldest of them became the splice anchor — forty-
//! seven commits older than the row it was compared against, so of course they
//! disagreed and the tool refused. Before that run no row carried both a full
//! phase set and a run count, in three successive files. The third test below
//! holds the property that would have caught it on the day.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// The eight phase counters, read off the map rather than listed here — a list
/// here would be the next thing to go stale when a phase is added.
fn phase_counters() -> Vec<String> {
    let text = std::fs::read_to_string(root().join("bench/runbench_phases.txt"))
        .expect("the phase map is readable");
    text.lines()
        .filter(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| l.split_whitespace().nth(1).map(str::to_string))
        .collect()
}

fn row(commit: &str, each: u64, run: Option<u64>, only: &[String]) -> String {
    let mut fields: Vec<String> = vec![format!("\"commit\":\"{commit}\"")];
    for c in only {
        fields.push(format!("\"{c}\":{each}"));
    }
    if let Some(n) = run {
        fields.push(format!("\"run_instructions\":{n}"));
    }
    format!("{{{}}}", fields.join(","))
}

/// The model welfare writes, which the tool reads on stdin.
fn model() -> String {
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(root().join("scripts/welfare"))
        .arg("--")
        .arg("--model")
        .current_dir(root())
        .output()
        .expect("welfare runs");
    assert!(done.status.success(), "welfare --model should hold");
    String::from_utf8_lossy(&done.stdout).into_owned()
}

/// Run the tool over a fixture history. Returns (succeeded, stdout+stderr).
fn rescore(tag: &str, lines: &[String]) -> (bool, String) {
    // One directory per case. Both tests run in the same process, so a name
    // keyed only on the pid is one name, and cargo runs them concurrently:
    // the two fixtures overwrote each other and the passing case read the
    // refusing case's row. That failure looked exactly like a real refusal.
    let dir = std::env::temp_dir().join(format!("rescore-splice-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("stage");
    let path = dir.join("history.jsonl");
    std::fs::write(&path, format!("{}\n", lines.join("\n"))).expect("fixture");

    let mut child = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(root().join("scripts/welfare_rescore"))
        .arg("--")
        .arg(&path)
        .current_dir(root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("rescore starts");
    child.stdin.as_mut().expect("stdin").write_all(model().as_bytes()).expect("model in");
    let done = child.wait_with_output().expect("rescore ends");
    let _ = std::fs::remove_dir_all(&dir);
    (
        done.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&done.stdout),
            String::from_utf8_lossy(&done.stderr)
        ),
    )
}

#[test]
fn a_row_carrying_every_phase_is_rebuilt_against_the_measured_row() {
    let ph = phase_counters();
    let older = row("aaaaaaa", 500, None, &ph);
    let anchor = row("bbbbbbb", 1000, None, &ph);
    // The measured row carries only the phases a modern row still has, and
    // agrees with the anchor on them.
    let modern = row("ccccccc", 1000, Some(1000), &["encode_instructions".to_string()]);

    let (ok, said) = rescore("agrees", &[older, anchor, modern]);
    assert!(ok, "the rescore should hold when the splice agrees:\n{said}");

    let out: Vec<&str> = said.lines().filter(|l| l.starts_with('{')).collect();
    assert_eq!(out.len(), 3, "three rows in, three out:\n{said}");
    assert!(
        out[0].contains("\"run_instructions\":500"),
        "the older row is half the anchor, so it rebuilds to exactly 500:\n{}",
        out[0]
    );
    assert!(
        out[1].contains("\"run_instructions\":1000"),
        "the anchor rebuilds to the measured row, which is the splice:\n{}",
        out[1]
    );
}

/// Feeding the tool its own output must give the same answer. That is not a
/// nicety here: it is exactly what CI does on every push to main, and the one
/// property under which "rewrite the whole column every time" is safe.
///
/// The second run is the one that matters. On the first, the older row has no
/// run count and is rebuilt to 500; the output carries that 500. On the second
/// that row looks measured, and a tool that cannot tell a rebuilt count from a
/// measured one picks it as the splice anchor and refuses.
#[test]
fn the_rescore_is_idempotent_over_its_own_output() {
    let ph = phase_counters();
    let older = row("aaaaaaa", 500, None, &ph);
    let anchor = row("bbbbbbb", 1000, None, &ph);
    let modern = row("ccccccc", 1000, Some(1000), &["encode_instructions".to_string()]);

    let (ok, said) = rescore("once", &[older, anchor, modern]);
    assert!(ok, "the first pass should hold:\n{said}");
    let first: Vec<String> =
        said.lines().filter(|l| l.starts_with('{')).map(str::to_string).collect();
    assert_eq!(first.len(), 3, "three rows in, three out:\n{said}");

    let (ok, said) = rescore("twice", &first);
    assert!(
        ok,
        "the tool reads its own output on every CI run, so a second pass must hold:\n{said}"
    );
    let second: Vec<String> =
        said.lines().filter(|l| l.starts_with('{')).map(str::to_string).collect();
    assert_eq!(second, first, "a second pass must not move a single row");
}

/// A count this tool computed is its arithmetic forever. Nothing re-measures a
/// commit from three weeks ago, so `run_source` is a fact about where a number
/// came from and cannot expire — which makes it unlike the welfare column
/// beside it, and the first cut of the mark got that wrong by treating them
/// alike.
///
/// The degraded pass is where it shows. With no row available as a splice
/// anchor nothing is rebuilt, and a mark re-derived from scratch then reads
/// every previously-rebuilt row as MEASURED, because all it can see is a row
/// carrying a count. That is precisely the confusion the mark exists to
/// prevent, written by the mark itself. Watched red: restoring the strip turns
/// both rebuilt rows into "measured" here.
#[test]
fn a_rebuilt_count_stays_marked_rebuilt_when_the_anchor_is_gone() {
    let ph = phase_counters();
    let older = row("aaaaaaa", 500, None, &ph);
    let anchor = row("bbbbbbb", 1000, None, &ph);
    let modern = row("ccccccc", 1000, Some(1000), &["encode_instructions".to_string()]);

    let (ok, said) = rescore("sticky-one", &[older, anchor, modern]);
    assert!(ok, "the first pass should hold:\n{said}");
    let first: Vec<String> =
        said.lines().filter(|l| l.starts_with('{')).map(str::to_string).collect();
    assert_eq!(first.len(), 3, "three rows in, three out:\n{said}");
    assert!(
        first[0].contains("\"run_source\":\"rebuilt\"")
            && first[1].contains("\"run_source\":\"rebuilt\""),
        "the two phase-carrying rows are rebuilt and say so:\n{}\n{}",
        first[0],
        first[1]
    );
    assert!(
        first[2].contains("\"run_source\":\"measured\""),
        "the row whose count this tool did not write is measured:\n{}",
        first[2]
    );

    // Drop the measured row. Nothing can serve as an anchor now, so nothing is
    // rebuilt on this pass — and the two rows must still say where their
    // counts came from.
    let (ok, said) = rescore("sticky-two", &first[..2]);
    assert!(ok, "a history with no anchor still scores:\n{said}");
    let second: Vec<String> =
        said.lines().filter(|l| l.starts_with('{')).map(str::to_string).collect();
    for line in &second {
        assert!(
            line.contains("\"run_source\":\"rebuilt\""),
            "a rebuilt count does not become measured because this pass could not \
             rebuild it:\n{line}"
        );
    }
}

#[test]
fn the_rescore_refuses_when_the_splice_rows_moved() {
    let ph = phase_counters();
    let older = row("aaaaaaa", 500, None, &ph);
    let anchor = row("bbbbbbb", 1000, None, &ph);
    // One counter apart from the passing case: the measured row did different
    // work from the anchor, so nothing may be spliced onto it.
    let moved = row("ccccccc", 1001, Some(1000), &["encode_instructions".to_string()]);

    let (ok, said) = rescore("moved", &[older, anchor, moved]);
    assert!(!ok, "a splice onto a row that moved must be refused, not scaled:\n{said}");
    assert!(
        said.contains("splice rows disagree"),
        "the refusal should say what it refused:\n{said}"
    );
}
