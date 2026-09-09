//! `bench/compile_epochs.txt` names each commit that changed HOW the compile
//! counters are measured, and by how much. `scripts/welfare_rescore` divides
//! the baseline by every factor from a row's own epoch forward, so a row
//! measured on a smaller workload is scored against a smaller baseline.
//!
//! The property that file exists for: two rows straddling a boundary, whose
//! counters stand in the ratio the file records, score the same. That is what
//! "the same compiler, measured two ways" means, and without it the welfare
//! column steps where the ruler moved rather than where the compiler did.
//!
//! THE FACTORS ARE READ OFF THE FILE, not listed here. A list here would be
//! the next thing to go stale when a boundary is added, and the count in it
//! would be the third count in this repo to go stale that way.
//!
//! THE BOUNDARIES A HISTORY CARRIES MUST BE A SUFFIX OF THE TABLE, and the
//! three tests below are the three situations that rule covers. All present:
//! the table applies. A hole in the middle, or the newest gone while an older
//! one stays: a stale table, refused. None at all: the whole table has fallen
//! off the front, which is a staged fixture — every other spec over this tool
//! writes one — and gets no epochs rather than a scaling it never had. The
//! bounded history file (ci.yml keeps the newest 500 rows) is why the oldest
//! boundary leaving is legitimate rather than stale.
//! The rows that are not under test carry no compile counters, so they are
//! scored on nothing and drop their welfare column, which is the tool's own
//! arm for a row that predates every counter.
//!
//! THE NUMBERS ARE EXACT, not near. The factors are recorded to four places
//! and the before-row carries 10,000 of each counter, so the boundary row's
//! reading is an integer with no rounding anywhere in the fixture.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// `(commit, counter, factor)` for every line of the epoch table, in order.
fn epochs() -> Vec<(String, String, f64)> {
    let text = std::fs::read_to_string(root().join("bench/compile_epochs.txt"))
        .expect("the epoch table is readable");
    text.lines()
        .filter(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let parts: Vec<&str> = l.split_whitespace().collect();
            assert_eq!(parts.len(), 3, "an epoch line is `<commit> <counter> <factor>`: {l}");
            (parts[0].to_string(), parts[1].to_string(), parts[2].parse().expect("a factor"))
        })
        .collect()
}

/// The boundary commits, in the order the file lists them, without repeats.
fn boundaries() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (commit, _, _) in epochs() {
        if !out.contains(&commit) {
            out.push(commit);
        }
    }
    out
}

fn row(commit: &str, counters: &[(String, u64)]) -> String {
    let mut fields: Vec<String> = vec![format!("\"commit\":\"{commit}\"")];
    for (name, value) in counters {
        fields.push(format!("\"{name}\":{value}"));
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

/// Run the tool over a fixture history and hand back the rows it wrote.
fn rescore(tag: &str, lines: &[String]) -> Vec<String> {
    // One directory per case: the tests in this file run in the same process
    // and cargo runs them concurrently, so a name keyed only on the pid is one
    // name and the fixtures overwrite each other.
    let dir = std::env::temp_dir().join(format!("rescore-epochs-{}-{tag}", std::process::id()));
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
    // A tool that dies before reading stdin breaks the pipe, and an expect
    // here reports THAT instead of what the tool said. Let the write fail and
    // the status assertion below print the diagnostic.
    let _ = child.stdin.as_mut().expect("stdin").write_all(model().as_bytes());
    drop(child.stdin.take());
    let done = child.wait_with_output().expect("rescore ends");
    let _ = std::fs::remove_dir_all(&dir);
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&done.stdout),
        String::from_utf8_lossy(&done.stderr)
    );
    assert!(done.status.success(), "the rescore should hold on {tag}:\n{said}");
    said.lines().filter(|l| l.starts_with('{')).map(str::to_string).collect()
}

/// Run the tool over a fixture it must REFUSE, and hand back what it said.
fn refused(tag: &str, lines: &[String]) -> String {
    let dir = std::env::temp_dir().join(format!("rescore-partial-{}-{tag}", std::process::id()));
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
    let _ = child.stdin.as_mut().expect("stdin").write_all(model().as_bytes());
    drop(child.stdin.take());
    let done = child.wait_with_output().expect("rescore ends");
    let _ = std::fs::remove_dir_all(&dir);
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&done.stdout),
        String::from_utf8_lossy(&done.stderr)
    );
    assert!(!done.status.success(), "the rescore should refuse {tag}, and it wrote:\n{said}");
    said
}

