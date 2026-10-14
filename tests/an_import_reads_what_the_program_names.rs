//! A program pays for the part of the shipped library it reaches (kanso#1952).
//! `std/text` holds two dozen pub functions; a program that calls one of them
//! should not lex, parse, infer and check the rest. Watched red on the
//! compiler before the prune: `text/pad_start` was in the compiled program,
//! which held 48 declarations, and inference visited 627 expressions.

#[test]
fn an_import_reads_only_the_declarations_the_program_names() {
    let source = "import \"std/text\"\n\nprint (text/uppercase \"kanso\")\n";
    kanso::infer::work::reset();
    let program = kanso::compile_entry("main.kso", source).expect("the program compiles");
    let (_, visits) = kanso::infer::work::taken();
    let names: Vec<&str> = program.fns.iter().map(|d| d.name.as_str()).collect();
    assert!(names.iter().any(|n| n.ends_with("uppercase")), "{names:?}");
    assert!(!names.iter().any(|n| n.ends_with("pad_start")), "{names:?}");
    assert_eq!(visits, 224, "expression visits");
}
