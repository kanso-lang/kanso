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

/// Task #70. A function of five parameters answers differently the second time
/// it is called with the same arguments: the first argument arrives holding the
/// second's value. Five KValues want ten argument registers, AArch64 has eight,
/// so two travel on the stack and a non-tail `call tailcc` gets that wrong.
/// Silent corruption, not a crash, and only on arm — the x86 runner answers
/// correctly, which is why nothing caught it. So this pins the interpreter and
/// names the one wrong answer arm is allowed to give.
#[test]
fn five_parameters_corrupt_the_second_call_on_arm() {
    let fixture = "five_parameters_corrupt_the_second_call.kso";
    let right = "[2 1 3 4 5][2 1 3 4 5]\n";
    let corrupt = "[2 1 3 4 5][2 2 3 4 5]\n";
    let (native_out, _, _) = run(fixture, &[]);
    let (interp_out, _, interp_code) = run(fixture, &["--interp"]);

    assert_eq!(interp_out, right);
    assert_eq!(interp_code, Some(0));
    assert!(
        native_out == right || native_out == corrupt,
        "native gave a third answer: {native_out:?}"
    );
    assert_eq!(
        native_out == right,
        !cfg!(target_arch = "aarch64"),
        "arm should corrupt and x86 should not; delete this entry when arm stops"
    );
}

/// Task #70, the same defect wearing a crash. A three-deep chain of tail calls
/// where the middle one takes five parameters: the corrupted argument is a
/// literal-cache pointer, so the runtime dereferences 5 and dies. `kanso run`
/// reports that as running out of stack, which it is not — the backtrace is
/// five frames deep and the call graph has no cycle in it. Whether it faults
/// at all is the host's answer, so this accepts either outcome.
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
        "native neither answered nor died: out={native_out:?} err={native_err:?}"
    );
}
