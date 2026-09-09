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
//! EVERY BOUNDARY'S COMMIT APPEARS IN EVERY FIXTURE HERE, because the tool
//! takes the epoch table only when it finds all of them. A history carrying
//! SOME is a stale table and is refused; one carrying NONE is not the history
//! the table describes — a staged fixture — and gets no epochs at all, which
//! is what lets every other spec over this tool stage three rows and say
//! nothing about compile epochs. The refusal has its own test below.
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

/// A table naming three boundaries and finding two is stale, and the answer it
/// then gives is the dangerous kind: the rows past the missing boundary sit on
/// the wrong divisor and every number stays plausible. So a PARTIAL match is
/// refused rather than scored.
///
/// One fixture per boundary, each carrying every commit but that one. The rows
/// hold no counters — what is under test is the refusal, which happens before
/// anything is scored.
#[test]
fn a_history_missing_one_boundary_is_refused() {
    let commits = boundaries();
    assert!(commits.len() > 1, "a partial match needs at least two boundaries to be partial");

    for left_out in &commits {
        let lines: Vec<String> =
            commits.iter().filter(|c| c != &left_out).map(|c| row(c, &[])).collect();
        let said = refused(left_out, &lines);
        assert!(
            said.contains("a compile epoch names a commit no history row carries"),
            "leaving {left_out} out should be refused by name, and the tool said:\n{said}"
        );
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
