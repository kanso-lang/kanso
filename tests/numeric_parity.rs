use std::process::Command;

fn ran(program: &str, extra: &[&str]) -> (i32, String, String) {
    // One directory per program and engine: these run in parallel, and a
    // shared work dir means one test deletes another's entry mid-run. The key
    // is the whole program, hashed — twelve digits of it collided the moment a
    // fourth test arrived, because `9223372036854775807 * 2` and
    // `-9223372036854775808 / -1` open with the same twelve, and the two tests
    // then read each other's answers.
    use std::hash::{Hash, Hasher};
    let mut hasher = std::hash::DefaultHasher::new();
    program.hash(&mut hasher);
    extra.hash(&mut hasher);
    let dir = std::env::temp_dir().join(format!("kanso-numeric-{:016x}", hasher.finish()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp work dir");
    std::fs::write(dir.join("main.kso"), program).expect("the entry writes");
    let out = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(".")
        .args(extra)
        .current_dir(&dir)
        .output()
        .expect("kanso runs");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Arithmetic the two engines agree on, which is nearly all of it — including
/// the factorial the landing page advertises, since 20! is under 2^63.
#[test]
fn the_engines_agree_below_the_native_ceiling() {
    let program = "print (2432902008176640000 + 1)\n";

    let (_, native, _) = ran(program, &[]);
    let (_, interp, _) = ran(program, &["--interp"]);

    assert_eq!(native, "2432902008176640001\n");
    assert_eq!(interp, native, "the engines disagree below the ceiling");
}

/// KNOWN GAP, pinned rather than fixed (task #46). Above 2^63 the engines
/// disagree: the interpreter is arbitrary-precision, as the spec says, and the
/// native build refuses rather than wrapping. Nothing in the golden corpus
/// crosses this line, so neither answer was pinned and a change to either
/// would have gone unnoticed. This test exists so that stops being true — when
/// the gap is closed, it is what fails and tells you to delete it.
#[test]
fn above_the_native_ceiling_the_engines_still_disagree() {
    let program = "print (9223372036854775807 * 2)\n";

    let (native_code, _, native_err) = ran(program, &[]);
    let (_, interp, _) = ran(program, &["--interp"]);

    assert_eq!(interp, "18446744073709551614\n", "the oracle stopped being exact");
    assert_ne!(native_code, 0, "native now succeeds — the gap may be closed; see #46");
    assert!(
        native_err.contains("integer overflow"),
        "native failed for some other reason: {native_err}"
    );
}

/// A literal wider than the native payload used to be truncated into it, so
/// `1 * 18446744073709551616` printed 0 and `0 % 18446744073709551616` said
/// "modulo by zero" — silent wrong answers, where the whole point of this
/// build's ceiling is to refuse rather than produce one. Found by
/// scripts/numeric_differential.py on its first run.
#[test]
fn a_literal_too_wide_for_the_native_build_is_refused_not_wrapped() {
    for (program, was) in [
        ("print (1 * 18446744073709551616)\n", "0"),
        ("print (1 + 9223372036854775808)\n", "-9223372036854775807"),
    ] {
        let (code, out, err) = ran(program, &[]);

        assert_ne!(code, 0, "native accepted a literal it cannot hold: {out}");
        assert!(
            err.contains("does not fit this build's 64-bit int"),
            "refused for the wrong reason: {err}"
        );
        assert_ne!(out.trim(), was, "the old wrapped answer came back");
    }
}

/// The only signed division that overflows: the least integer over -1 is one
/// past the greatest. C leaves it undefined and this machine answered by
/// wrapping, so native printed the least integer again — a wrong number, with
/// exit 0, where every other overflow on this build is a refusal.
///
/// Found by classifying what scripts/numeric_differential.py reports: of 417
/// disagreements, 396 were literals too wide to compile and 164 were the loud
/// runtime overflow. This was the one left over.
#[test]
fn the_least_integer_divided_by_minus_one_is_refused_not_wrapped() {
    let program = "print (-9223372036854775808 / -1)\n";

    let (code, out, err) = ran(program, &[]);
    let (_, interp, _) = ran(program, &["--interp"]);

    assert_eq!(interp, "9223372036854775808\n", "the oracle stopped being exact");
    assert_ne!(code, 0, "native answered instead of refusing: {out:?}");
    assert!(err.contains("integer overflow"), "refused for another reason: {err}");
}
