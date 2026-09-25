//! A tag already known to be the int tag is not compared with it.
//!
//! Arithmetic on two values asks whether both tags are 0 before it takes the
//! fast path. A literal's tag is known, and so is an unboxed parameter's, and
//! `n + 1` used to write `icmp eq i64 0, 0` and an `and` for the literal on
//! every addition.
//!
//! The program adds a literal to a boxed value and compares it with one. No
//! line in its module may compare two constants, and the binary must print
//! what the interpreter prints.
//!
//! Watched red with every tag compared: five lines compared 0 with 0.

use std::process::Command;

const MAIN: &str = "import \"./counted\"\n\nplay\n";

const LIBRARY: &str = "import \"std/list\"

pub play = print \"sum {list/sum (list/map [1 2 3] bumped)}\"

fn bumped x
  if (x > 1) (x + 1) x
";

#[test]
fn no_line_compares_two_constants() {
    let dir = std::env::temp_dir().join(format!("kanso_known_tag_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("counted")).expect("a module directory");
    std::fs::write(dir.join("counted/counted.kso"), LIBRARY).expect("the library writes");
    std::fs::write(dir.join("main.kso"), MAIN).expect("the program writes");
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
    assert_eq!(native.stdout, oracle.stdout, "the engines disagree");
    assert_eq!(String::from_utf8_lossy(&native.stdout), "sum 8\n");
    let constant = |w: &str| w.trim_end_matches(',').parse::<i64>().is_ok();
    let found: Vec<&str> = module
        .lines()
        .filter(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            match words.iter().position(|w| *w == "icmp") {
                Some(at) if words.len() >= at + 5 => {
                    constant(words[at + 3]) && constant(words[at + 4])
                }
                _ => false,
            }
        })
        .collect();
    assert!(found.is_empty(), "{} compares of two constants: {:?}", found.len(), found);
}
