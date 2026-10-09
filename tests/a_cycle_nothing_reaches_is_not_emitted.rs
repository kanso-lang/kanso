//! A group of functions that call each other round, and that nothing the
//! program runs can reach, is not emitted.
//!
//! std/list sorts by merging, and the merge is four functions calling round:
//! `merge`, `merge_on`, `pick` and `advance`. The prune used to strike a
//! definition once no other surviving definition named it, so it struck
//! `sort`, `msort` and `span` and kept the four, since each was still named by
//! the one before it. On the codegen corpus, which never sorts, the cycles it
//! kept and what only they called were 995 of 3,068 lines of IR.
//!
//! A program that sums must not carry the merge, one that sorts must, and both
//! must print what the interpreter prints.
//!
//! Watched red with the prune counting mentions: the summing program's module
//! defined `d_list/merge_5`.
//!
//! Since 2026-10-09 a native build walks from its entry before it emits, so a
//! group no body names is never emitted and the prune has nothing to strike
//! in the summing program. What it still strikes is a group a body names but
//! the module never calls: `text/to_float (text/slice ...)` is emitted as one
//! fused runtime call, so std/text's `to_float` survives the walk and is
//! called by nothing. Watched red with every definition marked live: the
//! module defined `d_text/to_float_1`.

use std::process::Command;

fn built(name: &str, source: &str) -> (String, String, String) {
    let dir = std::env::temp_dir().join(format!("kanso_dead_cycle_{name}_{}", std::process::id()));
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

const MERGE: &str = "define tailcc %KValue @\"d_list/merge_5\"(";

#[test]
fn a_program_that_never_sorts_carries_no_merge() {
    let (module, native, oracle) =
        built("summed", "import \"std/list\"\n\nprint \"{list/sum [3 1 2]}\"\n");
    assert_eq!(native, oracle, "the engines disagree");
    assert!(!module.contains(MERGE), "a program that never sorts emitted the merge");
}

#[test]
fn a_program_that_sorts_keeps_the_merge() {
    let (module, native, oracle) =
        built("sorted", "import \"std/list\"\n\nprint \"{list/sort [3 1 2]}\"\n");
    assert_eq!(native, oracle, "the engines disagree");
    assert!(module.contains(MERGE), "a program that sorts lost the merge");
}

const TO_FLOAT: &str = "@\"d_text/to_float_1\"(";

#[test]
fn a_library_function_only_a_fused_call_names_is_not_emitted() {
    let (module, native, oracle) = built(
        "fused",
        "import \"std/text\"\n\ncs = \"x1.5\"\nprint \"{text/to_float (text/slice cs 2 4)}\"\n",
    );
    assert_eq!(native, oracle, "the engines disagree");
    assert!(!module.contains(TO_FLOAT), "a fused call left the function it replaces in the module");
}
