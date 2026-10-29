//! A fiber waiting in `net/read` lets the others run (#2208).
//!
//! A program that is both ends of a connection dials itself in one statement
//! and accepts in the next. The client's read used to block the runtime before
//! the server's accept and write could run, so neither line printed and the
//! program never ended. A read with nothing arrived now goes back in the queue,
//! as `accept` already did. Each engine runs the program under a deadline,
//! because the failure is a hang.

use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn program(read: &str) -> String {
    format!(
        "import \"std/net\"\n\n\
         fn both l port\n  \
         net/dial \"127.0.0.1\" port .> (c -> net/{read} c) .> (_ -> print \"read\")\n  \
         net/accept l .> (c -> net/write c \"hello\") .> (_ -> print \"wrote\")\n\n\
         net/listen 0 .> (l -> net/port l .> (p -> both l p))\n"
    )
}

/// What the program printed, or `None` when it was still running after ten
/// seconds and had to be killed.
fn ran(tag: &str, source: &str, interp: bool) -> Option<String> {
    let dir = std::env::temp_dir().join(format!("kanso-both-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    std::fs::write(dir.join("both.kso"), source).expect("the program writes");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kanso"));
    // Its own process group, so a hang is killed whole: `kanso play` runs the
    // program it built as a child, and that child would outlive a kill of
    // `kanso` alone, holding the streams it inherited open.
    cmd.arg("play")
        .arg("both.kso")
        .current_dir(&dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0);
    if interp {
        cmd.arg("--interp");
    }
    let mut child = cmd.spawn().expect("kanso runs");
    let deadline = Instant::now() + Duration::from_secs(10);
    while child.try_wait().expect("the child can be asked").is_none() {
        if Instant::now() > deadline {
            let group = format!("-{}", child.id());
            let _ = Command::new("kill").args(["-KILL", "--", &group]).status();
            let _ = child.wait();
            let _ = std::fs::remove_dir_all(&dir);
            return None;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let done = child.wait_with_output().expect("the output reads");
    let _ = std::fs::remove_dir_all(&dir);
    Some(String::from_utf8_lossy(&done.stdout).into_owned())
}

#[test]
fn a_program_can_be_both_ends_of_a_connection() {
    for read in ["read", "read_bytes"] {
        for (engine, interp) in [("native", false), ("interp", true)] {
            let tag = format!("{read}-{engine}");
            let printed = ran(&tag, &program(read), interp);
            assert_eq!(
                printed.as_deref(),
                Some("wrote\nread\n"),
                "{tag}: None is a program that never finished"
            );
        }
    }
}
