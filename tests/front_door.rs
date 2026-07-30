//! What the toolchain says to somebody who has not used it yet.
//!
//! Asking for help is not a mistake, so it answers on stdout and exits clean.
//! `kanso --help | less` showed nothing when help went to stderr, which is the
//! first thing anybody tries. And a bare usage dump makes the reader diff it
//! against what they typed, so what was wrong comes before what to do.

use std::process::Command;

fn kanso(args: &[&str]) -> (String, String, Option<i32>) {
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("kanso runs");
    (
        String::from_utf8_lossy(&done.stdout).into_owned(),
        String::from_utf8_lossy(&done.stderr).into_owned(),
        done.status.code(),
    )
}

#[test]
fn help_is_an_answer_rather_than_a_failure() {
    for asked in [&["--help"], &["-h"], &["help"]] {
        let (out, err, code) = kanso(asked);
        assert!(out.starts_with("usage: kanso <verb>"), "{asked:?} said {out:?}");
        assert!(out.contains("install <dir>"), "{asked:?} lists no install: {out}");
        assert_eq!(err, "", "{asked:?} wrote to stderr");
        assert_eq!(code, Some(0), "{asked:?} did not exit clean");
    }
}

#[test]
fn a_mistake_is_named_before_the_usage() {
    let cases: [(&[&str], &str); 3] = [
        (&[], "kanso: no verb given"),
        (&["nonsense"], "kanso: `nonsense` is not a verb"),
        (&["check"], "kanso: `check` wants a file or directory"),
    ];
    for (args, said) in cases {
        let (out, err, code) = kanso(args);
        assert_eq!(err.lines().next(), Some(said), "{args:?} said {err:?}");
        assert!(err.contains("usage: kanso <verb>"), "{args:?} gave no usage: {err}");
        assert_eq!(out, "", "{args:?} wrote to stdout");
        assert_eq!(code, Some(2), "{args:?} did not exit 2");
    }
}
