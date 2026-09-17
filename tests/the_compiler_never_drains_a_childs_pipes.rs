//! The compiler launches its children with `status()`, never `output()`.
//!
//! `output()` opens pipes to the child and drains them, and how many `poll` and
//! `read` calls that takes depends on when the child's bytes arrive — which is
//! scheduling, not work. The codegen rows count a whole `kanso build` tree with
//! `--trace-children`, so that loop is inside the measurement.
//!
//! Measured 2026-09-17: two runs of one binary on one box, same corpus, same
//! environment, counted 1,016,046,470 and 1,016,048,745. Eight frames of ten
//! thousand six hundred and forty-two accounted for the whole difference, and
//! every one of them was the pipe-draining loop — `FileDesc::read_to_end` +903,
//! `small_probe_read` +591, `read_output` +253, `read` +220, `__memcpy_avx`
//! +176, `poll` +88, `__errno_location` +44.
//!
//! The `preserve_none` probe was the only caller, and it read nothing but
//! `status.success()`. With the child's streams sent to null and `status()` in
//! place of `output()` there is no pipe and no loop.
//!
//! `src/eval.rs` still uses `output()` and must: `os/run` hands a kanso program
//! the child's stdout and stderr, so draining them is the feature. That file is
//! not on this path — the compile and codegen rows never run a kanso program's
//! `os/run`.

use std::path::Path;

#[test]
fn main_rs_launches_every_child_with_status() {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/main.rs");
    let text = std::fs::read_to_string(&p).expect("src/main.rs reads");

    let hits: Vec<String> = text
        .lines()
        .enumerate()
        .filter(|(_, l)| l.contains(".output()"))
        .map(|(n, l)| format!("src/main.rs:{}: {}", n + 1, l.trim()))
        .collect();
    assert!(
        hits.is_empty(),
        "the compiler drains a child's pipes, and how long that takes is \
         scheduling rather than work:\n{}",
        hits.join("\n")
    );

    assert!(
        text.contains("fn preserve_none_probe"),
        "this spec is pointed at the file that holds the probe, and the probe \
         has moved"
    );
}

/// And the probe's own streams go nowhere, so `status()` did not merely move
/// the child's noise onto the compiler's stderr.
#[test]
fn the_probe_sends_its_childs_streams_to_null() {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/main.rs");
    let text = std::fs::read_to_string(&p).expect("src/main.rs reads");
    let body = text
        .split_once("fn preserve_none_probe")
        .expect("the probe is in this file")
        .1
        .split_once("\nfn ")
        .expect("the probe ends where the next function begins")
        .0;
    for want in ["Stdio::null()", ".status()"] {
        assert!(
            body.contains(want),
            "the probe does not use {want}, so its child's output is either \
             drained or printed"
        );
    }
}
