//! `&f` — partial application. The interpreter is the oracle; the two
//! backends decline it out loud, which is the only way the differential law
//! permits a feature to live on fewer engines.

use std::path::PathBuf;
use std::process::Command;

fn written(name: &str, source: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("kanso-partial-test");
    std::fs::create_dir_all(&dir).expect("temp work dir");
    let file = dir.join(format!("{name}.kso"));
    std::fs::write(&file, source).expect("program writes");
    file
}

fn interp(name: &str, source: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["play", written(name, source).to_str().expect("utf-8"), "--interp"])
        .output()
        .expect("kanso runs")
}

fn native(name: &str, source: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["play", written(name, source).to_str().expect("utf-8")])
        .output()
        .expect("kanso runs")
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).trim().to_string()
}

/// The plain case: fix the first argument, hand the result around, finish it
/// somewhere else.
#[test]
fn a_partial_carries_its_argument_to_wherever_it_is_finished() {
    let out = interp(
        "carry",
        "fn add a b\n  a + b\n\nfn apply_five f\n  f 5\n\nprint \"{apply_five (&add 2)}\"\n",
    );

    assert_eq!(stdout(&out), "7");
}

/// The reason `&` cannot take an arity: the callee here is a parameter, so the
/// arity is not knowable where the `&` is written. It resolves when the
/// arguments arrive.
#[test]
fn a_partial_of_a_parameter_finishes_at_the_call_site() {
    let out = interp(
        "of_param",
        "fn add a b c\n  a + b + c\n\nfn foo f\n  &f 2\n\nprint \"{(foo add) 5 7}\"\n",
    );

    assert_eq!(stdout(&out), "14");
}

/// "Decline it out loud" is the header's claim, and until this spec existed
/// nothing held the backend to it. The message it gave named the program's
/// fault — `&f` holds 1 argument(s), and no `f` takes more — for a program
/// with no fault: `f` is a parameter, and the arity the sentence asks after
/// is not knowable until the arguments arrive, which is the feature the spec
/// above specifies. A backend that covers less has to say so as a limit of
/// its own, or the reader goes looking for a mistake that is not there.
#[test]
fn the_native_backend_declines_a_partial_over_a_value_as_its_own_limit() {
    let program = "fn add a b c\n  a + b + c\n\nfn foo f\n  &f 2\n\nprint \"{(foo add) 5 7}\"\n";
    let said = stderr(&native("of_param_native", program));

    assert!(
        said.contains("`f` is a value here")
            && said.contains("settles its arity when its arguments arrive"),
        "the backend did not name its own limit: {said}"
    );
    assert!(
        !said.contains("takes more"),
        "the refusal still reads as the program's mistake: {said}"
    );
    // The oracle runs the same program, which is what makes the refusal a
    // limit rather than a judgement about the program.
    assert_eq!(stdout(&interp("of_param_oracle", program)), "14");
}

/// Short of every arity it stays a partial rather than dispatching early, so
/// the arguments can arrive in more than one step.
#[test]
fn a_partial_grows_until_an_arity_matches() {
    let out = interp(
        "grows",
        "fn add a b c\n  a + b + c\n\nfn half f\n  f 3\n\nprint \"{(half (&add 1)) 6}\"\n",
    );

    assert_eq!(stdout(&out), "10");
}

/// Arity picks the group before patterns pick the arm, so a partial completes
/// against whichever arity its argument count reaches.
#[test]
fn a_partial_completes_against_the_arity_its_count_reaches() {
    let out = interp(
        "arity",
        "print \"{(&roll 4) 5}\"\n\nfn roll n\n  n + 1\n\nfn roll n sides\n  n * sides\n",
    );

    assert_eq!(stdout(&out), "20");
}

/// Past every arity is the error, and it says which arities existed.
#[test]
fn too_many_arguments_names_the_arities_that_exist() {
    let out = interp("over", "fn add a b\n  a + b\n\nprint \"{(&add 1) 2 3}\"\n");

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("arms take 2"), "diagnostic was: {stderr}");
}

/// The differential law wants both engines to agree, not merely to coexist.
/// Native lowers a partial as the lambda it is equivalent to, so this runs the
/// same program through both and compares the bytes.
#[test]
fn native_and_the_interpreter_agree_on_a_partial() {
    let source =
        "fn add a b\n  a + b\n\nfn apply_five f\n  f 5\n\nprint \"{apply_five (&add 2)}\"\n";
    let program = written("agree", source);

    let native = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["play", program.to_str().expect("utf-8")])
        .output()
        .expect("kanso runs");

    assert_eq!(stdout(&native), stdout(&interp("agree_interp", source)));
    assert_eq!(stdout(&native), "7");
}

/// Dispatch happens where the arguments arrive, not at the `&`. With three
/// arms live, `(&roll 4) 5` reaches the two-argument one and `(&roll 4) 5 6`
/// reaches the three-argument one — the `&` chooses nothing, which is why
/// several live arms are not an ambiguity.
#[test]
fn a_partial_dispatches_on_the_total_count_not_at_the_ampersand() {
    let source = "fn roll n\n  n + 1\n\nfn roll n sides\n  n * sides\n\nfn roll n sides bonus\n  n * sides + bonus\n\nprint \"{(&roll 4) 5}\" >> print \"{(&roll 4) 5 6}\"\n";
    let program = written("total_count", source);

    let native = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["play", program.to_str().expect("utf-8")])
        .output()
        .expect("kanso runs");

    assert_eq!(stdout(&native), stdout(&interp("total_count_interp", source)));
    assert_eq!(stdout(&native), "20\n26");
}

/// Currying past every arm is the one thing nothing can finish.
#[test]
fn holding_more_arguments_than_any_arm_takes_is_refused() {
    let program = written("overheld", "fn add a b\n  a + b\n\nprint \"{(&add 1 2 3)}\"\n");

    let out = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["build", program.to_str().expect("utf-8")])
        .current_dir(std::env::temp_dir())
        .output()
        .expect("kanso runs");

    assert!(!out.status.success(), "a partial past every arity was accepted");
}
