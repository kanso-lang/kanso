//! The emitted-code gate counts what the compiler WROTE. Its `calls` column
//! was a bare `grep -c 'call '` over the whole .ll, and the prelude's comments
//! use the word eight times — "a call into glibc's memcpy", "a real call on
//! every `if` condition" — so rewording a comment moved the counter with the
//! emitted code byte-identical. That is the one thing the gate exists to rule
//! out, so the property gets a spec of its own.
//!
//! The counting text is lifted out of scripts/gates/emitted_code.sh rather
//! than copied here: a spec that restates the pipeline goes green while the
//! gate it stands for regresses.

use std::io::Write;
use std::process::{Command, Stdio};

/// The two lines of the gate that decide the decoder's `calls` column: the
/// comment stripper's definition, and the pipeline that reads it.
fn counting_text(script: &str) -> (String, String) {
    let define = script
        .lines()
        .find(|l| l.starts_with("strip_comments()"))
        .expect("the gate defines strip_comments");
    let count = script
        .lines()
        .map(str::trim)
        .find(|l| l.contains("calls=") && l.contains("grep -c"))
        .expect("the gate counts calls with grep");
    (define.to_string(), count.to_string())
}

#[test]
fn a_comment_that_says_call_is_not_an_emitted_call() {
    let script = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/gates/emitted_code.sh"),
    )
    .expect("the gate is readable");
    let (define, count) = counting_text(&script);

    let dir = std::env::temp_dir().join(format!("kanso-call-counter-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch directory");
    let ll = dir.join("jsonbench.ll");
    std::fs::write(
        &ll,
        "; the prelude spends a call into glibc's memcpy before it moves a byte\n\
         ; and a real call on every `if` condition, which is prose, not code\n\
         define i64 @f() {\n  \
         %r = call i64 @g()\n  \
         ret i64 %r\n\
         }\n",
    )
    .expect("fixture writes");

    // The pipeline names jsonbench.ll, so it runs in a directory holding one.
    let mut sh = Command::new("sh")
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("sh runs");
    write!(sh.stdin.as_mut().expect("stdin"), "{define}\n{count}\n").expect("script writes");
    let out = sh.wait_with_output().expect("sh finishes");
    let got = String::from_utf8_lossy(&out.stdout).trim().to_string();

    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(
        got, "calls=1",
        "two comments mention a call and the module emits one. \
         the gate counted the prose: {got}"
    );
}
