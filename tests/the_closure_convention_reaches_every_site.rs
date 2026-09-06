//! `preserve_none` on a closure body is only safe if EVERY site that calls
//! through a closure pointer carries it too. Leaving one behind is a
//! miscompile, not a slowdown: with the emitted fast arm converted and the
//! runtime's `k_call2` left on the C convention, encodebench printed
//! `error[runtime]: bytes takes a string` where its answer is `done: 74072800`.
//!
//! So this asks the emitter for the same program twice, once under each
//! convention, and pins what moves between them. It needs no clang -- the
//! question is what the compiler WRITES, and a spec that shelled out would be
//! measuring the host instead.
use kanso::codegen::{emit_ir, ClosureConvention};

/// A capture-free lambda handed to a function that applies it: the shape that
/// lifts to a `klam` body with a `w_klam` wrapper, and whose application goes
/// through the inlined dispatcher. Self-contained on purpose -- `kanso::compile`
/// on a bare string resolves no imports, the way the other codegen specs work.
const PROGRAM: &str = "\
fn apply f x
  f x

play = apply (y -> y + 1) 41
";

fn ir(convention: ClosureConvention) -> String {
    let program = kanso::compile("spec.kso", PROGRAM, false).expect("the fixture compiles");
    emit_ir(&program, convention).expect("the fixture lowers to IR")
}

#[test]
fn absent_writes_no_convention_anywhere() {
    let out = ir(ClosureConvention::Absent);
    assert!(
        !out.contains("preserve_nonecc"),
        "`Absent` must write IR any clang can take -- it is what the compile-vein \
         tests pass, and what a host without clang 19 gets"
    );
}

#[test]
fn preserve_none_reaches_the_define_and_the_call_site_together() {
    let out = ir(ClosureConvention::PreserveNone);
    let defines = out.matches("define preserve_nonecc %KValue @w_").count();
    let calls = out.matches("call preserve_nonecc %KValue %fnp(").count();
    assert!(defines > 0, "the closure wrapper must carry the convention; found none in:\n{out}");
    assert_eq!(
        calls, 1,
        "the fast arm that calls through the closure pointer must carry it too. \
         A define without its call site is the miscompile this whole change has \
         to avoid."
    );
}

/// The two conventions must differ ONLY by the keyword. Anything else moving
/// would mean the choice is reaching decisions it has no business making --
/// and the compile-side goldens are pinned against `Absent`, so a difference
/// beyond the keyword would make those goldens describe a program nobody
/// builds.
#[test]
fn the_convention_changes_the_keyword_and_nothing_else() {
    let absent = ir(ClosureConvention::Absent);
    let preserve = ir(ClosureConvention::PreserveNone).replace("preserve_nonecc ", "");
    assert_eq!(absent, preserve, "with the keyword removed the two must be byte-identical");
}
