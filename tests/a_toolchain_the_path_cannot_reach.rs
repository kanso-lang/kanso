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
//! ONLY THE FIXED PREFIX IS ASSERTED. Both messages interpolate the host's own
//! io error — `No such file or directory (os error 2)` on this box — and
//! src/eval.rs already carries the reason a host string must not be pinned: it
//! moves with the libc and the toolchain. The prefix is the compiler's, and it
//! is the part a rewording would lose.
use std::process::Command;

fn a_program() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("kanso-no-clang");
    std::fs::create_dir_all(&dir).expect("a directory to build in");
    std::fs::write(dir.join("main.kso"), "import \"std/io\"\n\nio/write \"hi\\n\"\n")
        .expect("the program writes");
    dir
}

/// `kanso build` spawns clang itself, and says so when it cannot.
#[test]
fn a_build_says_it_cannot_invoke_clang() {
    let dir = a_program();
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("build")
        .arg(&dir)
        .env("PATH", "/nonexistent")
        .output()
        .expect("kanso runs");
    let said = String::from_utf8_lossy(&done.stderr);
    assert!(said.contains("cannot invoke clang:"), "build said: {said}");
    assert!(!done.status.success(), "a build with no clang must not exit zero");
}

/// `kanso run` builds through a cached binary, so the same absence arrives
/// under a different sentence. Two messages, one cause, and the corpus had
/// neither.
#[test]
fn a_run_says_it_cannot_build() {
    let dir = a_program();
    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg(&dir)
        .env("PATH", "/nonexistent")
        .output()
        .expect("kanso runs");
    let said = String::from_utf8_lossy(&done.stderr);
    assert!(said.contains("cannot build:"), "run said: {said}");
    assert!(!done.status.success(), "a run with no clang must not exit zero");
}
