//! Defects that are real, reduced, and not yet fixed.
//!
//! Recorded the way tests/golden/wasm_gaps.txt records an engine's declines: a
//! defect written down stops being a surprise, and a defect that quietly
//! changes shape is how it survives a rewrite. Each test here asserts what the
//! compiler DOES, never what it should — so it goes red the moment somebody
//! fixes the thing, which is the reminder to delete the entry rather than
//! update it.

use std::process::Command;

fn run(fixture: &str, engine: &[&str]) -> (String, String, Option<i32>) {
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(format!("tests/golden/known_wrong/{fixture}"))
        .args(engine)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("kanso runs");
    (
        String::from_utf8_lossy(&done.stdout).into_owned(),
        String::from_utf8_lossy(&done.stderr).into_owned(),
        done.status.code(),
    )
}

/// Task #72. An effect handed to a parameter nobody reads never happens, and
/// nothing says so. Laziness means an argument that is never forced is never
/// evaluated, so the write is not performed — the same shape as Haskell's
/// `const "x" (putStrLn "y")`.
///
/// It is recorded rather than accepted because the language already refuses
/// the neighbouring mistake: a function whose body does io must hand the io
/// back, or it "would abandon the effects above it". That check reads a
/// function's own body, so this one walks past it.
#[test]
fn an_effect_handed_to_a_parameter_nobody_reads_never_happens() {
    let fixture = "an_effect_handed_over_and_ignored.kso";
    let dropped = "dropped it\n";

    let (native, native_err, native_code) = run(fixture, &[]);
    let (interp, _, interp_code) = run(fixture, &["--interp"]);

    assert_eq!(native, dropped, "native performed the write — task #72 is fixed, delete this");
    assert_eq!(interp, dropped, "the oracle performed the write — task #72 is fixed, delete this");
    assert_eq!(native_err, "", "a diagnostic appeared: {native_err}");
    assert_eq!(native_code, Some(0));
    assert_eq!(interp_code, Some(0));
}
