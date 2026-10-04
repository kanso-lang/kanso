//! Every workflow installs the rustc that the goldens were measured under, and
//! names it rather than following `stable`.
//!
//! A golden whose measured-on line names rustc holds numbers that belong to
//! that rustc: the compiler's instruction counts, the interpreter's run, the
//! emitter, start-up. Until 2026-10-04 every job installed
//! `dtolnay/rust-toolchain@stable`, so the runner's rustc moved whenever a new
//! release shipped. It went from 1.98.1 to 1.99.0 after 2026-10-01, and the
//! cost goldens job went red on every pull request, a docs-only one included.
//! The rows the runner printed under 1.99.0 had fallen by up to 2.55% on a tree
//! whose code had not changed. Taking them would have credited the compiler
//! with the toolchain's work, which the 2026-09-15 rule forbids: a counter reads
//! the code under test and nothing else, and external state is put into a known
//! state before it is measured.
//!
//! So each workflow names one version in a top-level `RUST_TOOLCHAIN`, every
//! toolchain step installs `${{ env.RUST_TOOLCHAIN }}`, and the version is the
//! one every golden's measured-on line names. Moving to a new rustc is then one
//! pull request that changes all three together and re-sits the rows.

use std::fs;
use std::path::Path;

const STEP: &str = "dtolnay/rust-toolchain@";
const PINNED: &str = "toolchain: ${{ env.RUST_TOOLCHAIN }}";

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// The version a workflow's top-level `env:` names, if it names one.
fn named_version(text: &str) -> Option<String> {
    let mut in_env = false;
    for line in text.lines() {
        if line == "env:" {
            in_env = true;
            continue;
        }
        if in_env {
            if !line.starts_with(' ') {
                in_env = false;
                continue;
            }
            if let Some(v) = line.trim().strip_prefix("RUST_TOOLCHAIN:") {
                return Some(v.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

/// Every toolchain step in a workflow that does not install the named version,
/// as `line N: why`.
fn floating_steps(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut bad = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let Some(at) = line.find(STEP) else { continue };
        let rev = line[at + STEP.len()..].trim();
        if rev != "master" {
            bad.push(format!("line {}: installs `@{rev}`, which picks its own rustc", i + 1));
            continue;
        }
        let own = indent(line);
        let body = lines[i + 1..].iter().take_while(|l| l.trim().is_empty() || indent(l) > own);
        if !body.into_iter().any(|l| l.trim() == PINNED) {
            bad.push(format!("line {}: no `{PINNED}` under the step", i + 1));
        }
    }
    bad
}

#[test]
fn every_workflow_installs_the_rustc_it_names() {
    let mut versions = Vec::new();
    let mut problems = Vec::new();
    let dir = root().join(".github/workflows");
    let mut files: Vec<_> = fs::read_dir(&dir)
        .expect("the workflows directory")
        .map(|e| e.expect("a workflow entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml"))
        .collect();
    files.sort();
    for path in &files {
        let text = fs::read_to_string(path).expect("a readable workflow");
        if !text.contains(STEP) {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        match named_version(&text) {
            Some(v) => versions.push((name.clone(), v)),
            None => problems.push(format!("{name}: installs rust with no RUST_TOOLCHAIN")),
        }
        for why in floating_steps(&text) {
            problems.push(format!("{name} {why}"));
        }
    }
    assert!(
        problems.is_empty(),
        "a workflow installs a rustc it does not name:\n  {}",
        problems.join("\n  ")
    );
    assert!(!versions.is_empty(), "no workflow installs rust at all");
    let (first, want) = &versions[0];
    for (name, v) in &versions {
        assert_eq!(
            v, want,
            "{name} pins rustc {v} and {first} pins {want}; one version for every workflow"
        );
    }
    assert!(
        want.split('.').count() == 3 && want.split('.').all(|p| p.parse::<u32>().is_ok()),
        "RUST_TOOLCHAIN is {want:?}, which is not a release version like 1.98.1"
    );
}

#[test]
fn the_pinned_rustc_is_the_one_the_goldens_name() {
    let ci = fs::read_to_string(root().join(".github/workflows/ci.yml")).expect("ci.yml");
    let want = named_version(&ci).expect("ci.yml names a RUST_TOOLCHAIN");
    let mut naming = 0;
    let mut wrong = Vec::new();
    let mut files: Vec<_> = fs::read_dir(root().join("bench"))
        .expect("bench")
        .map(|e| e.expect("a bench entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .collect();
    files.sort();
    for path in &files {
        let text = fs::read_to_string(path).expect("a readable golden");
        for line in text.lines() {
            let Some(facts) = line.strip_prefix("# measured-on ") else { continue };
            for fact in facts.split(' ') {
                if let Some(v) = fact.strip_prefix("rustc=") {
                    naming += 1;
                    if v != want {
                        let name = path.file_name().unwrap().to_string_lossy();
                        wrong.push(format!("{name} names rustc={v}"));
                    }
                }
            }
        }
    }
    assert!(naming > 0, "no golden names a rustc, so nothing here is checked");
    assert!(
        wrong.is_empty(),
        "ci.yml pins rustc {want} and these goldens were measured under another:\n  {}",
        wrong.join("\n  ")
    );
}
