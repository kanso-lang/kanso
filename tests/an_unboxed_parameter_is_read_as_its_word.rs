//! A parameter that crosses as a raw i64 is read as that i64.
//!
//! An integer parameter the escape analysis unboxes arrives as `i64 %xNr`
//! and is boxed on entry. Every read of its tag or payload used to take the
//! box back apart with an `extractvalue`, though the tag is 0 and the payload
//! is the argument. Those reads, and the boxes nothing else read, were 195 of
//! the decoder's emitted lines.
//!
//! The program compares and passes on two unboxed integers. No function in
//! its module may extract a word from a boxed unboxed parameter, and the
//! binary must print what the interpreter prints.
//!
//! Watched red with the words left unrecorded: nine reads took a parameter
//! back apart.

use std::process::Command;

const MAIN: &str = "import \"./counted\"\n\nplay\n";

const LIBRARY: &str = "pub play = print \"sum {climbed 1 20 0}\"

fn climbed i stop acc
  reached i stop acc (i > stop)

fn reached _ _ acc true
  acc

fn reached i stop acc false
  climbed (i + 1) stop (acc + i)
";

#[test]
fn no_unboxed_parameter_is_taken_back_apart() {
    let dir = std::env::temp_dir().join(format!("kanso_unboxed_word_{}", std::process::id()));
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
    assert_eq!(String::from_utf8_lossy(&native.stdout), "sum 210\n");
    let mut unboxed: Vec<String> = Vec::new();
    let mut found: Vec<String> = Vec::new();
    for line in module.lines() {
        if line.starts_with("define ") {
            unboxed = line
                .split("i64 %x")
                .skip(1)
                .filter_map(|rest| rest.split_once('r').map(|(n, _)| n.to_string()))
                .filter(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
                .collect();
            continue;
        }
        for n in &unboxed {
            if line.contains(&format!("extractvalue %KValue %x{n},")) {
                found.push(line.trim().to_string());
            }
        }
    }
    assert!(found.is_empty(), "{} reads took a parameter apart: {:?}", found.len(), found);
}
