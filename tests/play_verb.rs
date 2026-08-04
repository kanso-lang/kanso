//! What `kanso play` accepts and refuses.
//!
//! A play file is the relaxed form: functions and types defined right
//! beside the statements that use them, no `pub play` ceremony. The verb
//! is built to stay small — it runs and never builds, its imports are the
//! stdlib and nothing else, and nothing can import a play file — so the
//! form cannot leak into real programs.

use std::path::PathBuf;
use std::process::Command;

fn play(name: &str, engine: &[&str]) -> std::process::Output {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/play").join(name);
    Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("play")
        .arg(&fixture)
        .args(engine)
        .env("KANSO_SEED", "2685821657736338717")
        .output()
        .expect("kanso runs")
}

/// The relaxed form runs, and both engines print the same bytes.
#[test]
fn declarations_and_statements_share_the_file() {
    let golden =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/play/greeting.stdout");
    let want = std::fs::read_to_string(&golden).expect("the golden reads");
    for engine in [&[][..], &["--interp"][..]] {
        let run = play("greeting.kso", engine);

        assert_eq!(
            String::from_utf8_lossy(&run.stdout),
            want,
            "engine {engine:?}: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert!(run.status.success(), "engine {engine:?}");
    }
}

/// Nothing can import a play file, so `pub` inside one claims an audience
/// that cannot exist.
#[test]
fn pub_is_refused_in_a_play_file() {
    let run = play("no_exports.kso", &[]);

    let err = String::from_utf8_lossy(&run.stderr);
    assert!(err.contains("a play file exports nothing"), "wrong diagnostic: {err}");
    assert_eq!(run.status.code(), Some(2));
}

/// The stdlib and nothing else — the same contract the web playground has.
#[test]
fn a_local_import_is_refused_in_a_play_file() {
    let run = play("no_local_imports.kso", &[]);

    let err = String::from_utf8_lossy(&run.stderr);
    assert!(
        err.contains("a play file imports the stdlib and nothing else"),
        "wrong diagnostic: {err}"
    );
    assert_eq!(run.status.code(), Some(2));
}

/// The verb runs; it does not build. A binary would be the first step of a
/// real program, and real programs use `run` and library files.
#[test]
fn play_files_do_not_build() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/play/greeting.kso");
    let run = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("build")
        .arg(&fixture)
        .output()
        .expect("kanso runs");

    assert_ne!(run.status.code(), Some(0), "build accepted a play file");
}
