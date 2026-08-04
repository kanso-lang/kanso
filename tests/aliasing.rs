//! In-place mutation is only ever safe on a value nobody else holds, so the
//! cases here hand the same accumulator to two consumers and check that the
//! second one sees what it was given. The interpreter never mutates in place,
//! which makes it the oracle for exactly this: a builder written through one
//! of two owners shows up as a byte difference between the engines and
//! nowhere else.

use std::path::PathBuf;
use std::process::Command;

fn written(name: &str, source: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("kanso-aliasing-test");
    std::fs::create_dir_all(&dir).expect("temp work dir");
    let file = dir.join(format!("{name}.kso"));
    std::fs::write(&file, source).expect("program writes");
    file
}

fn out(args: &[&str]) -> String {
    let result = Command::new(env!("CARGO_BIN_EXE_kanso")).args(args).output().expect("kanso runs");
    assert!(result.status.success(), "kanso failed: {}", String::from_utf8_lossy(&result.stderr));
    String::from_utf8_lossy(&result.stdout).trim().to_string()
}

fn both_engines(name: &str, source: &str) -> (String, String) {
    let program = written(name, source);
    let path = program.to_str().expect("utf-8");
    (out(&["play", path, "--interp"]), out(&["play", path]))
}

/// An imported name is enrolled twice — once qualified, once bare — and the
/// two declarations share a span. The bare clone is called by nobody, so it
/// passes every call-site test by default; if it is allowed to vote, it wins
/// its twin a parameter the real declaration was refused, and the shared span
/// carries that verdict into the emitted code.
#[test]
fn an_imported_builder_held_twice_is_not_written_through() {
    let (interpreted, native) = both_engines(
        "imported_alias",
        "import \"std/text\"\n\nbase = text/append (text/bytes \"\") \"A\"\nx = text/append base \"B\"\ny = text/append base \"C\"\nprint \"{text/utf8 x} {text/utf8 y}\"\n",
    );

    assert_eq!(native, interpreted, "native mutated a builder the interpreter copied");
    assert_eq!(native, "AB AC");
}

/// The same aliasing one call deeper, where the second owner is reached
/// through a wrapper rather than named directly.
#[test]
fn a_builder_passed_twice_through_a_wrapper_is_not_written_through() {
    let (interpreted, native) = both_engines(
        "wrapped_alias",
        "import \"std/text\"\n\nfn addb acc b\n  text/append acc b\n\nbase = text/append (text/bytes \"\") \"A\"\nx = addb base \"B\"\ny = addb base \"C\"\nprint \"{text/utf8 x} {text/utf8 y}\"\n",
    );

    assert_eq!(native, interpreted, "native mutated a builder the interpreter copied");
    assert_eq!(native, "AB AC");
}

/// The list half of the same rule, which reaches in-place pushes through a
/// different path and was already sound — it holds the fix to the shape of
/// the bug rather than to the one name that exposed it.
#[test]
fn a_list_held_twice_is_not_pushed_into() {
    let (interpreted, native) = both_engines(
        "list_alias",
        "base = push [] 1\nx = push base 2\ny = push base 3\nprint \"{length x} {length y}\"\n",
    );

    assert_eq!(native, interpreted);
    assert_eq!(native, "2 2");
}

/// The gain the analysis exists for has to survive the fix: an accumulator
/// with one owner is still extended in place, so the guard above is narrow
/// rather than a retreat from mutating at all.
#[test]
fn a_singly_owned_builder_is_still_written_through() {
    // a real program, because only a real program builds: the grower is a
    // library file and the statements that drive it are the entry beside it
    let dir = std::env::temp_dir().join("kanso-aliasing-build/single_owner");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("grower")).expect("temp work dir");
    std::fs::write(
        dir.join("grower/grower.kso"),
        "import \"std/text\"\n\npub fn grow acc b\n  text/append acc b\n",
    )
    .expect("library writes");
    std::fs::write(
        dir.join("main.kso"),
        "import \"grower\"\nimport \"std/text\"\n\nout = grower/grow (grower/grow (text/bytes \"\") \"A\") \"B\"\nprint \"{text/utf8 out}\"\n",
    )
    .expect("entry writes");
    // built from its own output directory: a program's binary takes the
    // program's name, which would collide with the program's directory
    let work = std::env::temp_dir().join("kanso-aliasing-build/out");
    std::fs::create_dir_all(&work).expect("temp work dir");
    let built = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["build", dir.to_str().expect("utf-8"), "--release"])
        .current_dir(&work)
        .output()
        .expect("kanso build runs");
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    let ir = std::fs::read_to_string(work.join("single_owner.ll")).expect("emitted IR reads");

    assert!(
        ir.contains("@k_b_append_mut"),
        "no append site is extended in place any more — the uniqueness analysis \
         stopped paying for itself"
    );
}

/// The map half of the same rule: `put` extends in place only where the map
/// is moved, so one held by two owners keeps its second owner's view intact.
#[test]
fn a_map_held_twice_is_not_written_through() {
    let (interpreted, native) = both_engines(
        "map_alias",
        "base = put (put {:} \"a\" 1) \"b\" 2\nx = put base \"c\" 3\ny = put base \"d\" 4\nprint \"{length x} {length y}\"\n",
    );

    assert_eq!(native, interpreted);
    assert_eq!(native, "3 3");
}

/// The chain license is identity, never type: a loop whose crossing bytes
/// value is born fresh each iteration has its header above the mark, so a
/// rewind would hand the next iteration a dangling accumulator. The license
/// must refuse it, and both engines must agree on the output.
#[test]
fn a_fresh_builder_each_iteration_is_not_licensed() {
    let (interpreted, native) = both_engines(
        "fresh_builder",
        "import \"std/text\"\n\nfn churn acc 0\n  text/utf8 acc\n\nfn churn acc n\n  pad = \"{n}-{n}-{n}-{n}-{n}-{n}-{n}-{n}\"\n  churn (text/append (text/bytes (text/utf8 acc)) pad) (n - 1)\n\nseed = text/bytes \"go\"\nprint \"{length (churn seed 40)}\"\n",
    );

    assert_eq!(native, interpreted);
    assert_eq!(native, "850");
}
