//! Reading a field of a type another module declared.
//!
//! Accessors are functions, and a function has to exist where it is called.
//! A module that hands out a record but never reads one of its own fields
//! used to prune every getter its importers needed, so field reads worked
//! only inside the module that declared the type — which is to say, no
//! library could expose a record anyone could read.
//!
//! Both engines run every case here, because the three defects behind it were
//! found by the engines disagreeing: one answered a number where the other
//! reported a field error.

use std::process::Command;

fn run(fixture: &str, engine: &[&str]) -> (String, String) {
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(format!("tests/golden/fields/{fixture}"))
        .args(engine)
        .env("KANSO_SEED", "2685821657736338717")
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
fn a_field_of_an_imported_record_reads() {
    for engine in ENGINES {
        let (out, err) = run("reads", engine);
        assert_eq!(out, "3 4\n", "{engine:?} did not read the imported record: {err}");
    }
}

/// The other half: a record that lacks the field must say so. Native used to
/// answer a number here — it took the by-value calling convention on trust
/// and read two words out of a record holding one.
#[test]
fn a_field_the_record_lacks_is_reported_not_invented() {
    let mut answers = Vec::new();
    for engine in ENGINES {
        let (out, err) = run("missing", engine);
        assert!(
            err.contains("`geo/label` has no field `x`"),
            "{engine:?} did not report the missing field: out={out:?} err={err:?}"
        );
        assert!(out.is_empty(), "{engine:?} printed something anyway: {out:?}");
        answers.push(err);
    }
    assert_eq!(answers[0], answers[1], "the engines disagree on a missing field");
}

/// The corpus had no program that read a field across a module boundary, so
/// the guard in getter_identity.rs could not see this path — and this is the
/// path where the internal name is most likely to escape, because it is the
/// one the resolver and the dispatcher both touch.
#[test]
fn neither_engine_shows_the_getter_its_internal_name() {
    let internal = {
        let name = kanso::ast::getter_name("x");
        name.strip_suffix('x').expect("a getter is named from its field").to_string()
    };
    for fixture in ["reads", "missing"] {
        for engine in ENGINES {
            let (out, err) = run(fixture, engine);
            assert!(
                !out.contains(&internal) && !err.contains(&internal),
                "{fixture} leaked `{internal}` on {engine:?}: out={out:?} err={err:?}"
            );
        }
    }
}
