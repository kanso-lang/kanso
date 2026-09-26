//! A loop written as a chain of binds runs, on every engine.
//!
//! Each step is `io/write "" .> (_ -> step (n + 1))`, so the next link exists
//! only once the one before it has run and its callback has been called.
//!
//! Four hundred thousand links, which is far past what any stack holds. It is
//! deep on purpose: native builds each link in its own C frame unless the
//! executor walks the chain rather than recursing into it, so a shallower
//! fixture would pass with that walk removed.

use std::process::Command;

fn ran(engine: &[&str]) -> String {
    let dir = std::env::temp_dir().join("kanso-seq-chain");
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    std::fs::write(
        dir.join("run.kso"),
        "import \"std/io\"\n\nfn step 400000\n  io/write \"done\\n\"\n\n\
         fn step n\n  io/write \"\" .> (_ -> step (n + 1))\n\nstep 0\n",
    )
    .expect("the program writes");

    let done = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("play")
        .arg("run.kso")
        .args(engine)
        .current_dir(&dir)
        .output()
        .expect("kanso runs");
    let said = String::from_utf8_lossy(&done.stderr).into_owned();
    let _ = std::fs::remove_dir_all(&dir);
    said
}

/// Both engines run it to the end, in the same words.
#[test]
fn a_loop_written_as_a_chain_of_binds_runs_on_both_engines() {
    let native = ran(&[]);
    let interp = ran(&["--interp"]);

    assert_eq!(native, "", "native did not run the chain: {native}");
    assert_eq!(interp, native, "the engines disagree about a chain of binds");
}
