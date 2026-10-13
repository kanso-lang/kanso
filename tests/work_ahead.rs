//! Work-ahead (#1856): a pooled failure is raised where the program as
//! written raises it (ruled 2026-10-11).
//!
//! Each fixture lifts a call that dies, and the pool forces it during the
//! sleep, before "first" prints. The transcript must be the one the program
//! produces without work-ahead, which is what the interpreter produces, since
//! it does not pool. A pool that let the death end the process printed
//! nothing at all on stdout, and one that caught the death but left the cell
//! holding its blackhole printed "a lazy binding demands its own value" where
//! the recursion runs out of stack.
//!
//! The success path, and the counter that says the pool ran, is in the mem
//! corpus: `tests/golden/mem/work_ahead_forces_what_follows_a_park`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/workahead")
}

fn golden(name: &str, extension: &str) -> String {
    let path = corpus().join(format!("{name}.{extension}"));
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing golden {path:?}"))
}

fn transcript(verb: &str, program: &Path, engine: &[&str]) -> (String, String, Option<i32>) {
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg(verb)
        .arg(program)
        .args(engine)
        .env("KANSO_SEED", "2685821657736338717")
        .output()
        .expect("kanso runs");
    (
        String::from_utf8_lossy(&done.stdout).into_owned(),
        String::from_utf8_lossy(&done.stderr).into_owned(),
        done.status.code(),
    )
}

fn the_failure_is_raised_where_it_is_read(verb: &str, name: &str, program: &Path) {
    for engine in [&[][..], &["--interp"][..]] {
        let (out, err, code) = transcript(verb, program, engine);

        assert_eq!(out, golden(name, "stdout"), "{name}, engine {engine:?}: {err}");
        assert_eq!(err, golden(name, "stderr"), "{name}, engine {engine:?}");
        assert_eq!(code, Some(1), "{name}, engine {engine:?}");
    }
}

/// A stack overflow inside the pool arrives as a signal, on the signal's own
/// stack.
#[test]
fn a_pooled_overflow_fails_where_it_is_read() {
    let name = "a_pooled_overflow_fails_where_it_is_read";
    let program = corpus().join(format!("{name}.kso"));
    the_failure_is_raised_where_it_is_read("play", name, &program);
}

/// A runtime error inside the pool arrives through `exit`, and the work had
/// started building a second constant when it died.
#[test]
fn a_pooled_death_fails_where_it_is_read() {
    let name = "a_pooled_death_fails_where_it_is_read";
    let program = corpus().join(name);
    the_failure_is_raised_where_it_is_read("run", name, &program);
}
