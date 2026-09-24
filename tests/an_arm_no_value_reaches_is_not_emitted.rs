//! An arm whose pattern names a record type the program never builds is not
//! emitted, and neither is what only that arm calls.
//!
//! std/list's `next` has an arm for every lazy adapter the library declares,
//! so a program that maps once used to emit every adapter's arm and each
//! arm's helpers: on the codegen corpus 4,628 lines of IR, 3,068 without them,
//! and the child tree of a release build 1,613,493,611 instructions against
//! 856,255,424.
//!
//! A program that maps and never drops must not carry `next_skipped`, and one
//! that drops must, and both must print what the interpreter prints.
//!
//! Watched red with the pass returning no pruned program: the mapping
//! program's module defined `d_list/next_skipped_2`.

use std::process::Command;

fn built(name: &str, source: &str) -> (String, String, String) {
    let dir = std::env::temp_dir().join(format!("kanso_unbuilt_arm_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    std::fs::write(dir.join("main.kso"), source).expect("the program writes");
    let build = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("build")
        .arg("main.kso")
        .current_dir(&dir)
        .output()
        .expect("kanso runs");
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let native = Command::new(dir.join("main")).output().expect("the binary runs");
    let oracle = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg("main.kso")
        .arg("--interp")
        .current_dir(&dir)
        .output()
        .expect("the interpreter runs");
    let module = std::fs::read_to_string(dir.join("main.ll")).expect("the module reads");
    let _ = std::fs::remove_dir_all(&dir);
    (
        module,
        String::from_utf8_lossy(&native.stdout).into_owned(),
        String::from_utf8_lossy(&oracle.stdout).into_owned(),
    )
}

const SKIPPED: &str = "define tailcc %KValue @\"d_list/next_skipped_2\"(";

#[test]
fn a_program_that_never_drops_carries_no_skipping_arm() {
    let (module, native, oracle) = built(
        "mapped",
        "import \"std/list\"\n\nprint \"{list/to_list (list/map [1 2 3] (x -> x * 2))}\"\n",
    );
    assert_eq!(native, oracle, "the engines disagree");
    assert!(!module.contains(SKIPPED), "a program that never drops emitted next_skipped");
}

#[test]
fn a_program_that_drops_keeps_the_skipping_arm() {
    let (module, native, oracle) = built(
        "skipped",
        "import \"std/list\"\n\nprint \"{list/to_list (list/drop (list/map [1 2 3] (x -> x * 2)) 1)}\"\n",
    );
    assert_eq!(native, oracle, "the engines disagree");
    assert!(module.contains(SKIPPED), "a program that drops lost next_skipped");
}
