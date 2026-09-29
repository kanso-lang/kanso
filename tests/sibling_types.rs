//! An annotation may name a type any file of its module declares.
//!
//! A module's files share their declarations: a function one file declares is
//! called from the next, and a record one file declares is built there. Until
//! 2026-09-28 an annotation was the exception. The checker asked only the
//! annotating file's own types, so `p:pt` with `pt` declared in the file beside
//! it was refused as naming no type, on every engine, because every engine
//! runs the same checker. Found by the generated-program differential, which
//! splits a program across two files of one module.

use std::process::Command;

fn run(fixture: &str, engine: &[&str]) -> (String, String) {
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(format!("tests/golden/sibling_types/{fixture}"))
        .args(engine)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("kanso runs");
    (
        String::from_utf8_lossy(&done.stdout).into_owned(),
        String::from_utf8_lossy(&done.stderr).into_owned(),
    )
}

const ENGINES: [&[&str]; 2] = [&[], &["--interp"]];

#[test]
fn an_annotation_names_a_type_the_next_file_declares() {
    for engine in ENGINES {
        let (out, err) = run("annotates", engine);
        assert_eq!(
            out, "[\"tag 2\" \"pt 3\" \"other\"]\n",
            "{engine:?} did not dispatch on the sibling file's types: {err}"
        );
    }
}

/// A knot is a constant whose constructor stores its own name, and a record
/// the next file declares constructs as well as one declared above it. Until
/// 2026-09-29 the cycle check asked only the file's own types, so all three
/// constants here were refused as defined in terms of themselves.
#[test]
fn a_knot_ties_through_a_record_the_next_file_declares() {
    for engine in ENGINES {
        let (out, err) = run("knots", engine);
        assert_eq!(
            out, "2 1 3 ring/node 1 ring/node 2 <cycle>\n",
            "{engine:?} did not tie the knots: {err}"
        );
    }
}

/// A cycle through two files is refused in each file, at each constant. The
/// module is checked as one program, and until 2026-09-29 the refusal named
/// the module and no location, so a reader had to search every file for two
/// names.
#[test]
fn a_cycle_across_files_is_refused_where_each_constant_is() {
    for engine in ENGINES {
        let (out, err) = run("cycle", engine);
        assert_eq!(out, "", "{engine:?} ran a program with no value");
        assert_eq!(
            err,
            "error[name]: `second` is defined in terms of itself, so it has no value \
             (module tests/golden/sibling_types/cycle/ring)\n  \
             --> tests/golden/sibling_types/cycle/ring/cycle.kso:1:1\n   \
             1 | second = first\n       ^\n\
             error[name]: `first` is defined in terms of itself, so it has no value \
             (module tests/golden/sibling_types/cycle/ring)\n  \
             --> tests/golden/sibling_types/cycle/ring/decls.kso:1:1\n   \
             1 | first = second\n       ^\n",
            "{engine:?} did not place the refusals"
        );
    }
}

/// An operator's arm keeps its bare name when its module is imported, so the
/// checker cannot tell it from the importer's own code by a slash in the name.
/// Until 2026-09-29 it could not tell at all, and refused the arm building its
/// own module's record as a construction across the import. Two shapes: the
/// arm sharing a file with a function the import prefixed, and the arm alone
/// in its file with the record declared in the other.
#[test]
fn an_operator_arm_builds_the_record_its_module_declares() {
    for fixture in ["arms", "arm_alone"] {
        for engine in ENGINES {
            let (out, err) = run(fixture, engine);
            assert_eq!(out, "sums/pt 4 \"ab\"\n", "{fixture} {engine:?} refused the arm: {err}");
        }
    }
}
