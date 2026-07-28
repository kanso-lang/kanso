//! The playground's examples are the language's shop window, and a visitor
//! runs them on whichever engine the tab picked. These read the samples out of
//! docs/play.js rather than a list kept here, so a new example cannot ship
//! without coverage.

use std::path::PathBuf;
use std::process::Command;

/// Every `name: `...`` entry of play.js's EXAMPLES object.
fn examples() -> Vec<(String, String)> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(manifest.join("docs/play.js")).expect("play.js reads");
    let start = source.find("const EXAMPLES = {").expect("play.js declares EXAMPLES");
    let mut found = Vec::new();
    let mut rest = &source[start..];
    while let Some(at) = rest.find(": `") {
        let name: String = rest[..at]
            .chars()
            .rev()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let body = &rest[at + 3..];
        let end = body.find('`').expect("every example literal closes");
        found.push((name, unescape_template(&body[..end])));
        rest = &body[end..];
    }
    assert!(found.len() >= 9, "expected the playground's examples, found {}", found.len());
    found
}

/// What the browser receives, not what the file holds: a template
/// literal's backslash escapes are resolved by JavaScript before the source
/// reaches the compiler, so testing the raw text would test a program no
/// visitor runs.
fn unescape_template(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some(next @ ('\\' | '`' | '$')) => out.push(next),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            },
            other => out.push(other),
        }
    }
    out
}

fn written(name: &str, source: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("kanso-playground-test");
    std::fs::create_dir_all(&dir).expect("temp work dir");
    let file = dir.join(format!("{name}.kso"));
    std::fs::write(&file, source).expect("example writes");
    file
}

/// `random` draws from entropy unless KANSO_SEED pins the stream, and the
/// concurrency example rolls dice — without a seed the engines cannot be
/// compared at all, only observed disagreeing.
fn play(file: &PathBuf, engine: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(file)
        .args(engine)
        .env("KANSO_SEED", "2685821657736338717")
        .current_dir(std::env::temp_dir())
        .output()
        .expect("kanso run runs")
}

/// The interpreter is the oracle, so an example it cannot run is broken
/// outright — this is the failure the playground shows a visitor first.
#[test]
fn every_playground_example_runs_on_the_interpreter() {
    for (name, source) in examples() {
        let file = written(&name, &source);

        let run = play(&file, &["--interp"]);

        assert!(
            run.status.success(),
            "the interpreter failed on the {name} example: {}",
            String::from_utf8_lossy(&run.stderr)
        );
    }
}

/// The differential law: an engine that speaks a feature is byte-identical on
/// it. An engine may decline what it does not lower yet, but it says so with a
/// diagnostic and a failing status rather than printing something else.
#[test]
fn every_playground_example_agrees_between_the_interpreter_and_native() {
    for (name, source) in examples() {
        let file = written(&name, &source);
        let oracle = play(&file, &["--interp"]);

        let native = play(&file, &[]);

        match native.status.success() {
            true => assert_eq!(
                String::from_utf8_lossy(&native.stdout),
                String::from_utf8_lossy(&oracle.stdout),
                "native output diverges from the interpreter on the {name} example"
            ),
            false => assert!(
                !native.stderr.is_empty(),
                "native declined the {name} example without saying why"
            ),
        }
    }
}

/// The playground compiles each example to a wasm module and runs it in the
/// tab, falling back to the interpreter when the backend does not cover the
/// program. Either is fine; a panic or a module the encoder cannot finish is
/// not. Behaviour inside the browser needs a wasm host, which this suite has
/// no way to reach — see the browser-differential thread in the compiler log.
#[test]
fn every_playground_example_survives_the_browser_backend() {
    for (name, source) in examples() {
        let program = kanso::compile_source("run", &format!("{name}.kso"), &source)
            .unwrap_or_else(|e| panic!("the {name} example must compile: {e}"));

        let emitted = kanso::wasm_backend::compile(&program, false);

        if let Ok(module) = emitted {
            assert!(
                !module.bytes.is_empty(),
                "the browser backend accepted the {name} example and emitted nothing"
            );
        }
    }
}
