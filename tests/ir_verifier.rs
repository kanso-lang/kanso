//! Who checks the IR kanso writes.
//!
//! Nothing in this project reads the emitted .ll. `kanso build` writes it and
//! hands it to clang, so the only verifier a program ever meets is the one on
//! the machine that built it — and that verdict has already differed by host:
//! a thunk released in a block its creation did not dominate was refused on
//! macOS and compiled and run on linux, from identical source.
//!
//! The reason is recorded here because it decides what the project can rely
//! on: **clang only runs LLVM's verifier when it was built with assertions.**
//! Apple's clang is; the linux runner's is not, so on that host `clang -c` on
//! a .ll parses the module and codegens it without ever checking it. A tool
//! built with assertions — `opt`, `llvm-as` — checks on any host.
//!
//! So this asks each candidate in turn and needs one of them to refuse a
//! five-line dominance violation. A host with none of them says so instead of
//! going quietly green, and the emitter's own output is then put through
//! whichever answered.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Tools that read a .ll and answer, most-preferred first. The versioned
/// spellings are what the linux images ship; the bare ones are what a
/// developer's path has.
const CANDIDATES: [&str; 12] = [
    "opt",
    "llvm-as",
    "/opt/homebrew/opt/llvm/bin/opt",
    "opt-19",
    "llvm-as-19",
    "opt-18",
    "llvm-as-18",
    "opt-17",
    "llvm-as-17",
    "opt-16",
    "llvm-as-16",
    // last, because it answers only where it was built with assertions
    "clang",
];

fn read_ir(tool: &str, path: &Path) -> Option<Output> {
    let out = std::env::temp_dir().join("kanso_ir_check.out");
    let mut command = Command::new(tool);
    match tool {
        "clang" => command.arg("-c").arg("-o").arg(&out),
        _ if tool.contains("opt") => command.arg("-passes=verify").arg("-disable-output"),
        _ => command.arg("-o").arg(&out),
    };
    command.arg(path).output().ok()
}

/// A register used in a block its definition does not reach. The exact shape
/// LLVM refused in the thunk-release defect, reduced to five lines.
fn does_not_dominate() -> PathBuf {
    let bad = std::env::temp_dir().join("kanso_does_not_dominate.ll");
    std::fs::write(
        &bad,
        "define i64 @f(i64 %n) {\n\
         entry:\n  %c = icmp eq i64 %n, 0\n  br i1 %c, label %made, label %uses\n\n\
         made:\n  %v = add i64 %n, 1\n  br label %done\n\n\
         uses:\n  %w = add i64 %v, 2\n  br label %done\n\n\
         done:\n  %r = phi i64 [ 0, %made ], [ %w, %uses ]\n  ret i64 %r\n}\n",
    )
    .expect("the fixture writes");
    bad
}

/// The first tool on this host that refuses invalid IR. Every test here needs
/// one, and which one it is belongs in the failure message.
fn verifier() -> Option<&'static str> {
    let bad = does_not_dominate();
    CANDIDATES.into_iter().find(|tool| match read_ir(tool, &bad) {
        Some(answer) => {
            !answer.status.success()
                && String::from_utf8_lossy(&answer.stderr).contains("does not dominate all uses")
        }
        None => false,
    })
}

#[test]
fn this_host_has_something_that_reads_ir() {
    assert!(
        verifier().is_some(),
        "no tool on this host refuses IR that does not dominate its uses. \
         Tried {CANDIDATES:?}. clang is not one — it verifies only when built \
         with assertions, which the linux images are not, so a build here \
         checks nothing about the IR kanso wrote."
    );
}

/// And the emitter's own output, on a program that reaches the paths the
/// defect lived on: a bound thunk in a function whose arguments can fail.
#[test]
fn the_ir_kanso_writes_passes_that_verifier() {
    let Some(tool) = verifier() else {
        return; // the test above is the one that reports this
    };
    let dir = std::env::temp_dir().join("kanso_ir_written");
    std::fs::create_dir_all(&dir).expect("the directory is made");
    let source = dir.join("written.kso");
    std::fs::copy(manifest_dir().join("tests/golden/micro/bound_thunk_across_arms.kso"), &source)
        .expect("the fixture copies");

    let built = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("build")
        .arg("written.kso")
        .current_dir(&dir)
        .output()
        .expect("kanso runs");
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));

    let checked = read_ir(tool, &dir.join("written.ll")).expect("the verifier runs");

    assert!(
        checked.status.success(),
        "kanso wrote IR {tool} refuses: {}",
        String::from_utf8_lossy(&checked.stderr)
    );
}
