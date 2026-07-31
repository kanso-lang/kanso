//! A field getter's internal name must never reach a reader.
//!
//! Reading a field is applying a function, and that function is named — today
//! by a prefix no source can spell. The name is an implementation detail, so
//! every diagnostic that mentions a function has to remember to translate it,
//! and each new diagnostic is a fresh chance to forget. Three sites decode it
//! now; there is no reason to expect three forever.
//!
//! Rather than patch the next one after it ships, this fails the whole class:
//! if the internal name appears in anything a reader sees, from any engine,
//! the build goes red. What it cannot do is prove the encoding is a good idea
//! — that is what making a getter's identity structural would settle.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The exact spelling a reader must never see, asked of the compiler rather
/// than written out here, so changing the encoding cannot leave this behind
/// looking for a string nothing produces any more.
fn internal() -> String {
    let name = kanso::ast::getter_name("age");
    name.strip_suffix("age").expect("a getter's name is built from its field").to_string()
}

fn corpus() -> Vec<PathBuf> {
    let mut found = Vec::new();
    for dir in ["examples", "tests/golden/micro", "tests/golden/runtime", "tests/golden/errors"] {
        let dir = root().join(dir);
        let mut here: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "kso"))
            .collect();
        here.sort();
        found.extend(here);
    }
    found
}

/// Everything the reader sees: both streams, from one engine.
fn seen(program: &Path, engine: &[&str]) -> String {
    let source = std::fs::read_to_string(program).unwrap_or_default();
    // `run` is the verb; a file's `pub play` is what `run` finds
    let verb = "run";
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg(verb)
        .arg(program.file_name().expect("programs have names"))
        .args(engine)
        .env("KANSO_SEED", "2685821657736338717")
        .current_dir(program.parent().expect("programs live in a directory"))
        .output()
        .expect("kanso runs");
    let mut text = String::from_utf8_lossy(&done.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&done.stderr));
    text
}

#[test]
fn no_program_in_the_corpus_shows_a_getter_its_internal_name() {
    let internal = internal();
    for program in corpus() {
        for engine in [&[][..], &["--interp"][..]] {
            let text = seen(&program, engine);
            assert!(
                !text.contains(&internal),
                "{} leaked `{internal}` on {engine:?}:\n{text}",
                program.display()
            );
        }
    }
}

/// The corpus says nothing about paths it does not walk, and the getter paths
/// are reached by failing rather than by succeeding. These are the three ways
/// a read can fail — no such field anywhere, a field this value lacks, and the
/// accessor used as a value — each of which names a function internally and
/// must not say so out loud.
#[test]
fn a_failed_field_read_names_the_field_and_not_the_function() {
    let internal = internal();
    let work = std::env::temp_dir().join("kanso-getter-identity");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).expect("a directory of its own");
    let cases = [
        (
            "no_such_field",
            "type point\n  x\n\npub play =\n  p = point 1\n  print \"{p.y}\"\n",
            "field `y`",
        ),
        (
            "wrong_record",
            "import \"std/list\"\n\ntype city\n  name\n\ntype user\n  age\n\n\
             pub play =\n  rows = [(city \"a\") (user 3)]\n  \
             print \"{list/to_list (list/map rows _.name)}\"\n",
            "has no field `name`",
        ),
        ("not_a_record", "pub play =\n  n = 5\n  print \"{n.name}\"\n", "field `name`"),
    ];
    for (name, source, expected) in cases {
        let program = work.join(format!("{name}.kso"));
        std::fs::write(&program, source).expect("the probe writes");
        let mut answers = Vec::new();
        for engine in [&[][..], &["--interp"][..]] {
            let text = seen(&program, engine);
            assert!(!text.contains(&internal), "{name} leaked `{internal}`:\n{text}");
            assert!(
                text.contains(expected),
                "{name} stopped reporting `{expected}`, so this probe no longer \
                 reaches the getter path it was written for:\n{text}"
            );
            answers.push(text);
        }
        // the differential law, on the paths this file exists to cover
        assert_eq!(answers[0], answers[1], "{name}: the engines disagree");
    }
}
