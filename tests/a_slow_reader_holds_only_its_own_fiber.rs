//! A client that stops reading holds its own fiber and no other.
//!
//! `net/write` wrote to a blocking socket and came back when every byte was
//! taken. A fiber waiting to read or to accept yields to the scheduler, and
//! a write did not: while one client's socket was full, no other fiber ran,
//! on either engine (#2407). A write the connection has no room for now
//! leaves its fiber holding the write and how much of it has gone, and the
//! scheduler waits on the socket for room.
//!
//! The server below answers "big" with twelve megabytes and anything else
//! with one word. The first client asks for the big answer and reads none
//! of it. It only looks at the first byte, which is how the test knows the
//! server is in the middle of that write. The second client then asks its
//! small question and has to be answered while the first still reads
//! nothing. Only after that does the first client read, and it has to get
//! every byte.
//!
//! Watched red with the yield taken out of both engines: natively the
//! second client's read timed out, because the server was inside the first
//! client's write, and the test stops there. The interpreter's half was
//! seen red through `tests/golden/micro/a_writer_waits_for_its_reader`,
//! which is the same fact with no client outside the program: the reader
//! the write waits for is a fiber of the same program, and a write that
//! does not yield never ends. That program was killed at twenty seconds on
//! each engine.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const SERVER: &str = r#"import "std/net"
import "std/os"

fn doubled s 0
  s

fn doubled s n
  doubled "{s}{s}" (n - 1)

fn answered c "big"
  net/write c (doubled "kanso " 21) .> (_ -> net/close_conn c)

fn answered c _
  net/write c "small" .> (_ -> net/close_conn c)

fn heard c
  net/read c .> (asked -> answered c asked)

fn forked l c left
  heard c
  served l left

fn served _ 0
  print "took both callers"

fn served l left
  net/accept l .> (c -> forked l c (left - 1))

fn serving_at l p
  os/write_file "port.txt" "{p}" .> (_ -> served l 2)

net/listen 0 .> (l -> net/port l .> (p -> serving_at l p))
"#;

/// What the big answer weighs: six bytes doubled twenty-one times.
const BIG: usize = 6 << 21;

fn two_callers(port: u16, engine: &[&str]) {
    let patience = Some(Duration::from_secs(20));
    let mut slow = TcpStream::connect(("127.0.0.1", port)).expect("the first caller connects");
    slow.set_read_timeout(patience).expect("a timeout sets");
    slow.write_all(b"big").expect("the first caller asks");
    // One byte looked at and left where it is: the server has begun the big
    // answer, and this caller has taken none of it.
    let seen = slow.peek(&mut [0u8; 1]).expect("the big answer begins");
    assert_eq!(seen, 1, "engine {engine:?}: the big answer begins");

    let mut quick = TcpStream::connect(("127.0.0.1", port)).expect("the second caller connects");
    quick.set_read_timeout(patience).expect("a timeout sets");
    quick.write_all(b"quick").expect("the second caller asks");
    let mut word = String::new();
    quick.read_to_string(&mut word).unwrap_or_else(|e| {
        panic!("engine {engine:?}: the second caller waited on the first caller's reading: {e}")
    });
    assert_eq!(word, "small", "engine {engine:?}: the second caller's answer");

    let mut all = Vec::new();
    slow.read_to_end(&mut all).expect("the first caller reads its answer");
    assert_eq!(all.len(), BIG, "engine {engine:?}: every byte of the big answer arrives");
    assert!(all.starts_with(b"kanso kanso "), "engine {engine:?}: the big answer's own bytes");
}

fn served(engine: &[&str]) -> String {
    let dir = std::env::temp_dir().join(format!("kanso-slow-reader-{}", engine.len()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    std::fs::write(dir.join("serve.kso"), SERVER).expect("the program writes");
    let mut child = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("play")
        .arg("serve.kso")
        .args(engine)
        .current_dir(&dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("kanso runs");
    // The port arrives by file, as in tests/sockets_serve.rs: a piped stdout
    // is block-buffered and would not show it until the program ended.
    let announced = dir.join("port.txt");
    let by = Instant::now() + Duration::from_secs(30);
    let port = loop {
        let read = std::fs::read_to_string(&announced).ok();
        if let Some(port) = read.and_then(|text| text.trim().parse::<u16>().ok()) {
            break port;
        }
        if Instant::now() > by {
            let _ = child.kill();
            panic!("engine {engine:?}: the program never announced its port");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    // A failed drive must not leave the server running behind the panic.
    let drove = std::panic::catch_unwind(|| two_callers(port, engine));
    let deadline = Instant::now() + Duration::from_secs(30);
    while drove.is_ok() && child.try_wait().expect("the child reports").is_none() {
        if Instant::now() > deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let _ = child.kill();
    let output = child.wait_with_output().expect("the child's output reads");
    let _ = std::fs::remove_dir_all(&dir);
    if let Err(why) = drove {
        std::panic::resume_unwind(why);
    }
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn the_second_caller_is_answered_while_the_first_reads_nothing() {
    assert_eq!(served(&[]), "took both callers\n", "natively");
    assert_eq!(served(&["--interp"]), "took both callers\n", "interpreted");
}
