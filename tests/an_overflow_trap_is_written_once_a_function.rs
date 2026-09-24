//! A function's checked integer ops share one overflow trap.
//!
//! Every `+`, `-` and `*` the emitter proves to be between two integers is an
//! `llvm.*.with.overflow` call and a branch on its flag. The block the branch
//! takes on overflow is the same three lines wherever the op is: a call to
//! `k_die` with the overflow message, and `unreachable`. It was written once an
//! op, so `spread` below carried six copies, and clang parsed each one.
//!
//! The program compiles a function with six checked ops, counts the calls to
//! `k_die` with the overflow message in each defined function, and runs it
//! twice: once on numbers that fit, where native prints what the interpreter
//! prints, and once on numbers that do not, where native dies with the
//! overflow message and the interpreter answers exactly.
//!
//! Watched red with a trap written at every op: `spread` held 6.

use std::process::Command;

const LIBRARY: &str = "pub play = print \"spread {spread 1 2 3}\"

pub over = print \"spread {spread 9223372036854775000 1000 1}\"

fn spread a:int b:int c:int
  a * b + b * c - a + c * 7
";

/// For each defined function, how many times it calls `k_die` with `symbol`.
fn traps(module: &str, symbol: &str) -> Vec<(String, usize)> {
    let call = format!("call void @k_die(ptr {symbol})");
    let mut found = Vec::new();
    let mut current: Option<(String, usize)> = None;
    for line in module.lines() {
        if line.starts_with("define ") {
            current = Some((line.to_string(), 0));
        } else if line == "}" {
            found.extend(current.take());
        } else if let Some((_, n)) = current.as_mut() {
            if line.trim() == call {
                *n += 1;
            }
        }
    }
    found
}

/// Builds `main.kso` calling `entry` from the library, then runs it natively
/// and on the interpreter: (module, native output, oracle output).
fn built(entry: &str) -> (String, std::process::Output, std::process::Output) {
    let dir =
        std::env::temp_dir().join(format!("kanso_overflow_trap_{entry}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("counted")).expect("a module directory");
    std::fs::write(dir.join("counted/counted.kso"), LIBRARY).expect("the library writes");
    let main = format!("import \"./counted\"\n\n{entry}\n");
    std::fs::write(dir.join("main.kso"), main).expect("the program writes");
    let build = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("build")
        .arg("main.kso")
        .current_dir(&dir)
        .output()
        .expect("kanso runs");
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let native = Command::new(dir.join("main")).output().expect("the binary runs");
    let oracle = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg("main.kso")
        .arg("--interp")
        .current_dir(&dir)
        .output()
        .expect("the interpreter runs");
    let module = std::fs::read_to_string(dir.join("main.ll")).expect("the module reads");
    let _ = std::fs::remove_dir_all(&dir);
    (module, native, oracle)
}

#[test]
fn a_function_writes_its_overflow_trap_once() {
    let (module, native, oracle) = built("play");
    assert_eq!(native.stdout, oracle.stdout, "the engines disagree");
    assert_eq!(String::from_utf8_lossy(&native.stdout), "spread 28\n");
    let symbol = module
        .lines()
        .find(|l| l.contains("c\"integer overflow ("))
        .and_then(|l| l.split(' ').next())
        .expect("the module carries the overflow message")
        .to_string();
    let counts = traps(&module, &symbol);
    let spread =
        counts.iter().find(|(name, _)| name.contains("spread")).expect("spread is defined");
    assert_eq!(spread.1, 1, "spread's overflow traps");
    let many: Vec<_> = counts.iter().filter(|(_, n)| *n > 1).collect();
    assert!(many.is_empty(), "functions with more than one overflow trap: {many:?}");
}

#[test]
fn the_shared_trap_still_dies_with_the_overflow_message() {
    let (_, native, oracle) = built("over");
    assert_eq!(String::from_utf8_lossy(&oracle.stdout), "spread 9214148664817920226007\n");
    assert!(!native.status.success(), "native answered: {native:?}");
    assert!(native.stdout.is_empty(), "native printed: {native:?}");
    let err = String::from_utf8_lossy(&native.stderr);
    assert!(err.contains("integer overflow (int64 native build"), "{err}");
}
