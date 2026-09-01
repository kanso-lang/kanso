//! A counter the runtime has only just started emitting is a measurement, not
//! a worsening.
//!
//! Every golden in a vein gains the new row on the same commit. The trend gate
//! read the absence of a row as a baseline of nought, so the first reading of
//! `survive_slots` printed as eight worsenings with no improvement anywhere —
//! a pure regression, which is the one move the gate refuses outright — for a
//! change that only made the runtime measure something it had not measured.
//!
//! The gate already says this sentence for a benchmark that JOINS a golden.
//! This is the transposed case: a row joining every sample rather than a
//! sample joining every row.
//!
//! The second fixture is what stops the rule being an escape hatch. A counter
//! the baseline DOES carry is judged the way it always was, so the exemption
//! lasts exactly one commit.

use std::path::Path;
use std::process::Command;

const COUNTER: &str = "survive_slots";

fn git(dir: &Path, args: &[&str]) {
    let done = Command::new("git").args(args).current_dir(dir).output().expect("git runs");
    assert!(
        done.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&done.stderr)
    );
}

/// Copy one directory's files, rewriting each through `edit(name, text)`.
fn laid(from: &Path, to: &Path, edit: &dyn Fn(&str, &str) -> String) {
    std::fs::create_dir_all(to).expect("a directory");
    for entry in std::fs::read_dir(from).expect("the source directory reads") {
        let path = entry.expect("a directory entry").path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().expect("named").to_string_lossy().into_owned();
        match std::fs::read_to_string(&path) {
            Ok(text) => std::fs::write(to.join(&name), edit(&name, &text)).expect("writes"),
            Err(_) => {
                std::fs::copy(&path, to.join(&name)).expect("the binary golden copies");
            }
        }
    }
}

/// Stand up a scratch repository whose COMMITTED goldens are `before` and
/// whose working tree is `after`, then run the trend gate against that commit.
/// Answers whether the gate passed, and what it printed.
fn judged(
    key: &str,
    before: &dyn Fn(&str, &str) -> String,
    after: &dyn Fn(&str, &str) -> String,
) -> (bool, String) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let stage = std::env::temp_dir().join(format!("kanso-minted-counter-{key}"));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).expect("a staging directory");

    let bench = root.join("bench");
    let mem = root.join("tests/golden/mem");
    laid(&bench, &stage.join("bench"), before);
    laid(&mem, &stage.join("tests/golden/mem"), before);

    git(&stage, &["init", "--quiet"]);
    git(&stage, &["config", "user.email", "spec@kanso.invalid"]);
    git(&stage, &["config", "user.name", "spec"]);
    git(&stage, &["add", "-A"]);
    git(&stage, &["commit", "--quiet", "-m", "the baseline"]);

    laid(&bench, &stage.join("bench"), after);
    laid(&mem, &stage.join("tests/golden/mem"), after);

    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(root.join("scripts/trend_gate"))
        .arg("--")
        .arg("HEAD")
        .current_dir(&stage)
        .output()
        .expect("the trend gate runs");
    // stderr rides along so a gate that refused to run says why, rather than
    // failing a fixture with an empty quotation.
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&done.stdout),
        String::from_utf8_lossy(&done.stderr)
    );
    let _ = std::fs::remove_dir_all(&stage);
    (done.status.success(), said)
}

fn unchanged(_name: &str, text: &str) -> String {
    text.to_string()
}

/// The tree as it was before the runtime emitted the counter at all.
fn without_the_counter(_name: &str, text: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        if line.starts_with(&format!("{COUNTER}=")) {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

#[test]
fn a_counter_the_baseline_never_carried_is_a_measurement() {
    let (passed, said) = judged("minted", &without_the_counter, &unchanged);
    assert!(said.contains("MINTED"), "the gate should say the counter is new; it said:\n{said}");
    assert!(
        !said.contains("worsened: encode_survive_slots"),
        "a counter with no baseline cannot have worsened; it said:\n{said}"
    );
    assert!(passed, "a runtime that measures one more thing is not a regression; it said:\n{said}");
}

#[test]
fn a_counter_the_baseline_carries_is_judged_the_way_it_always_was() {
    let raised = |name: &str, text: &str| {
        if name != "cost_golden_basket.txt" {
            return text.to_string();
        }
        let mut out = String::new();
        let mut hit = false;
        for line in text.lines() {
            if line.starts_with(&format!("{COUNTER}=")) {
                out.push_str(&format!("{COUNTER}=99000000"));
                hit = true;
            } else {
                out.push_str(line);
            }
            out.push('\n');
        }
        assert!(hit, "the basket golden no longer pins {COUNTER}");
        out
    };
    let (passed, said) = judged("carried", &unchanged, &raised);
    assert!(
        said.contains("worsened: basket_survive_slots"),
        "a counter with a baseline still moves against it; it said:\n{said}"
    );
    assert!(!passed, "one counter worse and none better is the refusal; it said:\n{said}");
}
