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
//! A binding moved into the park fails at the park instead, before anything
//! prints, because the program as written computed it before the sleep.
//!
//! The success path, and the counter that says the pool ran, is in the mem
//! corpus: `tests/golden/mem/work_ahead_forces_what_follows_a_park` and
//! `tests/golden/mem/work_ahead_moves_a_binding_into_the_park`.

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

fn the_transcript_is_the_one_written(verb: &str, name: &str, program: &Path) {
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
    the_transcript_is_the_one_written("play", name, &program);
}

/// A runtime error inside the pool arrives through `exit`, and the work had
/// started building a second constant when it died.
#[test]
fn a_pooled_death_fails_where_it_is_read() {
    let name = "a_pooled_death_fails_where_it_is_read";
    let program = corpus().join(name);
    the_transcript_is_the_one_written("run", name, &program);
}

/// A binding ahead of the park is computed there, and its overflow is raised
/// there too, so "first" never prints. Raised where its value is read, the
/// overflow came after "first".
#[test]
fn a_moved_binding_fails_at_the_park() {
    let name = "a_moved_binding_fails_at_the_park";
    let program = corpus().join(format!("{name}.kso"));
    the_transcript_is_the_one_written("play", name, &program);
}

/// A binding ahead of an effect group fails before the group's first line
/// prints. The sleep in a group does not pool, so nothing moves past "start".
#[test]
fn a_binding_before_a_print_fails_before_it() {
    let name = "a_binding_before_a_print_fails_before_it";
    let program = corpus().join(format!("{name}.kso"));
    the_transcript_is_the_one_written("play", name, &program);
}

/// Two effect lines run as one group, and the pool runs when every side is
/// waiting. The size it lifts runs out of stack there, is dropped, and fails
/// again where "then" reads it, after "start" and "first".
#[test]
fn a_pooled_overflow_in_a_group_fails_where_it_is_read() {
    let name = "a_pooled_overflow_in_a_group_fails_where_it_is_read";
    let program = corpus().join(format!("{name}.kso"));
    the_transcript_is_the_one_written("play", name, &program);
}

/// The rig the compiler page tells a reader to build. Nothing else reads
/// `bench/workahead`, and when files that declare beside their statements
/// became libraries, both programs stopped building without a test noticing.
#[test]
fn the_rig_the_page_names_compiles() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for program in ["aswritten", "overlapped"] {
        let checked = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .current_dir(&root)
            .arg("check")
            .arg(format!("bench/workahead/{program}"))
            .output()
            .expect("kanso runs");
        assert!(
            checked.status.success(),
            "bench/workahead/{program}: {}",
            String::from_utf8_lossy(&checked.stderr)
        );
    }
}
