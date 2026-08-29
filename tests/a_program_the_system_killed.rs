//! A native program the operating system kills has no exit code, and the
//! driver says which signal ended it rather than failing in silence.
//!
//! This cannot live in the runtime corpus. That corpus asserts the native
//! binary and the interpreter oracle write identical stderr and both exit 1;
//! a signalled program exits neither 1 nor the same way under `--interp`,
//! where there is no second process to signal. So the pin lives here.
//!
//! SIGSEGV has its own message and its own fixture (deep_recursion). This is
//! the other arm: any signal but that one, named by number.
#![cfg(unix)]

use std::process::Command;

// The `sleep` is not padding. `os/run` returns when the shell exits, and the
// signal is delivered asynchronously, so a shell that killed and exited at
// once leaves a race the program could win — printing before it dies, on a
// slower runner or a different kernel. Keeping the shell alive holds the
// program inside `os/run` until the signal has landed.
const KILLS_ITSELF: &str = r#"import "std/os"

os/run "sh" ["-c" "kill -TERM $PPID; sleep 5"] . (_ -> print "survived")
"#;

#[test]
fn a_program_the_system_killed_names_the_signal() {
    let dir = std::env::temp_dir().join("kanso-killed-by-signal");
    let _ = std::fs::create_dir_all(&dir);
    let file = dir.join("killed.kso");
    std::fs::write(&file, KILLS_ITSELF).expect("fixture writes");
    let out = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("run")
        .arg("killed.kso")
        .current_dir(&dir)
        .output()
        .expect("kanso runs");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("error[runtime]: the program was ended by signal 15"),
        "expected the signal to be named, got: {err}"
    );
    assert!(
        !String::from_utf8_lossy(&out.stdout).contains("survived"),
        "the program ran on past its own death: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
