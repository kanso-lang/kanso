//! The emitter tests for a failure by comparing a value's tag with the err
//! tag, written as a number, where it used to call `k_not_failure` and let
//! the inliner open it. The number has to be the one the runtime gives
//! `K_ERR`, and the one the declared `k_not_failure` compares against, or
//! every failure check in a compiled program asks about the wrong tag.
//!
//! Watched red with `K_ERR_TAG` set to 6.

/// Where `K_ERR` stands in the runtime's tag enum.
fn runtime_err_tag() -> i64 {
    let runtime = include_str!("../src/runtime.c");
    let line = runtime
        .lines()
        .find(|l| l.starts_with("enum { K_INT,"))
        .expect("runtime.c declares its tag enum on one line");
    let names: Vec<&str> = line
        .trim_start_matches("enum {")
        .trim_end_matches("};")
        .split(',')
        .map(str::trim)
        .collect();
    names.iter().position(|n| *n == "K_ERR").expect("the enum names K_ERR") as i64
}

#[test]
fn the_emitter_compares_against_the_runtime_s_err_tag() {
    assert_eq!(kanso::codegen::K_ERR_TAG, runtime_err_tag());
}

#[test]
fn the_declared_predicate_compares_against_the_same_tag() {
    let emitter = include_str!("../src/codegen.rs");
    let at = emitter
        .find("define internal i64 @k_not_failure(%KValue %v) alwaysinline {")
        .expect("the module still declares k_not_failure");
    let body = &emitter[at..at + 200];
    let want = format!("icmp ne i64 %tag, {}", kanso::codegen::K_ERR_TAG);
    assert!(body.contains(&want), "k_not_failure no longer says `{want}`:\n{body}");
}
