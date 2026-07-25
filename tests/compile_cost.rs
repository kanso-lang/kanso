//! What compiling costs, counted rather than timed. Wall time says more
//! about the machine than the compiler; these numbers are exactly what
//! codegen chose to write, so a change that emits more work shows up as a
//! diff instead of a slower afternoon nobody can reproduce.

use std::fmt::Write as _;

fn ir_for(source: &str) -> String {
    let program = kanso::compile("sample.kso", source, true).expect("sample compiles");
    kanso::codegen::emit_ir(&program).expect("sample lowers to IR")
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

/// The golden lives beside the runtime cost goldens and is diffed the same
/// way: regenerate deliberately, never to make a red build green.
#[test]
fn compile_cost_matches_the_golden() {
    let mut actual = String::new();
    for (name, source) in SAMPLES {
        let (lines, calls, branches, defines) = shape(&ir_for(source));
        writeln!(
            actual,
            "{name} lines={lines} calls={calls} branches={branches} defines={defines}"
        )
        .expect("string write");
    }
    let golden_path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bench/compile_golden.txt");
    if std::env::var("KANSO_REGEN_COMPILE_GOLDEN").is_ok() {
        std::fs::write(&golden_path, &actual).expect("golden writes");
        return;
    }
    let expected = std::fs::read_to_string(&golden_path).unwrap_or_default();
    assert_eq!(
        actual, expected,
        "compile cost moved. if the change is intended, regenerate with \
         KANSO_REGEN_COMPILE_GOLDEN=1 and say why in the PR"
    );
}
