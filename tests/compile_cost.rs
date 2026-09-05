//! What compiling costs, counted rather than timed. Two kinds of number
//! live here. The emitted counts say what codegen wrote; the work counts say
//! what it took to decide — fixpoint rounds and expression visits — because
//! a compiler can grind a long time and emit very little. Wall time would
//! say more about the machine than about either.

use std::fmt::Write as _;

fn ir_for(source: &str) -> String {
    let program = kanso::compile("sample.kso", source, false).expect("sample compiles");
    kanso::codegen::emit_ir(&program).expect("sample lowers to IR")
}

/// The processing the emitted text cost: how many times the fixpoint went
/// round, and how many expressions it looked at getting there.
fn work_for(source: &str) -> (u64, u64) {
    let program = kanso::compile("sample.kso", source, false).expect("sample compiles");
    kanso::infer::work::reset();
    let _ = kanso::infer::infer(&program);
    kanso::infer::work::taken()
}

/// A sample that is a directory rather than a string, so the module loader is
/// on the path being measured. Every other sample is one file with no
/// imports, which leaves enrollment, qualification and the dependency walk
/// costing whatever they like.
fn module_dir(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/compile").join(name)
}

fn module_entry(name: &str) -> kanso::ast::Program {
    let entry = module_dir(name).join("main.kso");
    let source = std::fs::read_to_string(&entry).expect("the entry reads");
    kanso::compile_entry(&entry.to_string_lossy(), &source).expect("module compiles")
}

fn ir_for_module(name: &str) -> String {
    kanso::codegen::emit_ir(&module_entry(name)).expect("module lowers to IR")
}

fn work_for_module(name: &str) -> (u64, u64) {
    let program = module_entry(name);
    kanso::infer::work::reset();
    let _ = kanso::infer::infer(&program);
    kanso::infer::work::taken()
}

/// Directory samples, measured the same way and reported in the same file.
const MODULES: &[&str] = &["module"];

/// Counts that move only when the emitter's output does.
fn shape(ir: &str) -> (usize, usize, usize, usize) {
    let lines = ir.lines().filter(|l| !l.trim().is_empty()).count();
    // Comment lines are dropped before anything is counted. `calls` and
    // `branches` are substring searches, and the prelude's comments use the
    // word "call" eight times: rewording one of them moved this column with
    // the emitted text byte-identical, which is the one thing the column is
    // here to rule out. scripts/gates/emitted_code.sh reads the .ll the same
    // way, and tests/the_emitted_call_counter_ignores_comments.rs pins it
    // there.
    let code: String =
        ir.lines().filter(|l| !l.starts_with(';')).map(|l| format!("{l}\n")).collect();
    let calls = code.matches(" call ").count();
    let branches = code.matches("br i1 ").count();
    let defines = ir.lines().filter(|l| l.starts_with("define")).count();
    (lines, calls, branches, defines)
}

const SAMPLES: &[(&str, &str)] = &[
    (
        "recursion",
        "fn count 0 acc
  acc

fn count n acc
  count (n - 1) (acc + n)

main = print \"{count 10 0}\"
",
    ),
    (
        "dispatch",
        "fn describe 0
  \"zero\"

fn describe n:int
  \"int {n}\"

fn describe s:string
  \"string {s}\"

main = print (describe 3)
",
    ),
    (
        "guards",
        "main = print (rank 42)

fn rank n
  return \"low\" if n < 10
  return \"high\" if 100 < n
  \"middle\"
",
    ),
    (
        "records",
        "type point
  x
  y

main = print \"{shift (point 1 2)}\"

fn shift (point x y)
  point (x + 1) (y + 1)
",
    ),
    (
        "build_block",
        "type node
  id
  peer

main =
  build
    a = node 1 0
    b = node 2 0
    a.peer = b
    b.peer = a
  print \"{a}\"
",
    ),
];

/// A watched number, not a line in the sand. Compilation and runtime trade
/// against each other, and a language feature can cost one to buy the other,
/// so the point is that nothing moves silently — a diff here asks for a
/// reason, and the reason goes in the log next to the number.
#[test]
fn compile_cost_matches_the_golden() {
    let mut actual = String::new();
    for (name, source) in SAMPLES {
        let (lines, calls, branches, defines) = shape(&ir_for(source));
        let (rounds, visits) = work_for(source);
        writeln!(
            actual,
            "{name} lines={lines} calls={calls} branches={branches} \
             defines={defines} rounds={rounds} visits={visits}"
        )
        .expect("string write");
    }
    let mut modules = String::new();
    for name in MODULES {
        let ir = ir_for_module(name);
        let (lines, calls, branches, defines) = shape(&ir);
        let (rounds, visits) = work_for_module(name);
        writeln!(
            modules,
            "{name} lines={lines} calls={calls} branches={branches} defines={defines} \
             rounds={rounds} visits={visits}"
        )
        .expect("string write");
    }
    // A separate file, because the welfare index sums every row of the one
    // above: adding a sample there would read as a regression the size of the
    // sample, and re-baselining to absorb it would bank a loss that never
    // happened. The index keeps its fixed basis; coverage grows here.
    let module_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("bench/compile_golden_modules.txt");
    if std::env::var("KANSO_REGEN_COMPILE_GOLDEN").is_ok() {
        let header = "\
# What compiling a MODULE costs — an entry, a local module beside it, and the
# standard library each pulls in. Every sample in compile_golden.txt is a
# single file with no imports, which left the loader, enrollment and
# qualification costing whatever they liked.
#
# Kept apart from that file because the welfare index sums its rows: a sample
# added there reads as a regression the size of the sample.
";
        std::fs::write(&module_path, format!("{header}{modules}")).expect("module golden writes");
    }
    let stored_modules = std::fs::read_to_string(&module_path).unwrap_or_default();
    let expected_modules: String =
        stored_modules.lines().filter(|l| !l.starts_with('#')).map(|l| format!("{l}\n")).collect();
    assert_eq!(
        modules, expected_modules,
        "the cost of compiling a module moved. that is allowed — say which way \
         and why, then regenerate with KANSO_REGEN_COMPILE_GOLDEN=1"
    );
    let golden_path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bench/compile_golden.txt");
    if std::env::var("KANSO_REGEN_COMPILE_GOLDEN").is_ok() {
        let header = "\
# What compiling these samples costs. rounds and visits are the work the
# compiler did; lines, calls, branches and defines are what it wrote. The two
# move independently — grinding a longer fixpoint to emit the same text shows
# up here and nowhere else.
#
# This is a watched trend, not a hard floor. A feature may cost compile work
# to buy runtime work, or the reverse. Movement is fine and silence is not:
# regenerate deliberately and record which way it went, and why, in
# design/compiler-log.md beside the runtime goldens.
";
        std::fs::write(&golden_path, format!("{header}{actual}")).expect("golden writes");
        return;
    }
    let stored = std::fs::read_to_string(&golden_path).unwrap_or_default();
    // the file carries its own policy at the top; only the rows compare
    let expected: String =
        stored.lines().filter(|l| !l.starts_with('#')).map(|l| format!("{l}\n")).collect();
    assert_eq!(
        actual, expected,
        "compile cost moved. that is allowed — say which way and why, then \
         regenerate with KANSO_REGEN_COMPILE_GOLDEN=1. rounds and visits are \
         what compiling did; lines, calls and branches are what it wrote"
    );
}
