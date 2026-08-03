//! A loop written with `>>` runs out of stack, and the message should say why.
//!
//! `a . f` hands the continuation over as a closure, so nothing past the
//! current link exists until the link runs. `a >> b` takes `b` as an already
//! evaluated description, so building the first link requires evaluating the
//! second, which requires the third. The whole chain is constructed before any
//! of it runs, and the construction is what exhausts the stack.
//!
//! What a reader is told today is "recursion went deeper than the stack
//! holds", which points at the loop rather than at the operator that built it —
//! and the loop is the one part of the program that is fine.
//!
//! The hint has to be static to exist at all. The interpreter has its own frame
//! guard and can see the recursion; native only sees a SIGSEGV in a child and
//! translates it in the parent; wasm sees a trap nothing recorded. None of the
//! three can work out the cause at the moment it reports. A function that calls
//! itself in the right operand of `>>` is visible in the source, so all three
//! can say the same sentence, which is what the differential law asks for.

use std::process::Command;

fn ran(engine: &[&str]) -> String {
    let dir = std::env::temp_dir().join("kanso-seq-chain");
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    // Deep enough that native's real stack goes too, not only the
    // interpreter's ten-thousand-frame guard — otherwise the two engines
    // would be answering different questions.
    std::fs::write(
        dir.join("run.kso"),
        "import \"std/io\"\n\nfn step 400000\n  io/write \"done\\n\"\n\n\
         fn step n\n  io/write \"\" >> step (n + 1)\n\npub play = step 0\n",
    )
    .expect("the program writes");

    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg("run.kso")
        .args(engine)
        .current_dir(&dir)
        .output()
        .expect("kanso runs");
    let said = String::from_utf8_lossy(&done.stderr).into_owned();
    let _ = std::fs::remove_dir_all(&dir);
    said
}

/// Both engines name the operator and the function, in the same words.
#[test]
fn the_stack_message_names_the_operator_that_built_the_chain() {
    let native = ran(&[]);
    let interp = ran(&["--interp"]);

    assert!(
        native.contains("`>>`") && native.contains("step"),
        "native's message does not name the operator or the function: {native}"
    );
    assert_eq!(interp, native, "the engines disagree about a stack exhaustion");
}
