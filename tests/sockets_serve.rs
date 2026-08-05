//! A program that serves itself.
//!
//! The server and the request are adjacent statements — one parallel group,
//! which is what kanso has instead of goroutines and a select. That only works
//! because `accept` and `run` are scheduling points: each answers "not yet" and
//! goes back in the queue, so the other statement runs and makes the progress
//! the first one is waiting for. Written as blocking syscalls, they deadlocked.

use std::process::Command;

const SERVER_AND_CLIENT: &str = r#"import "std/io"
import "std/net"

fn answered c
  net/read c . (_ -> net/write c (page "kanso")) . (_ -> net/close_conn c)

fn page body
  "HTTP/1.1 200 OK\r\ncontent-length: {length body}\r\n\r\n{body}"

fn said r
  print "the page says: {r.stdout}"

fn flags url
  ["-s" "--retry" "5" "--retry-connrefused" url]

net/listen 8137 . (l -> net/accept l . answered)
io/run "curl" (flags "http://127.0.0.1:8137/") . said
"#;

/// Watched red on both engines: killed at sixty seconds with no output, each
/// half waiting on the other.
#[test]
fn one_program_serves_itself_on_both_engines() {
    let dir = std::env::temp_dir().join("kanso-sockets-serve");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    let program = dir.join("serve.kso");
    std::fs::write(&program, SERVER_AND_CLIENT).expect("the program writes");

    for engine in [&[][..], &["--interp"][..]] {
        let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("play")
            .arg("serve.kso")
            .args(engine)
            .current_dir(&dir)
            .output()
            .expect("kanso binary runs");

        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "the page says: kanso\n",
            "engine {engine:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