/// The welfare column a rescored row carries, or None when it has none.
///
/// The column is a STRING — `fixed` renders it to four places and `put` stores
/// what it rendered — so this reads between the quotes. A first cut walked
/// digits from the colon, hit the opening quote immediately, and answered the
/// empty string for every row: under a mutation that moved both numbers the
/// comparison was `"" == ""` and the spec passed. Watching it fail is what
/// found that, which is the whole reason for watching it fail.
fn welfare(row: &str) -> Option<String> {
    let at = row.find("\"welfare\":\"")? + "\"welfare\":\"".len();
    let rest = &row[at..];
    let end = rest.find('"')?;
    let seen = &rest[..end];
    assert!(
        !seen.is_empty() && seen.chars().all(|c| c == '.' || c == '-' || c.is_ascii_digit()),
        "a welfare column is a number: {seen:?}"
    );
    Some(seen.to_string())
}

#[test]
fn two_rows_straddling_a_boundary_score_the_same() {
    let all = epochs();
    let commits = boundaries();
    assert!(!commits.is_empty(), "the epoch table names at least one boundary");

    for under_test in &commits {
        let counters: Vec<(String, f64)> = all
            .iter()
            .filter(|(c, _, _)| c == under_test)
            .map(|(_, name, f)| (name.clone(), *f))
            .collect();
        assert!(!counters.is_empty(), "{under_test} names at least one counter");

        // 10,000 before the boundary, and the boundary's own reading after it.
        // Four-place factors make every one of these an exact integer.
        let before: Vec<(String, u64)> =
            counters.iter().map(|(n, _)| (n.clone(), 10_000)).collect();
        let after: Vec<(String, u64)> = counters
            .iter()
            .map(|(n, f)| {
                let scaled = 10_000.0 * f;
                assert!(
                    (scaled - scaled.round()).abs() < 1e-9,
                    "{n}'s factor {f} does not scale 10,000 to an integer"
                );
                (n.clone(), scaled.round() as u64)
            })
            .collect();

        // Every boundary commit is present, so the tool has no missing one to
        // refuse; the ones not under test carry no counters at all.
        let mut lines: Vec<String> = Vec::new();
        for commit in &commits {
            if commit == under_test {
                lines.push(row("earlier", &before));
                lines.push(row(commit, &after));
            } else {
                lines.push(row(commit, &[]));
            }
        }

        let out = rescore(under_test, &lines);
        assert_eq!(out.len(), lines.len(), "every row in, every row out, for {under_test}");

        let pair: Vec<&String> = out
            .iter()
            .filter(|l| l.contains("\"earlier\"") || l.contains(&format!("\"{under_test}\"")))
            .collect();
        assert_eq!(pair.len(), 2, "the pair under test is two rows for {under_test}");

        let left =
            welfare(pair[0]).unwrap_or_else(|| panic!("the earlier row scores: {}", pair[0]));
        let right =
            welfare(pair[1]).unwrap_or_else(|| panic!("the boundary row scores: {}", pair[1]));
        assert_eq!(
            left, right,
            "{under_test} is a change of measurement, so the same compiler either side of it \
             must score the same; the epoch table's factors did not flatten it"
        );
    }
}

/// A boundary missing from the MIDDLE of the table, or the newest one missing
/// while an older one is present, is a stale table, and the answer it then
/// gives is the dangerous kind: the rows past the hole sit on the wrong divisor
/// while every number stays plausible. So that refuses.
///
/// One fixture per boundary that is not the oldest, each carrying every commit
/// but that one — which leaves an older one present and so is never a suffix.
/// The rows hold no counters; what is under test is the refusal, which happens
/// before anything is scored.
#[test]
fn a_history_with_a_hole_in_the_table_is_refused() {
    let commits = boundaries();
    assert!(commits.len() > 1, "a hole needs at least two boundaries to be a hole");

    for left_out in commits.iter().skip(1) {
        let lines: Vec<String> =
            commits.iter().filter(|c| c != &left_out).map(|c| row(c, &[])).collect();
        let said = refused(left_out, &lines);
        assert!(
            said.contains("a compile epoch names a commit no history row carries"),
            "leaving {left_out} out leaves an older boundary present, so it should be \
             refused by name; the tool said:\n{said}"
        );
    }
}

