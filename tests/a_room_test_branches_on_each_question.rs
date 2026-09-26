//! A fast path that asks whether a buffer has room asks its two questions
//! with two branches.
//!
//! Every in-place push, put and append first asks whether the buffer's used
//! mark is at the frontier and then whether the new length fits its capacity.
//! Joined with `and i1` and one branch, as they were, LLVM at `-O3` computes
//! both as flags and ors them: `cmp`, `setne`, `cmp`, `setg`, `or`, `jne`
//! where two compares and two branches do. It splits such a branch only under
//! fast instruction selection. On the run program the split fast paths are
//! reached some ten million times.
//!
//! This reads the emitter's text, the way the capacity spec does, because the
//! room tests are the prelude's and a program reaches only the ones it uses.
//!
//! Watched red: with the list push's two tests joined again, the scan names
//! the `and` it finds.

use std::path::Path;

#[test]
fn no_room_test_joins_its_questions() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let codegen = std::fs::read_to_string(root.join("src/codegen.rs")).expect("codegen reads");
    let joined: Vec<&str> = codegen
        .lines()
        .filter(|l| l.trim_start().starts_with('%') && l.contains(" = and i1 %"))
        .filter(|l| l.contains("front, %"))
        .collect();
    assert!(joined.is_empty(), "a room test joins its questions again: {joined:?}");
    let split = codegen
        .lines()
        .filter(|l| l.starts_with("  br i1 %") && l.contains("front, label %"))
        .count();
    assert_eq!(split, 9, "the emitter should branch on the frontier at all nine room tests");
}
