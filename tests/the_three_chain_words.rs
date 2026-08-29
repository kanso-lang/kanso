//! `bind`, `rescue` and `annotate` — the three worded chain steps of the
//! 2026-08-26 gavel.
//!
//! They live here rather than in the micro corpus because that corpus runs
//! native and `--interp` and requires them to agree, and only the oracle
//! speaks these words so far. The differential law allows exactly that shape:
//! a feature may land on fewer engines while the others REJECT it, never
//! while they quietly answer something else. So each case is pinned twice —
//! what the oracle prints, and that the native backend refuses the word by
//! name.
//!
//! `rescue` is the one that is new capability rather than a respelling. An
//! err born at the edge used to reach the endpoint without knocking at any
//! continuation, because a chain step's synthesised callback returns a
//! failure argument instead of entering its body (`call_closure` in eval.rs).
//! docs/book/samples/ch05/fallback.kso is that program, and its golden is the
//! endpoint message.

use std::path::PathBuf;
use std::process::{Command, Output};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn corpus() -> PathBuf {
    root().join("tests/golden/chainwords")
}

/// Stage the case beside a generated entry that imports it, the way the
/// golden harness runs a library, so the program is entered where a user
/// enters it and the diagnostics name the case's own file.
fn run(name: &str, extra: &[&str]) -> Output {
    // the two tests run in parallel and stage the same cases, so the engine
    // being asked is part of the directory name or one deletes the other's
    let engine = match extra.is_empty() {
        true => "native",
        false => "oracle",
    };
    let stage = std::env::temp_dir().join(format!("kanso-chainwords-{engine}-{name}"));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).expect("the staging directory is made");
    std::fs::copy(corpus().join(format!("{name}.kso")), stage.join(format!("{name}.kso")))
        .expect("the case copies");
    let entry = stage.join(format!("run_{name}.kso"));
    std::fs::write(&entry, format!("import \"./{name}\"\n\n{name}/play\n"))
        .expect("the entry file writes");

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kanso"));
    // run from inside the staged directory and name the entry relatively,
    // the way a user does: an absolute entry puts an absolute path in the
    // trace line, which is a temp directory and cannot be pinned
    cmd.arg("run").arg(format!("run_{name}.kso")).args(extra).current_dir(&stage);
    let out = cmd.output().expect("kanso runs");
    let _ = std::fs::remove_dir_all(&stage);
    out
}

fn golden(name: &str, extension: &str) -> String {
    let path = corpus().join(format!("{name}.{extension}"));
    match path.exists() {
        true => std::fs::read_to_string(&path).expect("the golden reads"),
        false => String::new(),
    }
}

/// Every case in the corpus, and what it exits with. A case with no `.err`
/// beside it must say nothing on that stream.
const CASES: [(&str, i32); 4] = [
    ("a_rescue_opens_the_executors_door", 0),
    ("a_rescue_lets_a_success_past", 0),
    ("a_bind_never_sees_a_failure", 1),
    ("an_annotate_cannot_resurrect", 1),
];

#[test]
fn the_oracle_answers_every_case() {
    for (name, code) in CASES {
        let out = run(name, &["--interp"]);
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            golden(name, "out"),
            "{name} printed something else"
        );
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            golden(name, "err"),
            "{name} said something else on stderr"
        );
        assert_eq!(out.status.code(), Some(code), "{name} exited differently");
    }
}

/// The quieter engine. A word it cannot compile must be REFUSED by name — the
/// alternative the law forbids is a program that runs and answers something
/// the oracle would not.
#[test]
fn the_native_backend_refuses_each_word_by_name() {
    for (name, _) in CASES {
        let out = run(name, &[]);
        let said = String::from_utf8_lossy(&out.stderr).to_string();
        let named = ["bind", "rescue", "annotate"]
            .into_iter()
            .any(|w| said.contains(&format!("`{w}` is not yet supported")));
        assert!(named, "{name} must be refused by name on native, and said: {said}");
        assert_eq!(out.status.code(), Some(2), "{name} refuses before it runs");
        assert!(out.stdout.is_empty(), "{name} printed before refusing");
    }
}