/// The history file is bounded — ci.yml keeps the newest 500 rows — so the
/// oldest boundary leaves the window one day while the table still names it.
/// A boundary older than every row scales no row, and dropping it is the whole
/// of the repair; refusing there would turn main red for a file doing exactly
/// what it is meant to do. Read on 2026-09-09 the three sat at rows 444, 474
/// and 483 of 500, a position that drops by one with every merge, so the first
/// departure was 444 merges out.
///
/// THE PROPERTY IS THAT THE SURVIVORS SCORE THE SAME, not that the boundary
/// still flattens. Flattening survives the mutation this test exists to catch:
/// if the departed boundaries keep their factors in the product, every divisor
/// shifts by the same amount and every STEP is still right, so a pair either
/// side of a boundary agrees exactly as before. What moves is the absolute
/// column — against a floor that is ratcheted, which is where the damage is.
/// A first cut asserted flattening, and the mutation walked past it.
///
/// So each row is scored twice: once in a fixture carrying every boundary, and
/// once in one carrying only the suffix. A boundary older than every scored row
/// applies to none of them, so the two runs must agree to the digit.
#[test]
fn a_window_that_has_slid_past_the_oldest_boundary_scores_the_same() {
    let all = epochs();
    let commits = boundaries();

    for start in 1..commits.len() {
        let under_test = &commits[start];
        let counters: Vec<(String, f64)> = all
            .iter()
            .filter(|(c, _, _)| c == under_test)
            .map(|(_, name, f)| (name.clone(), *f))
            .collect();
        let before: Vec<(String, u64)> =
            counters.iter().map(|(n, _)| (n.clone(), 10_000)).collect();
        let after: Vec<(String, u64)> =
            counters.iter().map(|(n, f)| (n.clone(), (10_000.0 * f).round() as u64)).collect();

        // The same two counter-bearing rows in both fixtures; the only
        // difference is how many older boundaries sit in front of them.
        let staged = |from: usize| -> Vec<String> {
            let mut lines: Vec<String> = Vec::new();
            for commit in &commits[from..] {
                if commit == under_test {
                    lines.push(row("earlier", &before));
                    lines.push(row(commit, &after));
                } else {
                    lines.push(row(commit, &[]));
                }
            }
            lines
        };

        let whole = rescore(&format!("whole-{start}"), &staged(0));
        let slid = rescore(&format!("slid-{start}"), &staged(start));

        let pick = |out: &[String]| -> (String, String) {
            let pair: Vec<&String> = out
                .iter()
                .filter(|l| l.contains("\"earlier\"") || l.contains(&format!("\"{under_test}\"")))
                .collect();
            assert_eq!(pair.len(), 2, "the pair under test is two rows for {under_test}");
            (
                welfare(pair[0]).unwrap_or_else(|| panic!("the earlier row scores: {}", pair[0])),
                welfare(pair[1]).unwrap_or_else(|| panic!("the boundary row scores: {}", pair[1])),
            )
        };

        let (whole_left, whole_right) = pick(&whole);
        let (slid_left, slid_right) = pick(&slid);

        assert_eq!(
            whole_left, whole_right,
            "{under_test} is a change of measurement, so the pair straddling it flattens"
        );
        assert_eq!(
            whole_left, slid_left,
            "the {start} oldest boundaries apply to no row here, so dropping them out of the \
             window must not move what the earlier row scores"
        );
        assert_eq!(whole_right, slid_right, "nor what the boundary row scores");
    }
}

/// A counter named in the epoch table that the objective does not weigh would
/// sit there scaling nothing, and the file would look like it was doing its
/// job. `bench/objective_sources.txt` is the list of what welfare reads.
#[test]
fn every_epoch_counter_is_one_the_objective_weighs() {
    let sources = std::fs::read_to_string(root().join("bench/objective_sources.txt"))
        .expect("the objective's sources are readable");
    let weighed: Vec<&str> = sources
        .lines()
        .filter(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| l.split_whitespace().next())
        .collect();

    for (commit, counter, _) in epochs() {
        assert!(
            weighed.contains(&counter.as_str()),
            "{commit} scales `{counter}`, which the objective does not weigh — \
             bench/objective_sources.txt names {weighed:?}"
        );
    }
}
