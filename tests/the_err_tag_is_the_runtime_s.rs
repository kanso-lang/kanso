//! The emitter tests for a failure by comparing a value's tag with the err
//! tag, written as a number, where it used to call an LLVM twin of
//! `k_not_failure` and let the inliner open it. The number has to be the one
//! the runtime gives `K_ERR`, or every failure check in a compiled program
//! asks about the wrong tag. `tests/perf_ratchet.rs` checks that the emitted
//! compares use it.
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
