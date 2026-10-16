//! A recursion deeper than the stack ends in a fault on the guard page.
//! `kanso run` explains its child's segmentation fault as running out of
//! stack, but a binary run directly died with the bare signal and printed
//! nothing, and the lox port is run that way. The runtime now answers a fault
//! inside the main stack's reach with the interpreter's diagnostic.
#![cfg(unix)]

use std::process::Command;

/// A chain built a million deep and measured by a recursion that is not a
/// tail call, so no engine can turn it into a loop.
const PROGRAM: &str = "\
type node
  value
  next

fn chain 0
  \"end\"

fn chain n
  node n (chain (n - 1))

fn size (node _ rest)
  1 + size rest

fn size _
  0

pub play = print \"{size (chain 1000000)}\"
";

#[test]
fn a_built_binary_run_directly_says_it_ran_out_of_stack() {
    let work = std::env::temp_dir().join("kanso-binary-stack-overflow");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).expect("temp work dir");
    std::fs::write(work.join("deep.kso"), PROGRAM).expect("program writes");
    std::fs::write(work.join("main.kso"), "import \"./deep\"\n\ndeep/play\n")
        .expect("entry writes");
    let built = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["build", "main.kso"])
        .current_dir(&work)
        .output()
        .expect("kanso builds");
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));

    let output = Command::new(work.join("main")).current_dir(&work).output().expect("binary runs");

    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "error[runtime]: the program ran out of stack: recursion went deeper than the stack holds\n"
    );
    assert_eq!(output.status.code(), Some(1), "{:?}", output.status);
}
