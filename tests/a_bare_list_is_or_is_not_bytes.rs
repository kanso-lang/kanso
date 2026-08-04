//! Four builtins answer differently on the two engines for a list that was
//! never bytes, and three of them do worse than disagree about a diagnostic.
//!
//! The interpreter has no distinct bytes value — there is no `Value::Bytes` in
//! src/eval.rs — so bytes ARE a list of ints there. Native has a real `K_BYTES`
//! tag and refuses anything else. So `[97]` is bytes to one engine and a type
//! error to the other, and `text/find2 [97] 0 97 98` answers `1` under the
//! interpreter while native refuses to run it at all. A program can work on one
//! engine and be rejected by the other.
//!
//! GENUINE bytes agree on all four, which is what makes this narrow: the
//! divergence needs a list that was never bytes. Those agreements are asserted
//! here too, because a fix that made them diverge would be a worse trade than
//! the bug.
//!
//! Ignored, and not for the usual reason. This is not an acceptance criterion
//! for work that is merely unfinished — it cannot be written down until
//! somebody rules on whether kanso's bytes are a type or a convention. Way one
//! is native widening to accept a list of ints wherever it accepts bytes, which
//! says a list and bytes are interchangeable — the thing native's
//! representation exists to deny. Way two is the interpreter gaining a distinct
//! bytes value so it can refuse a list the way native does, which removes the
//! ambiguity at the root and touches every place the interpreter builds or
//! reads bytes. The two are not equivalent and the size of each is not the
//! argument. See design/compiler-log.md.

use std::process::Command;

/// What each engine says for one expression, as (native, interp).
fn both(expr: &str) -> (String, String) {
    let dir = std::env::temp_dir().join(format!("kanso-bytes-{}", expr.len()));
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    std::fs::write(dir.join("run.kso"), format!("import \"std/text\"\n\nprint \"{{{expr}}}\"\n"))
        .expect("the program writes");

    let say = |engine: &[&str]| {
        let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("play")
            .arg("run.kso")
            .args(engine)
            .current_dir(&dir)
            .output()
            .expect("kanso runs");
        let mut said = String::from_utf8_lossy(&done.stdout).into_owned();
        said.push_str(&String::from_utf8_lossy(&done.stderr));
        said
    };
    let out = (say(&[]), say(&["--interp"]));
    let _ = std::fs::remove_dir_all(&dir);
    out
}

/// The four sites, measured 2026-08-03. `utf8` is absent because it takes a
/// list of codepoints by design and already agrees.
#[test]
#[ignore = "needs the ruling on whether bytes are a type or a convention"]
fn a_list_that_was_never_bytes_reads_the_same_on_both_engines() {
    for expr in [
        "text/to_float [1]",
        "text/find2 [97] 0 97 98",
        "text/find2_below [97] 0 97 98 1",
        "text/append [97] \"x\"",
    ] {
        let (native, interp) = both(expr);
        assert_eq!(native, interp, "the engines disagree about `{expr}`");
    }
}

/// The half that already holds, and must go on holding whichever way the
/// ruling goes: real bytes, and a list of codepoints handed to `utf8`.
#[test]
fn genuine_bytes_already_read_the_same_on_both_engines() {
    for expr in [
        "text/to_float (text/bytes \"ab\")",
        "text/find2 (text/bytes \"ab\") 0 97 98",
        "text/append (text/bytes \"a\") \"x\"",
        "text/utf8 [97 98]",
    ] {
        let (native, interp) = both(expr);
        assert_eq!(native, interp, "the engines disagree about `{expr}`");
    }
}
