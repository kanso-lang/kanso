//! A release build runs a tail cycle whose arms differ in arity and in the
//! types of their parameters, four million hops deep, and prints what the
//! interpreter prints.
//!
//! On x86-64 with a clang that takes `preserve_nonecc`, `preserve_none_tails`
//! in src/main.rs rewrites such a cycle: every arm gets one signature of i64
//! words, padded to the widest, and every call flattens its arguments into
//! them. `ping` below is five words and `pong` six, and the words carry an
//! int, a float64 and a string, so a word dropped, swapped or left unpadded
//! shows up as a wrong sum or a crash. Four million hops overflow the stack
//! unless every one is a jump. On a host whose clang lacks the convention the
//! cycle keeps `tailcc`, and the spec checks that build instead.
//!
//! Watched red with the two halves of a flattened parameter put back in the
//! wrong order: the binary read a payload as a tag and stopped with
//! `no overload of \`cycle/ping\` matches these arguments`.

use std::process::Command;

const CYCLE: &str = "pub run = print (ping 1000000 0.0 \"done\")

fn ping n:int acc:float64 s
  if (n == 0) \"{acc} {s}\" (pong (n - 1) acc s 3)

fn pong n:int acc:float64 s k:int
  if (k == 0) (ping n acc s) (pong n (acc + 0.5) s (k - 1))
";

#[test]
fn four_million_hops_across_two_arities_agree_with_the_interpreter() {
    let dir = std::env::temp_dir().join(format!("kanso_tail_cycle_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("cycle")).expect("a module directory");
    std::fs::write(dir.join("cycle/cycle.kso"), CYCLE).expect("the library writes");
    std::fs::write(dir.join("main.kso"), "import \"./cycle\"\n\ncycle/run\n")
        .expect("the program writes");
    let build = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["build", "main.kso", "--release"])
        .current_dir(&dir)
        .output()
        .expect("kanso runs");
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let native = Command::new(dir.join("main")).output().expect("the binary runs");
    let oracle = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["run", "main.kso", "--interp"])
        .current_dir(&dir)
        .output()
        .expect("the interpreter runs");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(native.status.success(), "the release binary failed: {:?}", native.status);
    assert_eq!(String::from_utf8_lossy(&oracle.stdout), "1500000.0 done\n");
    assert_eq!(native.stdout, oracle.stdout, "the engines disagree");
}
