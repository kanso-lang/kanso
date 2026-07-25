//! What compiling costs, counted rather than timed. Two kinds of number
//! live here. The emitted counts say what codegen wrote; the work counts say
//! what it took to decide — fixpoint rounds and expression visits — because
//! a compiler can grind a long time and emit very little. Wall time would
//! say more about the machine than about either.

use std::fmt::Write as _;

fn ir_for(source: &str) -> String {
    let program = kanso::compile("sample.kso", source, true).expect("sample compiles");
    kanso::codegen::emit_ir(&program).expect("sample lowers to IR")
}

/// The processing the emitted text cost: how many times the fixpoint went
/// round, and how many expressions it looked at getting there.
fn work_for(source: &str) -> (u64, u64) {
    let program = kanso::compile("sample.kso", source, true).expect("sample compiles");
    kanso::infer::work::reset();
    let _ = kanso::infer::infer(&program);
    kanso::infer::work::taken()
}

/// Counts that move only when the emitter's output does.
fn shape(ir: &str) -> (usize, usize, usize, usize) {
    let lines = ir.lines().filter(|l| !l.trim().is_empty()).count();
    let calls = ir.matches(" call ").count();
    let branches = ir.matches("br i1 ").count();
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
  return \"low\" if (n < 10)
  return \"high\" if (100 < n)
  \"middle\"
",
    ),
    (
        "records",
        "type point
  x:int
  y:int

main = print \"{shift (point 1 2)}\"

fn shift (point x y)
  point (x + 1) (y + 1)
",
    ),
    (
        "build_block",
        "type node
  id:int
  peer:any

main =
  ring = build
    a = node 1 0
    b = node 2 0
    set a peer b
    set b peer a
    a
  print \"{ring}\"
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
