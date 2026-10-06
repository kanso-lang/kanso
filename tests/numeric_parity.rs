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

/// Above 2^63 the engines agree. The interpreter was always arbitrary
/// precision; the native build refused here until the 2026-10-07 gavel, and
/// this test said so and was written to fail the day the gap closed (task
/// #46). It now pins the agreement.
#[test]
fn above_the_machine_word_the_engines_agree() {
    let program = "print (9223372036854775807 * 2)\n";

    let (native_code, native, native_err) = ran(program, &[]);
    let (_, interp, _) = ran(program, &["--interp"]);

    assert_eq!(interp, "18446744073709551614\n", "the oracle stopped being exact");
    assert_eq!(native, interp, "native disagrees with the oracle: {native_err}");
    assert_eq!(native_code, 0, "native failed: {native_err}");
}

/// A literal wider than the native payload used to be truncated into it, so
/// `1 * 18446744073709551616` printed 0 and `0 % 18446744073709551616` said
/// "modulo by zero". Found by scripts/numeric_differential.py on its first
/// run; the native build then refused the literal, and since the 2026-10-07
/// gavel it reads it as a bignum.
#[test]
fn a_literal_wider_than_a_word_is_read_whole() {
    for (program, want) in [
        ("print (1 * 18446744073709551616)\n", "18446744073709551616\n"),
        ("print (1 + 9223372036854775808)\n", "9223372036854775809\n"),
    ] {
        let (code, out, err) = ran(program, &[]);
        let (_, interp, _) = ran(program, &["--interp"]);

        assert_eq!(interp, want, "the oracle stopped being exact");
        assert_eq!(out, want, "native disagrees with the oracle: {err}");
        assert_eq!(code, 0, "native failed: {err}");
    }
}

/// The only signed division of two words whose quotient is not a word: the
/// least integer over -1 is one past the greatest. C leaves it undefined and
/// the machine answered by wrapping, so native once printed the least
/// integer again with exit 0. Found by classifying what
/// scripts/numeric_differential.py reported. The quotient is a bignum now.
#[test]
fn the_least_integer_divided_by_minus_one_is_one_past_the_greatest() {
    let program = "print (-9223372036854775808 / -1)\n";

    let (code, out, err) = ran(program, &[]);
    let (_, interp, _) = ran(program, &["--interp"]);

    assert_eq!(interp, "9223372036854775808\n", "the oracle stopped being exact");
    assert_eq!(out, interp, "native disagrees with the oracle: {err}");
    assert_eq!(code, 0, "native failed: {err}");
}

/// The least integer modulo -1 is zero, and zero fits — but the quotient it is
/// computed from does not, and x86's division traps on the pair. ARM answers
/// without trapping, so this crashed with SIGFPE only on linux, where nobody
/// was looking: the gate below found it on its first CI run.
///
#[test]
fn the_least_integer_modulo_minus_one_is_zero_on_both_engines() {
    let program = "print (-9223372036854775808 % -1)\n";

    let (code, out, err) = ran(program, &[]);
    let (_, interp, _) = ran(program, &["--interp"]);

    assert_eq!(interp, "0\n", "the oracle stopped being exact");
    assert_eq!(out, interp, "native disagrees with the oracle: {err}");
    assert_eq!(code, 0, "native failed: {err}");
}
