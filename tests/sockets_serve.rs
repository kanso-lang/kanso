//! A program that serves itself.
//!
//! The server and the request are adjacent statements — one parallel group,
//! which is what kanso has instead of goroutines and a select. That only works
//! because `accept` and `run` are scheduling points: each answers "not yet" and
//! goes back in the queue, so the other statement runs and makes the progress
//! the first one is waiting for. Written as blocking syscalls, they deadlocked.
//!
//! The port is chosen per run and the program closes its listener. A pinned
//! port made this test wedge every full suite on the machine: `curl` reaches
//! whichever process holds the port, so a leftover server from an earlier run
//! answers the request and the fresh one waits on an `accept` that never
//! arrives — for a day, in the case that turned this up. The runtime's own
//! `listen` says so: "a test that pinned a port would collide with whatever
//! else is running on the machine."

use std::process::{Command, Stdio};

const SERVER_AND_CLIENT: &str = r#"import "std/io"
import "std/net"

fn answered l c
  net/read c
    . (_ -> net/write c (page "kanso"))
    . (_ -> net/close_conn c)
    . (_ -> net/close_listener l)

fn page body
  "HTTP/1.1 200 OK\r\ncontent-length: {length body}\r\n\r\n{body}"

fn said r
  print "the page says: {r.stdout}"

url = "http://127.0.0.1:PORT/"

flags = ["-s" "--retry" "5" "--retry-connrefused" url]

net/listen PORT . (l -> net/accept l . (c -> answered l c))
io/run "curl" flags . said
"#;

/// The shape every browser harness needs, and the one thing the socket test
/// above does not cover: many requests, state carried between them, and a
/// report that reaches the program. A smoke test decides pass or fail from
/// what the page posts back, so a server that answers `none` when it stops
/// leaves the harness with nothing to assert on.
const SERVE_UNTIL_A_REPORT: &str = r#"import "std/io"
import "std/net/http"

fn handled req carried
  answered req.path req carried

fn answered "/report" req _
  http/turn (http/ok "thanks") (http/done req.body)

fn answered _ _ carried
  http/turn (http/ok "the page") carried

url = "http://127.0.0.1:PORT"

get = ["-s" "--retry" "5" "--retry-connrefused" "{url}/"]

post = ["-s" "-d" "green" "{url}/report"]

http/serve_until PORT handled "open" . (r -> print "the report says: {r}")
io/run "curl" get . (_ -> io/run "curl" post)
"#;

/// A port nothing else on the machine is using, varied per engine so the two
/// halves of this test cannot collide with each other either.
fn free_port(offset: u16) -> u16 {
    let taken = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("the os hands out a port");
    let port = taken.local_addr().expect("a bound listener has an address").port();
    drop(taken);
    port.wrapping_add(offset).max(1024)
}

/// Runs one program on one engine and answers what it printed.
fn printed(name: &str, source: &str, engine: &[&str], offset: u16) -> String {
    let dir = std::env::temp_dir().join(format!("kanso-{name}-{offset}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    let source = source.replace("PORT", &free_port(offset).to_string());
    std::fs::write(dir.join("serve.kso"), source).expect("the program writes");

    let mut child = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("play")
        .arg("serve.kso")
        .args(engine)
        .current_dir(&dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("kanso binary runs");

    // A wedged run must fail rather than hang: this test hung the whole suite
    // for a day, and a suite that never finishes reports nothing.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let finished = loop {
        match child.try_wait().expect("the child reports its state") {
            Some(_) => break true,
            None if std::time::Instant::now() > deadline => break false,
            None => std::thread::sleep(std::time::Duration::from_millis(50)),
        }
    };
    if !finished {
        let _ = child.kill();
        let _ = child.wait();
        panic!("engine {engine:?}: the program did not finish within thirty seconds");
    }

    let output = child.wait_with_output().expect("the child's output reads");
    assert!(
        output.status.success(),
        "engine {engine:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Watched red on both engines: killed at sixty seconds with no output, each
/// half waiting on the other.
#[test]
fn one_program_serves_itself_on_both_engines() {
    for (offset, engine) in [(0u16, &[][..]), (1, &["--interp"][..])] {
        let said = printed("sockets-serve", SERVER_AND_CLIENT, engine, offset);
        assert_eq!(said, "the page says: kanso\n", "engine {engine:?}");
    }
}

/// Watched red on both engines: the server answered `<none>`, because the loop
/// stopped on a carried `none` and threw away what the page had posted.
#[test]
fn a_served_report_reaches_the_program_on_both_engines() {
    for (offset, engine) in [(2u16, &[][..]), (3, &["--interp"][..])] {
        let said = printed("sockets-report", SERVE_UNTIL_A_REPORT, engine, offset);
        assert_eq!(said, "the report says: green\n", "engine {engine:?}");
    }
}
