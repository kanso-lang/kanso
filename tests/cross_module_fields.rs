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

/// A register-returnable record comes back in two registers rather than as an
/// ordinary value, and every consumer downstream reads an ordinary one. Render
/// it, read a field of it, and hand it to a function that destructures it —
/// the three shapes that used to emit invalid IR or, worse, a number.
#[test]
fn a_record_returned_in_registers_survives_every_consumer() {
    for engine in ENGINES {
        let (out, err) = run("returned", engine);
        assert_eq!(
            out, "lib/pair 6 \"v\"\n6\n6/v\n2\ntrue\n1\n",
            "{engine:?} mishandled the returned record: {err}"
        );
    }
}

/// The same two words also encode a failure, so nothing downstream may treat
/// one as a record. Native used to read fields off it and print a number.
#[test]
fn a_failure_in_the_register_convention_stays_a_failure() {
    let mut answers = Vec::new();
    for engine in ENGINES {
        let (out, err) = run("failed", engine);
        assert!(out.is_empty(), "{engine:?} printed something for a failure: {out:?}");
        assert!(
            err.contains("unhandled err reached the entry: \"empty\""),
            "{engine:?} lost the failure: {err:?}"
        );
        answers.push(err);
    }
    assert_eq!(answers[0], answers[1], "the engines disagree on the failure");
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
    for fixture in ["reads", "missing", "returned", "failed"] {
        for engine in ENGINES {
            let (out, err) = run(fixture, engine);
            assert!(
                !out.contains(&internal) && !err.contains(&internal),
                "{fixture} leaked `{internal}` on {engine:?}: out={out:?} err={err:?}"
            );
        }
    }
}
