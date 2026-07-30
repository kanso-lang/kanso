use std::process::Command;

fn ran(program: &str, extra: &[&str]) -> (i32, String, String) {
    // one directory per program and engine: these run in parallel, and a
    // shared work dir means one test deletes another's entry mid-run
    let key: String = program.chars().filter(|c| c.is_ascii_digit()).take(12).collect();
    let dir = std::env::temp_dir().join(format!("kanso-numeric-{key}-{}", extra.len()));
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
