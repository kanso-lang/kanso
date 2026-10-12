//! A reader that stops early, as `head` does, closes the pipe a program is
//! writing to. A built binary is ended by SIGPIPE and says nothing. The
//! interpreter panicked on the failed write and printed two Rust backtraces,
//! and `kanso play` reported the native program's end as "ended by signal 13".
//! Each engine now stops the way the binary does: by SIGPIPE, with nothing on
//! stderr.
#![cfg(unix)]

use std::io::{BufRead, BufReader};
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

const SIGPIPE: i32 = 13;

/// More than a pipe buffer holds, so the program is still writing when the
/// reader goes away.
const PROGRAM: &str = "import \"std/list\"\nimport \"std/text\"\n\n\
lines = list/to_list (list/map (list/take list/naturals 50000) (n -> \"{n}\"))\n\n\
print (text/join lines \"\\n\")\n";

fn stopped_by_the_reader(engine: &[&str]) {
    let work = std::env::temp_dir().join(format!("kanso-closed-pipe-{}", engine.len()));
    std::fs::create_dir_all(&work).expect("temp work dir");
    let program = work.join("many_lines.kso");
    std::fs::write(&program, PROGRAM).expect("program writes");

    let mut child = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("play")
        .arg(&program)
        .args(engine)
        .current_dir(&work)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("kanso runs");
    let mut first = String::new();
    BufReader::new(child.stdout.take().expect("stdout is piped"))
        .read_line(&mut first)
        .expect("one line reads");
    assert_eq!(first, "1\n", "{engine:?} printed the first line");
    // The reader is dropped here, which closes the pipe.
    let output = child.wait_with_output().expect("kanso finishes");

    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "",
        "{engine:?} says nothing when its reader stops"
    );
    assert_eq!(
        output.status.signal(),
        Some(SIGPIPE),
        "{engine:?} ends by SIGPIPE: {:?}",
        output.status
    );
}

#[test]
fn the_interpreter_stops_quietly_when_its_reader_does() {
    stopped_by_the_reader(&["--interp"]);
}

#[test]
fn a_native_program_stops_quietly_when_its_reader_does() {
    stopped_by_the_reader(&[]);
}
