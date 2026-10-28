//! A program that opens a connection rather than waiting for one.
//!
//! `net/dial` is how a client written in kanso reaches a server, such as the
//! Postgres client senro needs (#2178). This harness plays the server: it
//! listens on a port the operating system picks, takes the program's greeting,
//! and answers with bytes framed the way a Postgres message is framed. The
//! answer includes 0xff, which is not text, so a read that decoded it would
//! show up as a changed byte or a refusal. `net/read_bytes` hands it back as it
//! arrived.
//!
//! Both engines that have sockets run the same program and must print the same
//! bytes. A dial that nothing answers fails with a reason naming the address,
//! and the two engines word it alike.
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::time::Duration;

const ANSWER: [u8; 9] = [b'R', 0, 0, 0, 8, 0, 0, 0xff, 0x0a];

fn program(port: u16) -> String {
    format!(
        "import \"std/net\"\n\n\
         fn spoken c\n  \
         net/write c \"hello\"\n    \
         .> (_ -> net/read_bytes c)\n    \
         .> (b -> print \"{{length b}} bytes: {{b}}\")\n    \
         .> (_ -> net/close_conn c)\n\n\
         net/dial \"localhost\" {port} .> spoken\n"
    )
}

/// Runs `source` on one engine from a directory of its own and answers what it
/// printed and what it said on stderr.
fn ran(tag: &str, source: &str, interp: bool) -> (String, String) {
    let dir = std::env::temp_dir().join(format!("kanso-dial-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    std::fs::write(dir.join("dial.kso"), source).expect("the program writes");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_kanso"));
    cmd.arg("play").arg("dial.kso").current_dir(&dir);
    if interp {
        cmd.arg("--interp");
    }
    let done = cmd.output().expect("kanso runs");
    let _ = std::fs::remove_dir_all(&dir);
    (
        String::from_utf8_lossy(&done.stdout).into_owned(),
        String::from_utf8_lossy(&done.stderr).into_owned(),
    )
}

/// Listens, answers one connection with `ANSWER`, and hands back what the
/// program said before it was answered.
fn serve_once() -> (u16, std::thread::JoinHandle<Vec<u8>>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("a port to listen on");
    let port = listener.local_addr().expect("the port it was given").port();
    let served = std::thread::spawn(move || {
        let (mut conn, _) = listener.accept().expect("the program connects");
        conn.set_read_timeout(Some(Duration::from_secs(10))).expect("a bounded read");
        let mut heard = [0u8; 5];
        conn.read_exact(&mut heard).expect("the program says hello");
        conn.write_all(&ANSWER).expect("the answer goes out");
        heard.to_vec()
    });
    (port, served)
}

#[test]
fn a_program_dials_a_server_and_reads_its_bytes_on_both_engines() {
    for (tag, interp) in [("native", false), ("interp", true)] {
        let (port, served) = serve_once();
        let (out, err) = ran(tag, &program(port), interp);
        let heard = served.join().expect("the server thread finishes");
        assert_eq!(heard, b"hello", "{tag}: the server heard something else");
        assert_eq!(out, "9 bytes: [82 0 0 0 8 0 0 255 10]\n", "{tag}: {err}");
    }
}

#[test]
fn a_dial_that_nothing_answers_names_the_address_on_both_engines() {
    // A port that was bound and let go. Something else could take it in the
    // gap, but nothing in this suite listens on a port it did not ask for.
    let port = {
        let held = TcpListener::bind(("127.0.0.1", 0)).expect("a port to borrow");
        held.local_addr().expect("the port it was given").port()
    };
    let source = format!("import \"std/net\"\n\nnet/dial \"127.0.0.1\" {port}\n");
    let want = format!(
        "error[endpoint]: unhandled err reached the executor: \"cannot connect to 127.0.0.1:{port}\"\n"
    );
    for (tag, interp) in [("native-refused", false), ("interp-refused", true)] {
        let (_, err) = ran(tag, &source, interp);
        assert_eq!(err, want, "{tag}");
    }
}
