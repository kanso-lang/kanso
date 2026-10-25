//! The tab emits only the groups the entry reaches, by the names its bodies
//! mention. When the walk misses one the module still calls, the emitter has
//! to notice and emit again with the group put back; these pin that it
//! notices, and that it stays quiet when nothing called was left out.

use super::{emit_with, reached_from_entry, ClosureConvention};

// The comment names `spread`, so the loader reads it although nothing
// calls it: an import reads only the declarations its program names.
const SOURCE: &str =
    "import \"std/list\"\n\n# spread is read and never called\nprint \"{list/sum [3 1 2]}\"\n";

fn program() -> crate::ast::Program {
    crate::compile_source("run", "main.kso", SOURCE).expect("the fixture compiles")
}

/// The program without the groups named `gone`, and those groups.
fn without(gone: &[&str]) -> (crate::ast::Program, Vec<crate::ast::FnDecl>) {
    let mut program = program();
    let (out, kept): (Vec<_>, Vec<_>) =
        program.fns.drain(..).partition(|d| gone.contains(&d.name.as_str()));
    program.fns = kept;
    (program, out)
}

#[test]
fn the_walk_reaches_what_the_entry_calls_and_not_the_rest() {
    let program = program();
    let reached = reached_from_entry(&program).expect("the fixture has an entry");
    assert!(reached.contains("list/sum"), "the walk missed a call: {reached:?}");
    assert!(!reached.contains("list/spread"), "the walk kept a group nothing calls: {reached:?}");
}

/// Emits the program with the groups named `gone` taken out, as the tab
/// takes out what its walk does not reach, and says whether the emitter
/// noticed one it needed.
fn noticed_without(gone: &[&str]) -> bool {
    let (program, out) = without(gone);
    assert_eq!(out.len(), gone.len(), "the fixture lacks one of {gone:?}");
    let names = out.iter().map(|d| d.name.clone()).collect();
    match emit_with(&program, ClosureConvention::Absent, false, &Vec::new(), names) {
        Ok(written) => {
            written.asked_left_out
                || written.names_any(out.iter().map(|d| (d.name.as_str(), d.params.len())))
        }
        Err(_) => true,
    }
}

/// The tab lets go of what its walk left out before it emits, and a
/// module it writes that way is the module the build that keeps them
/// writes.
#[test]
fn the_lean_emit_writes_what_the_whole_one_writes() {
    let lean = super::emit_ir_tab_pieces(program(), ClosureConvention::Absent)
        .expect("the fixture emits")
        .expect("the fixture misses nothing");
    let whole = super::emit_ir_tab_pieces_whole(program(), ClosureConvention::Absent)
        .expect("the fixture emits");
    assert!(lean == whole, "the lean emit wrote another module");
}

#[test]
fn a_group_nothing_calls_left_out_goes_unremarked() {
    assert!(!noticed_without(&["list/spread"]), "the emitter asked for a group nothing calls");
}

#[test]
fn a_called_group_left_out_is_noticed() {
    assert!(
        noticed_without(&["list/sum"]),
        "the entry calls `list/sum`, which was left out, and nothing noticed"
    );
}

/// An operator's arms are found by asking whether the program declares
/// the operator, and with the arm gone the answer sends records down the
/// numeric path. Nothing calls `+` by name, so the module names no symbol
/// of it either way: only the question shows the miss.
#[test]
fn a_question_about_a_left_out_operator_is_noticed() {
    // the micro golden an_operator_arm_takes_its_records_boxed, inline
    let source = "import \"std/os\"\n\ntype pt\n  n\n  s\n\n\
                  fn + (pt a s) (pt b t)\n  pt (a + b) \"{s}{t}\"\n\n\
                  pub play = os/args .> (_ -> print \"{pt 1 \"a\" + pt 3 \"b\"}\")\n";
    let mut program = crate::compile_source("check", "arms.kso", source).expect("it compiles");
    let before = program.fns.len();
    program.fns.retain(|d| d.name != "+");
    assert_eq!(program.fns.len() + 1, before, "the fixture has one `+` arm to leave out");
    let names = std::iter::once("+".to_string()).collect();
    let written = emit_with(&program, ClosureConvention::Absent, false, &Vec::new(), names)
        .expect("records added with no arm still lower");
    assert!(written.asked_left_out, "the emitter asked about `+` and nothing saw it");
}

/// The fallback sees a question about a left-out group only when the
/// question goes through `group_named` or `group_indices`. A scan of the
/// program's groups by name anywhere else in the emitter would answer
/// without being seen, and the tab would emit a program with the answer
/// wrong. The patterns are built here so this file does not match itself.
#[test]
fn a_lookup_by_name_goes_through_the_index() {
    let source = include_str!("../codegen.rs");
    let scan = concat!("self.program.fns", ".iter()");
    let index = concat!("self.group_by", "_name");
    assert_eq!(source.matches(scan).count(), 0, "the emitter scans its groups by hand");
    assert_eq!(source.matches(index).count(), 2, "the emitter reads the index around the helpers");
}
