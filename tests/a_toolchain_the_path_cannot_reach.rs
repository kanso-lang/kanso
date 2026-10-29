//! `clang` need not be uninstalled to be missing — only unfindable by the one
//! process that looks for it. `PATH` is per-process, so a test can take it away
//! without touching the machine.
//!
//! Both of these sat on tests/golden/unpinned_diagnostics.txt under the same
//! excuse: "A fixture cannot portably cause one: the container runs as root,
//! so an unwritable directory is not unwritable, and clang is installed." The
//! first clause is true and still covers the three write cases — root writes
//! into a mode-000 directory, measured. The second reads "clang is installed"
//! as though installation were the question. What the compiler asks is whether
//! `PATH` resolves it, and that is a variable rather than a fact about the box.
//!
//! The messages are asserted whole. Each used to interpolate the host's io
//! error, `No such file or directory (os error 2)`, which named a missing file
//! and not clang, and `run` and `play` never said clang at all (#2217). A
//! missing clang is now worded by the compiler, so no host text is in it.
use std::process::Command;

/// One directory per test, named by the caller. Both tests used to stage into
/// `kanso-no-clang`, and cargo runs them on separate threads: `fs::write`
/// truncates before it writes, so one test emptied `main.kso` while the other
/// test's `kanso` was reading it, and the build reported `an entry file needs
/// at least one statement` instead of the message under test. One failure in
/// twenty on a settled tree. kanso#1169, kanso#1175 and kanso#1177 are the
/// same defect at three other staging sites.
fn a_program(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kanso-no-clang-{name}"));
    std::fs::create_dir_all(&dir).expect("a directory to build in");
    std::fs::write(dir.join("main.kso"), "import \"std/io\"\n\nio/write \"no clang here\\n\"\n")
        .expect("the program writes");
    dir
}

fn without_clang(dir: &std::path::Path, args: &[&str]) -> (String, bool) {
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(args)
        .current_dir(dir)
        .env("PATH", "/nonexistent")
        .output()
        .expect("kanso runs");
    (String::from_utf8_lossy(&done.stderr).into_owned(), done.status.success())
}

/// `kanso build` spawns clang itself, and has no interpreter to offer.
#[test]
fn a_build_says_clang_is_not_on_the_path() {
    let (said, ok) = without_clang(&a_program("build"), &["build", "."]);
    assert_eq!(
        said,
        "error: cannot build: clang is not on PATH, and `kanso build` needs it to write a \
         native binary. install clang and build again\n"
    );
    assert!(!ok, "a build with no clang must not exit zero");
}

/// `kanso run` builds through a cached binary, and the interpreter runs the
/// same program without clang, so the message names that way out.
#[test]
fn a_run_says_clang_is_not_on_the_path() {
    let (said, ok) = without_clang(&a_program("run"), &["run", "."]);
    assert_eq!(
        said,
        "error: cannot build: clang is not on PATH, and `kanso run` builds a native binary \
         with it. install clang, or run on the interpreter with `kanso run . --interp`\n"
    );
    assert!(!ok, "a run with no clang must not exit zero");
}

/// `kanso play` is the first command chapter 01 teaches.
#[test]
fn a_play_says_clang_is_not_on_the_path() {
    let dir = a_program("play");
    // A text no other test plays, since an earlier play's binary would run
    // without clang and answer for this one.
    std::fs::write(dir.join("h.kso"), "print \"no clang {6 * 7}\"\n").expect("the file writes");
    let (said, ok) = without_clang(&dir, &["play", "h.kso"]);
    assert_eq!(
        said,
        "error: cannot build: clang is not on PATH, and `kanso play` builds a native binary \
         with it. install clang, or run on the interpreter with `kanso play h.kso --interp`\n"
    );
    assert!(!ok, "a play with no clang must not exit zero");
}
