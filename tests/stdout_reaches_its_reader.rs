//! A native program writes stdout through a buffer, and stderr goes straight
//! out. Two things followed from that and the interpreter does neither. Text
//! written to stdout before a write to stderr reached a shared pipe after it
//! (the lox port's error path). And a program killed while it waited, as a
//! server is, lost everything it had printed (httpd's access log). The buffer
//! is now emptied before the program writes to stderr, reports an error, or
//! waits.
#![cfg(unix)]

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn built(name: &str, source: &str) -> std::path::PathBuf {
    let work = std::env::temp_dir().join(format!("kanso-stdout-{name}"));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).expect("temp work dir");
    std::fs::write(work.join("main.kso"), source).expect("program writes");
    let status = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .args(["build", "main.kso"])
        .current_dir(&work)
        .status()
        .expect("kanso build runs");
    assert!(status.success(), "{name} builds");
    work.join("main")
}

#[test]
fn stdout_written_first_arrives_first() {
    let binary = built(
        "order",
        "import \"std/io\"\nimport \"std/os\"\n\n\
io/write \"first, on stdout\\n\"\n  .> (_ -> io/write_err \"second, on stderr\\n\")\n  .> (_ -> os/exit 1)\n",
    );
    // Both streams into one pipe, as `2>&1 | cat` makes them.
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("{} 2>&1 | cat", binary.display()))
        .output()
        .expect("the program runs");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "first, on stdout\nsecond, on stderr\n");
}

#[test]
fn a_runtime_error_follows_what_was_printed_before_it() {
    let binary = built(
        "die",
        r#"import "std/os"

print "before"
  .> (_ -> os/args)
  .> (a -> print "{put { 1:"a" } (2.5 + length a) "b"}")
"#,
    );
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("{} 2>&1 | cat", binary.display()))
        .output()
        .expect("the program runs");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.starts_with("before\nerror[runtime]:"), "got: {text}");
}

#[test]
fn a_program_killed_while_it_waits_keeps_what_it_printed() {
    let binary =
        built("killed", "import \"std/time\"\n\nprint \"started\" .> (_ -> time/sleep 60000)\n");
    let out = binary.with_file_name("out.txt");
    let file = std::fs::File::create(&out).expect("out.txt opens");
    let mut child =
        Command::new(&binary).stdout(Stdio::from(file)).spawn().expect("the program runs");
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut seen = String::new();
    while Instant::now() < deadline {
        seen = std::fs::read_to_string(&out).expect("out.txt reads");
        if !seen.is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    child.kill().expect("the program is killed");
    child.wait().expect("the program is reaped");
    assert_eq!(seen, "started\n");
}
