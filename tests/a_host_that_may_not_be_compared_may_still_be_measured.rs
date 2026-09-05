//! A golden's refusal tells a reader to copy CI's numbers out of the job log.
//! For the three compile veins that instruction was impossible to follow.
//!
//! Every compile gate called `measured_on.sh` under `set -e`, so a host the
//! golden does not name stopped the gate before it measured anything. On
//! 2026-09-03 the runner's rustc moved 1.98.0 to 1.98.1 and all three refused
//! at once, each printing a sentence pointing at rows that were never
//! produced. A branch in that state cannot be brought to green by anybody:
//! the numbers it needs do not exist and the only host allowed to make them
//! is the one that stopped.
//!
//! `host_gate.sh` splits the question in two. May this host be COMPARED, and
//! may it be MEASURED? A container answers no to both — its numbers going
//! into a golden over the runner's is the accident `measured_on` was written
//! after. CI answers no to the first and YES to the second, because CI's
//! sitting is the only one that may ever be recorded.
//!
//! Split into its own script so the three answers can be watched without a
//! callgrind run each time.

use std::path::{Path, PathBuf};
use std::process::Command;

struct Answer {
    code: i32,
    said: String,
}

/// `ci` chooses which host this is: CI sets GITHUB_ACTIONS, a container does
/// not. The variable is cleared rather than merely unset for the container
/// case, because the test process may itself be running in CI.
fn asked(name: &str, measured_on: &str, ci: bool) -> Answer {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let stage: PathBuf = std::env::temp_dir().join(format!("kanso-host-gate-{name}"));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).expect("a staging directory");
    let golden_at = stage.join("golden.txt");
    std::fs::write(&golden_at, format!("# measured-on {measured_on}\nsome_counter=1\n"))
        .expect("the golden writes");

    let mut cmd = Command::new("sh");
    cmd.arg(root.join("scripts/gates/host_gate.sh")).arg(&golden_at).current_dir(root);
    if ci {
        cmd.env("GITHUB_ACTIONS", "true");
    } else {
        cmd.env_remove("GITHUB_ACTIONS");
    }
    let out = cmd.output().expect("the script runs");
    let mut said = String::from_utf8_lossy(&out.stdout).into_owned();
    said.push_str(&String::from_utf8_lossy(&out.stderr));
    Answer { code: out.status.code().expect("the script exits"), said }
}

/// The version this host actually runs, so the matching case is a real match
/// rather than a string this test made up.
fn this_rustc() -> String {
    let out = Command::new("rustc").arg("--version").output().expect("rustc answers");
    let line = String::from_utf8_lossy(&out.stdout).into_owned();
    let after = line.split_whitespace().nth(1).expect("a version word").to_string();
    format!("rustc={after}")
}

/// The host the golden names. Compare as usual.
#[test]
fn a_host_the_golden_names_compares() {
    let answer = asked("match", &this_rustc(), false);
    assert_eq!(answer.code, 0, "the named host compares: {}", answer.said);
}

/// A container on the wrong toolchain. This is the accident `measured_on` was
/// written after, and it still stops here with nothing to paste.
#[test]
fn a_container_on_another_toolchain_stops_and_prints_no_rows() {
    let answer = asked("container", "rustc=0.0.0", false);
    assert_eq!(answer.code, 1, "a container stops: {}", answer.said);
    assert!(
        !answer.said.contains("Measuring anyway"),
        "and is never invited to measure: {}",
        answer.said
    );
}

/// CI on the wrong toolchain. THIS is the case that had no answer: the gate
/// must go on to measure, so the job log carries the sitting the refusal tells
/// a reader to copy.
#[test]
fn ci_on_another_toolchain_is_told_to_measure_and_still_fails() {
    let answer = asked("ci", "rustc=0.0.0", true);
    assert_eq!(
        answer.code, 3,
        "CI measures and refuses, which is neither a pass nor a plain stop: {}",
        answer.said
    );
    assert!(
        answer.said.contains("NOTHING IS"),
        "and it says plainly that no comparison happened: {}",
        answer.said
    );
}

/// The two refusals stay distinguishable from a gate called wrong. A usage
/// error must not read as "measure anyway" on CI, or a missing argument would
/// quietly produce a sitting nobody asked for.
#[test]
fn a_usage_error_is_not_a_host_answer() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Command::new("sh")
        .arg(root.join("scripts/gates/host_gate.sh"))
        .current_dir(root)
        .env("GITHUB_ACTIONS", "true")
        .output()
        .expect("the script runs");
    assert_eq!(out.status.code(), Some(2), "usage is its own answer, on CI too");
}
