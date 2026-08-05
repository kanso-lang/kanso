//! What a reader types first.
//!
//! An example is the shipped program somebody runs before they have read
//! anything, so a verb has to accept it. When `play` left the compiler, forty
//! of them became libraries whose two diagnostics pointed at each other:
//! `run` said to use `kanso play`, and `play` said to use `kanso run`.

use std::process::Command;

/// Watched red on all forty: `run` answered "is a library — nothing to run".
#[test]
fn every_example_runs_by_the_verb_its_shape_asks_for() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/examples");
    let mut refused = Vec::new();
    let mut ran = 0;
    for entry in std::fs::read_dir(dir).expect("the examples are there") {
        let path = entry.expect("directory entry").path();
        if path.extension().is_none_or(|x| x != "kso") {
            continue;
        }
        let name = path.file_name().expect("kso files have names").to_string_lossy().to_string();
        let source = std::fs::read_to_string(&path).expect("the example reads");
        // definitions beside statements is the play door; statements alone is
        // an entry file, which `run` takes
        let declares =
            source.lines().any(|line| line.starts_with("fn ") || line.starts_with("type "));
        let verb = match declares {
            true => "play",
            false => "run",
        };
        let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .args([verb, &name])
            .env("KANSO_SEED", "2685821657736338717")
            .current_dir(dir)
            .output()
            .expect("kanso binary runs");
        ran += 1;
        if !output.status.success() {
            let said = String::from_utf8_lossy(&output.stderr).trim().to_string();
            refused.push(format!("  kanso {verb} {name}: {said}"));
        }
    }

    assert!(ran > 40, "the examples moved: only {ran} of them");
    assert!(refused.is_empty(), "examples no verb will run:\n{}", refused.join("\n"));
}
