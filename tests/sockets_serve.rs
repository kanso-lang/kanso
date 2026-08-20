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
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const SERVER_AND_CLIENT: &str = r#"import "std/io"
import "std/net"
import "std/time"

fn answered l c
  net/read c
    . (_ -> net/write c (page "kanso"))
    . (_ -> net/close_conn c)
    . (_ -> net/close_listener l)

fn page body
  "HTTP/1.1 200 OK\r\ncontent-length: {length body}\r\n\r\n{body}"

fn said r
  print "the page says: {r.stdout}"

fn serving_at l p
  io/write_file "port.txt" "{p}" . (_ -> net/accept l) . (c -> answered l c)

asked = time/sleep 400 . (_ -> io/read_file "port.txt") . fetched

fn fetched p
  url = "http://127.0.0.1:{p}/"
  io/run "curl" ["-s" "--retry" "5" "--retry-connrefused" url] . said

net/listen 0 . (l -> net/port l . (p -> serving_at l p))
asked
"#;

/// The shape every browser harness needs, and the one thing the socket test
/// above does not cover: many requests, state carried between them, and a
/// report that reaches the program. A smoke test decides pass or fail from
/// what the page posts back, so a server that answers `none` when it stops
/// leaves the harness with nothing to assert on.
const SERVE_UNTIL_A_REPORT: &str = r#"import "std/io"
import "std/net"
import "std/net/http"
import "std/time"

fn handled req carried
  answered req.path req carried

fn answered "/report" req _
  http/reply (http/ok "thanks") (http/stop req.body)

fn answered _ _ carried
  http/reply (http/ok "the page") carried

fn serving_at l p
  io/write_file "port.txt" "{p}"
    . (_ -> http/serving l handled "open")
    . (r -> print "the report says: {r}")

asked = time/sleep 400 . (_ -> io/read_file "port.txt") . visited

fn visited p
  url = "http://127.0.0.1:{p}"
  get = ["-s" "--retry" "5" "--retry-connrefused" "{url}/"]
  post = ["-s" "-d" "green" "{url}/report"]
  io/run "curl" get . (_ -> io/run "curl" post)

net/listen 0 . (l -> net/port l . (p -> serving_at l p))
asked
"#;

/// A browser opens speculative connections and sends nothing down them, and
/// this server has to survive being spoken to and told nothing. Nothing in
/// kanso can open a bare socket — `net` has no `connect` — so the test drives
/// this one itself, and the program keeps a second statement running to give
/// the scheduler something to do while it waits.
const A_CONNECTION_THAT_SAYS_NOTHING: &str = r#"import "std/io"
import "std/net"
import "std/net/http"

fn handled req carried
  answered req.path req carried

fn answered "/report" req _
  http/reply (http/ok "thanks") (http/stop req.body)

fn answered _ _ carried
  http/reply (http/ok "the page") carried

fn serving_at l p
  io/write_file "port.txt" "{p}"
    . (_ -> http/serving l handled "open")
    . (r -> print "the report says: {r}")

net/listen 0 . (l -> net/port l . (p -> serving_at l p))
io/run "sleep" ["3"] . (_ -> print "waited")
"#;

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

    // The program binds 0 and writes back what it was given, so nothing is
    // bound, read and dropped — there is no gap for anything else to take.
    // It must arrive by file: a piped stdout is block-buffered, so a port
    // announced there does not appear until the program exits, and the program
    // will not exit until this drive has run. That circular wait is the
    // ten-minute hang this file used to carry a note about.
    let announced = dir.join("port.txt");
    let by = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let port = loop {
        if let Ok(text) = std::fs::read_to_string(&announced) {
            if let Ok(n) = text.trim().parse::<u16>() {
                break n;
            }
        }
        if std::time::Instant::now() > by {
            let _ = child.kill();
            panic!("engine {engine:?}: the program never announced its port");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
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

/// The port a listener was actually given, read back off the listener itself.
///
/// The assertion is not that the number is positive — it is that a client
/// reaches the listener AT that number, which is what makes it the bound port
/// rather than a plausible integer.
///
/// That closes the gap `free_port` leaves: bind 0, read the port, drop the
/// listener, hand the number on, and whatever binds in between takes it. A
/// program reading the port off the listener it already holds never lets go.
///
/// Two things this shape is working around, both learned by watching it fail.
/// The program is held open by a sleep rather than an `accept`, because a lone
/// statement waiting on accept has no sibling fiber to yield to: the scheduler
/// calls it deadlocked, the program exits, and the socket closes before any
/// client arrives. And the port travels by file rather than stdout, because a
/// piped stdout is block-buffered — the line does not appear until the process
/// ends, by which time there is nothing to connect to.
#[test]
fn a_listener_answers_the_port_it_was_given_on_both_engines() {
    const ANNOUNCE: &str = "import \"std/io\"\nimport \"std/net\"\nimport \"std/time\"\n\npub fn announced l p\n  io/write_file \"port.txt\" \"{p}\"\n    . (_ -> time/sleep 3000)\n    . (_ -> net/close_listener l)\n";
    const ENTRY: &str = "import \"./announce\"\nimport \"std/net\"\n\nnet/listen 0 . (l -> net/port l . (p -> announce/announced l p))\n";

    for (tag, engine) in [("native", &[][..]), ("interp", &["--interp"][..])] {
        let dir = std::env::temp_dir().join(format!("kanso-netport-{}-{tag}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).expect("the scratch directory is made");
        std::fs::write(dir.join("announce.kso"), ANNOUNCE).expect("the library is written");
        std::fs::write(dir.join("main.kso"), ENTRY).expect("the entry is written");

        let mut child = Command::new(env!("CARGO_BIN_EXE_kanso"))
            .arg("run")
            .arg(".")
            .args(engine)
            .current_dir(&dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("the program starts");

        let announced = dir.join("port.txt");
        let deadline = Instant::now() + Duration::from_secs(30);
        let port = loop {
            if let Ok(text) = std::fs::read_to_string(&announced) {
                if let Ok(n) = text.trim().parse::<u16>() {
                    break Some(n);
                }
            }
            if Instant::now() > deadline {
                break None;
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let port = port.unwrap_or_else(|| panic!("no port announced on {tag}"));

        // Only the bound port has the listener behind it.
        let reached = TcpStream::connect(("127.0.0.1", port));
        let _ = child.kill();
        let _ = child.wait();
        std::fs::remove_dir_all(&dir).ok();
        assert!(reached.is_ok(), "nothing listening on {port} for {tag}");
    }
}
