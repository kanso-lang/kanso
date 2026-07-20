//! The register-return ABI at its boundaries: a construction crossing into a
//! destructuring callee must arrive in the by-value convention from every
//! position, and a type whose first field is not an int must stay boxed —
//! the packed convention shifts field 0's payload into the tag word, which
//! is only sound for an int.

use std::process::Command;

fn run(source: &str) -> String {
    let dir = std::env::temp_dir().join(format!("kanso_regreturn_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("prog.kso");
    std::fs::write(&path, source).expect("write fixture");
    let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg("prog.kso")
        .current_dir(&dir)
        .output()
        .expect("kanso binary runs");
    assert!(
        output.status.success(),
        "run failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn construction_in_tail_call_argument_reaches_the_destructuring_callee() {
    let out = run(
        "type user\n  age:int\n  name:string\n\nfn foo (user age name)\n  \
         print \"{name} is age {age}\"\n\nmain = foo (user 44 \"clay\")\n",
    );

    assert_eq!(out, "clay is age 44\n");
}

#[test]
fn construction_bound_then_passed_reaches_the_destructuring_callee() {
    let out = run(
        "type user\n  age:int\n  name:string\n\nfn foo (user age name)\n  \
         \"{name}/{age}\"\n\nmain =\n  a = foo (user 1 \"x\")\n  b = foo (user 2 \"y\")\n  \
         print \"{a} {b}\"\n",
    );

    assert_eq!(out, "x/1 y/2\n");
}

#[test]
fn string_first_type_stays_boxed_and_correct() {
    let out = run(
        "type tag\n  label:string\n  weight:int\n\nmain = show (tag \"hot\" 9)\n\n\
         fn show (tag label weight)\n  print \"{label}:{weight}\"\n",
    );

    assert_eq!(out, "hot:9\n");
}
