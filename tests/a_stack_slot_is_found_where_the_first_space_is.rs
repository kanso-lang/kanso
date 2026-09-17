//! A stack slot is found by looking at the line's first space, and that finds
//! exactly the lines reading the whole line found.
//!
//! `FnEmit::write` diverts every `alloca` to the head of the entry block, and
//! it used to recognise one by searching the whole line for ` = alloca `. That
//! search was the emitter's fourth-largest frame on the build profile: 18.2
//! million instructions over 144,261 lines, about 126 a line. A slot reads
//! `%name = alloca <type>` and `%name` holds no space, so the needle begins at
//! the line's first space or it is nowhere.
//!
//! The narrowing is only safe while that holds for every line the emitter
//! writes -- including lines nobody has written yet, which is the whole reason
//! the check sits in `write` rather than at the seven sites that emit an
//! alloca. A line that carried the needle somewhere else would be written into
//! the block that asked for it, LLVM would keep a frame pointer for the
//! function, and the slot would be claimed afresh on every pass: the 8.78% the
//! decoder was paying before the diversion existed. Nothing would go red.
//!
//! So the two readings are run against each other over real emitted IR: every
//! line of the library, of a module and of the benchmark the build profile was
//! taken on. The whole-line reading is the oracle and it stays here.

use kanso::codegen::{emit_ir, is_a_stack_slot, ClosureConvention};

/// What `write` used to ask. The oracle: a slot is a line holding the needle
/// anywhere at all.
fn reading_the_whole_line(text: &str) -> bool {
    text.contains(" = alloca ")
}

fn ir_for(name: &str, source: &str) -> String {
    let program = kanso::compile(name, source, false).unwrap_or_else(|e| panic!("{name}: {e}"));
    emit_ir(&program, ClosureConvention::Absent).unwrap_or_else(|e| panic!("{name} lowers: {e}"))
}

fn ir_for_module(dir: &std::path::Path) -> String {
    let entry = dir.join("main.kso");
    let source = std::fs::read_to_string(&entry).expect("the entry reads");
    let program = kanso::compile_entry(&entry.to_string_lossy(), &source)
        .unwrap_or_else(|e| panic!("{}: {e}", entry.display()));
    emit_ir(&program, ClosureConvention::Absent).expect("the module lowers")
}

/// Every line of IR this tree can emit without leaving the repository.
fn every_emitted_line() -> Vec<(String, String)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut out = Vec::new();

    let module = root.join("tests/golden/compile/module");
    out.push(("tests/golden/compile/module".to_string(), ir_for_module(&module)));

    // The benchmark the 18.2 million was measured on.
    let bench = root.join("bench/runbench/main.kso");
    if bench.exists() {
        let source = std::fs::read_to_string(&bench).expect("runbench reads");
        let program =
            kanso::compile_entry(&bench.to_string_lossy(), &source).expect("runbench compiles");
        let ir = emit_ir(&program, ClosureConvention::Absent).expect("runbench lowers");
        out.push(("bench/runbench".to_string(), ir));
    }

    // And a sample that asks for stack slots outright, so the population is
    // never all negatives: a list literal lowers to `alloca [N x %KValue]`.
    out.push((
        "a list literal".to_string(),
        ir_for("slots.kso", "pub play =\n  xs = [1 2 3]\n  ys = [xs xs]\n  print (length ys)\n"),
    ));

    out
}

#[test]
fn the_two_readings_agree_on_every_line() {
    let mut lines = 0usize;
    let mut slots = 0usize;
    for (name, ir) in every_emitted_line() {
        for line in ir.lines() {
            // `write` is handed the line without the indent it adds.
            let text = line.strip_prefix("  ").unwrap_or(line);
            let narrow = is_a_stack_slot(text);
            let whole = reading_the_whole_line(text);
            assert_eq!(
                narrow, whole,
                "{name}: the two readings disagree on {text:?} -- first space \
                 says {narrow}, whole line says {whole}. A slot the narrow \
                 reading misses stays in the block that asked for it and \
                 nothing else goes red."
            );
            lines += 1;
            slots += usize::from(whole);
        }
    }
    assert!(
        lines > 10_000,
        "only {lines} lines of IR were read, so this agreement is not worth \
         much; the corpora above did not lower"
    );
    assert!(
        slots > 0,
        "{lines} lines and not one stack slot among them, so the agreement is \
         between two functions that both said no to everything"
    );
}

/// The narrow reading is narrow: it stops at the first space rather than
/// searching on. A line that carries the needle later reads false, which is
/// the exact assumption the test above is holding the emitter to.
#[test]
fn the_reading_stops_at_the_first_space() {
    assert!(is_a_stack_slot("%a12 = alloca [3 x %KValue]"));
    assert!(is_a_stack_slot(" = alloca i8"));
    assert!(!is_a_stack_slot("store %KValue %a12, ptr %b"));
    assert!(!is_a_stack_slot("%a12"));
    assert!(!is_a_stack_slot(""));
    assert!(
        !is_a_stack_slot("; the slot %a12 = alloca [3 x %KValue] was hoisted"),
        "a needle past the first space is deliberately not found, and \
         the_two_readings_agree_on_every_line is what says the emitter never \
         writes one"
    );
}
