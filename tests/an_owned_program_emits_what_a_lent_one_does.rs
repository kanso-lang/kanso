//! A caller done with its program hands it to the emitter, which then drops
//! the arms nothing can reach from that program where it stands. A caller that
//! goes on using its program lends it, and the emitter copies the arms it
//! keeps. The two must write the same module.
//!
//! `kanso build` and the browser hand theirs over. Copying the kept arms was
//! the largest thing the tab's compile held besides the program itself.
//!
//! Watched red with the owned path keeping every arm: the owned module defined
//! `d_list/next_skipped_2`, which the lent one does not.

use kanso::codegen::{emit_ir, emit_ir_dev, emit_ir_dev_owned, emit_ir_owned, ClosureConvention};

const MAPPED: &str =
    "import \"std/list\"\n\nprint \"{list/to_list (list/map [1 2 3] (x -> x * 2))}\"\n";

fn program() -> kanso::ast::Program {
    kanso::compile_entry("main.kso", MAPPED).expect("the program compiles")
}

#[test]
fn a_dev_module_is_the_same_lent_or_owned() {
    let lent = emit_ir_dev(&program(), ClosureConvention::Absent).expect("lent lowers");
    let owned = emit_ir_dev_owned(program(), ClosureConvention::Absent).expect("owned lowers");
    assert!(!lent.contains("@\"d_list/next_skipped_2\"("), "the lent module kept an unbuilt arm");
    assert_eq!(lent, owned);
}

#[test]
fn a_release_module_is_the_same_lent_or_owned() {
    let lent = emit_ir(&program(), ClosureConvention::Absent).expect("lent lowers");
    let owned = emit_ir_owned(program(), ClosureConvention::Absent).expect("owned lowers");
    assert_eq!(lent, owned);
}
