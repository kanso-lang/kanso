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

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

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

/// A browser opens speculative connections and sends nothing down them, and
/// this server has to survive being spoken to and told nothing. Nothing in
/// kanso can open a bare socket — `net` has no `connect` — so the test drives
/// this one itself, and the program keeps a second statement running to give
/// the scheduler something to do while it waits.
const A_CONNECTION_THAT_SAYS_NOTHING: &str = r#"import "std/io"
import "std/net/http"

fn handled req carried
  answered req.path req carried

fn answered "/report" req _
  http/turn (http/ok "thanks") (http/done req.body)

fn answered _ _ carried
  http/turn (http/ok "the page") carried

http/serve_until PORT handled "open" . (r -> print "the report says: {r}")
io/run "sleep" ["3"] . (_ -> print "waited")
"#;

/// A port nothing else on the machine is using. The os chooses it, and this
/// only declines to hand the same one out twice, because two halves of one run
/// that land on one port wedge each other.
///
/// Offsetting a free port was the old way of keeping them apart, and it lands
/// on a port nobody ever checked: "cannot listen on that port" reached CI on
/// the macos host as soon as a third test made the collision likely.
fn free_port() -> u16 {
    static ISSUED: Mutex<Vec<u16>> = Mutex::new(Vec::new());
    for _ in 0..100 {
        let taken = TcpListener::bind(("127.0.0.1", 0)).expect("the os hands out a port");
        let port = taken.local_addr().expect("a bound listener has an address").port();
        drop(taken);
        let mut issued = ISSUED.lock().expect("the ports handed out so far");
        if !issued.contains(&port) {
            issued.push(port);
            return port;
        }
    }
    panic!("the os kept offering ports this run had already used");
}

/// Runs one program on one engine and answers what it printed. `drive` runs
/// against the port once the program is up, for the tests whose client is this
/// harness rather than a statement of the program itself. `slot` only keeps
/// each engine's run in a directory of its own.
fn printed(
    name: &str,
    source: &str,
    engine: &[&str],
    slot: u16,
    drive: impl FnOnce(u16),
) -> String {
    // One server at a time. The port is chosen by asking the os and letting
    // go, so anything that binds between the asking and the program's own
    // listen takes it — and the likeliest such thing is this file's other
    // tests. Serialising removes the contention this suite inflicts on
    // itself; it does not remove the race against the rest of the machine,
    // which wants `listen 0` and a program that reports the port it got.
    static SERIAL: Mutex<()> = Mutex::new(());
    let _one_at_a_time = SERIAL.lock().unwrap_or_else(|e| e.into_inner());

    let dir = std::env::temp_dir().join(format!("kanso-{name}-{slot}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    let port = free_port();
    let source = source.replace("PORT", &port.to_string());
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

    drive(port);

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
    for (slot, engine) in [(0u16, &[][..]), (1, &["--interp"][..])] {
        let said = printed("sockets-serve", SERVER_AND_CLIENT, engine, slot, |_| {});
        assert_eq!(said, "the page says: kanso\n", "engine {engine:?}");
    }
}

/// Connects once and closes without sending a byte, the way a browser's
/// speculative connection does, then asks for something real. Watched red on
/// both engines: `error[endpoint]: unhandled err reached the executor:
/// "missing index 2"`, born in `http/parsed`.
fn a_silent_visitor_then_a_report(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(15);
    let silent = loop {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(open) => break open,
            Err(e) if Instant::now() > deadline => panic!("the door never opened: {e}"),
            Err(_) => std::thread::sleep(Duration::from_millis(20)),
        }
    };
    drop(silent);

    // Bounded on purpose. A dead server accepts nothing, so the connection
    // stays open and unanswered, and an unbounded read here blocks before the
    // deadline that is supposed to catch a wedged run ever gets a turn.
    let mut asking = TcpStream::connect(("127.0.0.1", port)).expect("the server still answers");
    asking
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("a read that cannot outlast the test");
    asking
        .write_all(b"POST /report HTTP/1.1\r\nhost: x\r\ncontent-length: 5\r\n\r\ngreen")
        .expect("the request goes out");
    let mut answer = String::new();
    let _ = asking.read_to_string(&mut answer);
}

/// Sends a head and then the body in two writes with a pause between them,
/// which is what a report large enough to cross a segment boundary looks like
/// from the server's side. Watched red on both engines: the handler saw only
/// the first chunk, and the browser differential's own json decode said
/// "unexpected end of input" on exactly this in CI.
fn a_report_that_arrives_in_two_chunks(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut asking = loop {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(open) => break open,
            Err(e) if Instant::now() > deadline => panic!("the door never opened: {e}"),
            Err(_) => std::thread::sleep(Duration::from_millis(20)),
        }
    };
    asking
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("a read that cannot outlast the test");
    asking
        .write_all(b"POST /report HTTP/1.1\r\nhost: x\r\nContent-Length: 10\r\n\r\ngre")
        .expect("the head and the first chunk go out");
    asking.flush().expect("the first chunk is not held back");
    std::thread::sleep(Duration::from_millis(300));
    asking.write_all(b"enenough").expect("the rest of the body goes out");
    let mut answer = String::new();
    let _ = asking.read_to_string(&mut answer);
}

#[test]
fn a_body_split_across_two_reads_arrives_whole_on_both_engines() {
    for (slot, engine) in [(6u16, &[][..]), (7, &["--interp"][..])] {
        let said = printed(
            "sockets-chunked",
            A_CONNECTION_THAT_SAYS_NOTHING,
            engine,
            slot,
            a_report_that_arrives_in_two_chunks,
        );
        assert_eq!(said, "the report says: greenenough\nwaited\n", "engine {engine:?}");
    }
}

#[test]
fn a_silent_connection_does_not_end_the_server_on_both_engines() {
    for (slot, engine) in [(4u16, &[][..]), (5, &["--interp"][..])] {
        let said = printed(
            "sockets-silent",
            A_CONNECTION_THAT_SAYS_NOTHING,
            engine,
            slot,
            a_silent_visitor_then_a_report,
        );
        assert_eq!(said, "the report says: green\nwaited\n", "engine {engine:?}");
    }
}

/// Watched red on both engines: the server answered `<none>`, because the loop
/// stopped on a carried `none` and threw away what the page had posted.
#[test]
fn a_served_report_reaches_the_program_on_both_engines() {
    for (slot, engine) in [(2u16, &[][..]), (3, &["--interp"][..])] {
        let said = printed("sockets-report", SERVE_UNTIL_A_REPORT, engine, slot, |_| {});
        assert_eq!(said, "the report says: green\n", "engine {engine:?}");
    }
}
