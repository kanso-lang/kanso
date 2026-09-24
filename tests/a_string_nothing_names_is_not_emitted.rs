//! A string constant the emitted module never reads is not in the module.
//!
//! A string is interned when the emitter reaches a literal or an err site, and
//! the function holding it may be pruned afterwards. On the codegen corpus 198
//! of 270 strings and 264 of their literal cells were named by nothing, 36% of
//! the module's bytes, and clang parsed and laid out every one.
//!
//! The program imports std/list and sums a list, so most of the library is
//! pruned. Every string and literal cell left in its module must be named
//! somewhere besides its own definition, and the binary must print what the
//! interpreter prints.
//!
//! Watched red with every interned string emitted: the module carried 328
//! strings and literal cells that nothing named.

use std::process::Command;

#[test]
fn every_string_left_in_the_module_is_named() {
    let dir = std::env::temp_dir().join(format!("kanso_unnamed_string_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let source = "import \"std/list\"\n\nprint \"sum {list/sum [3 1 2]}\"\n";
    std::fs::write(dir.join("main.kso"), source).expect("the program writes");
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
    assert_eq!(String::from_utf8_lossy(&native.stdout), "sum 6\n");
    let unnamed: Vec<&str> = module
        .lines()
        .filter_map(|line| line.strip_prefix('@')?.split_once(" = ").map(|(sym, _)| sym))
        .filter(|sym| {
            let digits = sym.strip_prefix('s').map(|rest| rest.trim_end_matches("_lit"));
            digits.is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
        })
        .filter(|sym| {
            let written = format!("@{sym}");
            module
                .match_indices(&written)
                .filter(|(at, _)| {
                    let after = module.as_bytes().get(at + written.len()).copied();
                    !after.is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
                })
                .count()
                < 2
        })
        .collect();
    assert!(unnamed.is_empty(), "{} strings nothing names: {:?}", unnamed.len(), unnamed);
}
