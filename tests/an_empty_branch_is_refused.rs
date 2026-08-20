// A block-if constant whose `else` holds nothing reached the branch-shape
// check with zero lines, and building the diagnostic's span panicked. This
// shape is reachable only through `kanso check` on a library file — the
// run/play corpus cannot hold it, so the pin lives here. Watched red as a
// parser panic before src/parser.rs learned the empty-branch refusal.
use std::process::Command;

#[test]
fn an_empty_branch_is_refused_not_panicked() {
    let dir = std::env::temp_dir().join("kanso-empty-branch");
    let _ = std::fs::create_dir_all(&dir);
    let file = dir.join("empty_branch.kso");
    std::fs::write(&file, "quote = if (1 < 2)\n  \"sale\"\nelse\n").expect("fixture writes");
    let out = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("check")
        .arg(&file)
        .output()
        .expect("kanso runs");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("a branch needs a body: the expression it answers with"),
        "expected the empty-branch refusal, got: {err}"
    );
    assert!(!err.contains("panicked"), "the compiler must diagnose, not panic: {err}");
}
