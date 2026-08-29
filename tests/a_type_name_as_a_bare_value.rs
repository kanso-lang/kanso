//! A type's name standing alone, with no arguments after it.
//!
//! Three shapes reach this, and until now they got three different answers
//! from the three engines. A record type with fields is refused by native as
//! a limit of its own; a record type with NO fields is a value on every
//! engine, because naming the one thing it describes builds it. A subtype and
//! a typeset carry no fields either, and the emitter's test was `fields is
//! empty`, so both were emitted as nullary records — `print "{age}"` for
//! `type age int` printed `<mod>/age` where the oracle prints `<fn>` and the
//! page refuses the name outright.
//!
//! Native says its limit now, which is the only way the differential law lets
//! a feature live on fewer engines.

use std::path::PathBuf;
use std::process::Command;

fn written(name: &str, source: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("kanso-bare-type-name-test");
    std::fs::create_dir_all(&dir).expect("temp work dir");
    let file = dir.join(format!("{name}.kso"));
    std::fs::write(&file, source).expect("program writes");
    file
}

fn run(name: &str, source: &str, extra: &[&str]) -> std::process::Output {
    let path = written(name, source);
    let mut args = vec!["play", path.to_str().expect("utf-8")];
    args.extend_from_slice(extra);
    Command::new(env!("CARGO_BIN_EXE_kanso")).args(args).output().expect("kanso runs")
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).trim().to_string()
}

const SUBTYPE: &str = "type age int\n\nprint \"{age}\"\n";
const NULLARY: &str = "type unit\n\nprint \"{unit}\"\n";
const RECORD: &str = "type point\n  x\n  y\n\nprint \"{point}\"\n";

/// The defect. Native emitted a nullary record for a subtype's name and
/// printed it, where the oracle hands back the constructor as a function
/// value. Two engines, two answers, neither an error.
#[test]
fn native_declines_a_bare_subtype_name_instead_of_building_a_record() {
    let said = stderr(&run("subtype_native", SUBTYPE, &[]));

    assert!(
        said.contains("`age` as a bare value is not yet supported"),
        "the backend did not name its own limit: {said}"
    );
    assert!(!said.contains("age\n"), "the backend still rendered the name: {said}");
    // The oracle runs it, which is what makes the refusal a limit rather than
    // a judgement about the program.
    assert_eq!(stdout(&run("subtype_oracle", SUBTYPE, &["--interp"])), "<fn>");
}

/// The control the fix must not break: a record type with no fields names one
/// thing, and naming it builds it, on both engines.
#[test]
fn a_record_type_with_no_fields_is_still_a_value_on_both_engines() {
    assert_eq!(stdout(&run("nullary_native", NULLARY, &[])), "unit");
    assert_eq!(stdout(&run("nullary_oracle", NULLARY, &["--interp"])), "unit");
}

/// The neighbour that was already right, and the sentence the subtype now
/// borrows: a record type WITH fields has always been declined out loud.
#[test]
fn native_already_declined_a_bare_record_name_the_same_way() {
    let said = stderr(&run("record_native", RECORD, &[]));

    assert!(
        said.contains("`point` as a bare value is not yet supported"),
        "the backend's older limit moved: {said}"
    );
    assert_eq!(stdout(&run("record_oracle", RECORD, &["--interp"])), "<fn>");
}
