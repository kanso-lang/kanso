//! A program that opens more than the handle table holds, one at a time.
//!
//! A socket and a process are named by `h % K_SOCKETS`, and closing one frees
//! its slot — but the guard used to compare how many had EVER been taken
//! against the size of the table. So the sixty-fourth open of a program's life
//! failed however many it had already closed, and said "at once" about a table
//! holding nothing.
//!
//! A server spends two a turn: one for the connection it accepts, and one for
//! the `run` in the statement beside it, which forks rather than blocking
//! because it has to yield. Forty requests is eighty, comfortably past the
//! sixty-four the old guard allowed.

use std::process::Command;

const A_SERVER_THAT_KEEPS_ANSWERING: &str = r#"import "std/io"
import "std/net/http"

fn handled _ n
  stepped n (n > 40)

fn stepped n true
  http/reply (http/ok "bye") (http/stop n)

fn stepped n false
  http/reply (http/ok "ok") (n + 1)

fn asked at
  more at (at > 41)

fn more _ true
  io/write_err "asked\n"

fn more at false
  where = "http://127.0.0.1:PORT/"
  wanted = ["-s" "-o" "/dev/null" "--retry-connrefused" where]
  io/run "curl" wanted . (_ -> asked (at + 1))

http/serve_until PORT handled 1 . (n -> print "answered {n}")
asked 1
"#;

fn free_port() -> u16 {
    let taken = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("the os hands out a port");
    let port = taken.local_addr().expect("a bound listener has an address").port();
    drop(taken);
    port
}

/// Watched red on the runtime before the guard asked about a slot:
/// "error[runtime]: too many processes at once", with nothing open.
#[test]
fn a_program_may_open_more_handles_than_the_table_holds() {
    let dir = std::env::temp_dir().join("kanso-many-handles");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    let source = A_SERVER_THAT_KEEPS_ANSWERING.replace("PORT", &free_port().to_string());
    std::fs::write(dir.join("serve.kso"), source).expect("the program writes");

    let output = Command::new(env!("CARGO_BIN_EXE_kanso"))
        .arg("play")
        .arg("serve.kso")
        .current_dir(&dir)
        .output()
        .expect("kanso binary runs");

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "answered 41\n",
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
