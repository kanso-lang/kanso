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
        .args(["run", written(name, source).to_str().expect("utf-8"), "--interp"])
        .output()
        .expect("kanso runs")
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// The plain case: fix the first argument, hand the result around, finish it
/// somewhere else.
#[test]
fn a_partial_carries_its_argument_to_wherever_it_is_finished() {
    let out = interp(
        "carry",
        "fn add a b\n  a + b\n\nfn apply_five f\n  f 5\n\npub play = print \"{apply_five (&add 2)}\"\n",
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
        "fn add a b c\n  a + b + c\n\nfn foo f\n  &f 2\n\npub play = print \"{(foo add) 5 7}\"\n",
    );

    assert_eq!(stdout(&out), "14");
}

/// Short of every arity it stays a partial rather than dispatching early, so
/// the arguments can arrive in more than one step.
#[test]
fn a_partial_grows_until_an_arity_matches() {
    let out = interp(
        "grows",
        "fn add a b c\n  a + b + c\n\nfn half f\n  f 3\n\npub play = print \"{(half (&add 1)) 6}\"\n",
    );

    assert_eq!(stdout(&out), "10");
}

/// Arity picks the group before patterns pick the arm, so a partial completes
/// against whichever arity its argument count reaches.
#[test]
fn a_partial_completes_against_the_arity_its_count_reaches() {
    let out = interp(
        "arity",
        "pub play = print \"{(&roll 4) 5}\"\n\nfn roll n\n  n + 1\n\nfn roll n sides\n  n * sides\n",
    );

    assert_eq!(stdout(&out), "20");
}

/// Past every arity is the error, and it says which arities existed.
#[test]
fn too_many_arguments_names_the_arities_that_exist() {
    let out = interp("over", "fn add a b\n  a + b\n\npub play = print \"{(&add 1) 2 3}\"\n");

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("arms take 2"), "diagnostic was: {stderr}");
}

/// The differential law's escape hatch: an engine may cover less, but it says
/// so. Silence here would be a program that prints one thing natively and
/// another in the interpreter.
#[test]
fn the_native_backend_declines_a_partial_out_loud() {
    let program = written(
        "declined",
        "fn add a b\n  a + b\n\nfn apply_five f\n  f 5\n\npub play = print \"{apply_five (&add 2)}\"\n",
    );

    let out = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["build", program.to_str().expect("utf-8")])
        .current_dir(std::env::temp_dir())
        .output()
        .expect("kanso runs");

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "native silently accepted a partial");
    assert!(stderr.contains("partial application"), "diagnostic was: {stderr}");
}
