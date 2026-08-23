//! A started process can be ended.
//!
//! `os/run` waits, which is what a script wants for `git rev-parse` and what
//! it cannot want for a browser: headless chrome ignores its own exit budget
//! and runs until something outside kills it. So a program needs the handle
//! and a way to end what it holds — Go's `cmd.Start()` and `Process.Kill()`.
//!
//! The observable is a side effect the child would have had. It writes a file
//! two seconds in; the program kills it, waits three, and asks whether the
//! file arrived. Killed, it never does.

use std::process::Command;

const START_AND_KILL: &str = r#"import "std/os"
import "std/time"

fn asked _
  os/exists "late" . (there -> print "the late file arrived: {there}")

os/start "sh" ["-c" "sleep 2 && echo late > late"]
  . os/kill
  . (_ -> time/sleep 3000)
  . asked
"#;

fn ran(engine: &[&str], dir: &std::path::Path) -> String {
    let program = dir.join("kill.kso");
    std::fs::write(&program, START_AND_KILL).expect("the program writes");

    let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("play")
        .arg("kill.kso")
        .args(engine)
        .current_dir(dir)
        .output()
        .expect("kanso binary runs");

    String::from_utf8_lossy(&output.stdout).into_owned() + &String::from_utf8_lossy(&output.stderr)
}

/// Watched red on both engines: `unknown name start`.
#[test]
fn a_killed_process_never_finishes_its_work() {
    for engine in [&[][..], &["--interp"][..]] {
        let dir = std::env::temp_dir().join(format!("kanso-kill-{}", engine.len()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a directory to run in");

        assert_eq!(ran(engine, &dir), "the late file arrived: false\n", "engine {engine:?}");
    }
}
