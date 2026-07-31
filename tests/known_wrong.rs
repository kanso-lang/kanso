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

/// Task #57. A walk over a list where each step is an effect, accumulating
/// conditionally — one arm returns the accumulator unchanged, the other pushes
/// a value derived from the step's answer — runs the native engine out of
/// stack. The interpreter, which is the oracle, answers correctly.
///
/// What is isolated: two NESTED binds are needed (one bind runs fine), the
/// accumulated value must be derived rather than constant, and the failing
/// step must be followed by another step. Neither records nor processes nor
/// files are involved. `--strict` fails the same way, so it is not thunk
/// memoization, and breaking the continuation lambda into named functions
/// makes it disappear, so it is the emitted shape rather than the semantics.
///
/// What is NOT known: why. The reduction stops here.
#[test]
fn native_runs_out_of_stack_where_the_interpreter_answers() {
    let (_, err, code) = run("push_through_nested_binds.kso", &[]);
    assert!(
        err.contains("the program ran out of stack"),
        "native no longer overflows — task #57 is fixed, delete this test: {err}"
    );
    assert_eq!(code, Some(1));

    let (out, err, code) = run("push_through_nested_binds.kso", &["--interp"]);
    assert_eq!(err, "");
    assert_eq!(out, "1: a gave a\n", "the oracle's answer changed");
    assert_eq!(code, Some(0));
}

/// Task #70. Two calls into the same three-deep chain of tail calls, differing
/// only in the literal that picks an arm at the bottom. The interpreter walks
/// it iteratively and always answers. The native build recurses, and whether
/// that reaches the end of the stack depends on the host: it dies on macOS and
/// arm, and finishes on the linux runner. So this pins the interpreter's answer
/// and accepts either native outcome, because asserting the one this laptop
/// gives would go red on a machine where the defect is merely latent.
#[test]
fn a_chain_of_tail_calls_recurses_on_native_where_the_oracle_loops() {
    let fixture = "tail_chain_recurses_on_native.kso";
    let expected = "[differ kanso p 9 ms\ndiffer kanso p 9 mb\n]\n";
    let (native_out, native_err, _) = run(fixture, &[]);
    let (interp_out, _, interp_code) = run(fixture, &["--interp"]);

    assert_eq!(interp_out, expected);
    assert_eq!(interp_code, Some(0));
    assert!(
        native_err.contains("ran out of stack") || native_out == expected,
        "native neither answered nor ran out of stack: out={native_out:?} err={native_err:?}"
    );
}
