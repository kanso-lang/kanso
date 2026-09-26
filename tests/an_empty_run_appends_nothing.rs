//! A slice appended to a byte builder that turns out to be empty or out of
//! range hands back the builder before claiming or copying anything.
//!
//! The emitted append of a slice used to carry an empty span through the
//! whole fast path as a zero length, chosen by `select` against a joined
//! range test: a fifth of the runs between two escapes in the JSON decoder
//! are empty. The span's two bounds are now two branches, with the
//! container's length loaded between them so LLVM cannot fold them back
//! together, and both lead out to a return of the accumulator as it stands.
//!
//! This reads the emitter's text, the way the room-test spec does, because
//! the path is the prelude's and a program reaches it only when it appends a
//! slice of bytes.
//!
//! Watched red: with the length loaded ahead of the first branch, the scan
//! says so.

use std::path::Path;

#[test]
fn an_empty_run_returns_the_accumulator_between_its_bounds() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let codegen = std::fs::read_to_string(root.join("src/codegen.rs")).expect("codegen reads");
    let lines: Vec<&str> = codegen.lines().collect();
    let at = |line: &str| {
        let found: Vec<usize> =
            lines.iter().enumerate().filter(|(_, l)| **l == line).map(|(i, _)| i).collect();
        assert_eq!(found.len(), 1, "`{line}` should appear once in the emitter, found {found:?}");
        found[0]
    };
    let lower = at("  br i1 %qorder, label %qspanhi, label %qempty");
    let length = at("  %qclen = load i64, ptr %qc");
    let upper = at("  br i1 %qhi, label %qspan, label %qempty");
    let empty = at("qempty:");
    assert!(
        lower < length && length < upper,
        "the span's length should be loaded between its two bounds tests (lines {lower}, \
         {length}, {upper})"
    );
    assert_eq!(lines[empty + 1], "  ret %KValue %acc", "an empty run should return the builder");
    assert!(
        !codegen.contains("select i1 %qgood"),
        "the append of a slice chooses a zero length by select again"
    );
}
