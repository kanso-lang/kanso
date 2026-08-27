//! Six socket failures, and the two engines that can reach them.
//!
//! A program hands `net/port`, `net/read`, `net/write`, `net/close_*`,
//! `net/accept` or `os/kill` a value that is not the handle it wants. Native
//! and the interpreter both speak here — neither refuses — so the differential
//! law requires the same sentence from both, and until this spec they gave
//! seven different ones:
//!
//!     net/port            native "that is not an open socket"  interp "that is not an open listener"
//!     net/accept          native "nothing connected"           interp "7 is not a listener"
//!     net/read            native "that is not a connection"    interp "7 is not a connection"
//!     net/write           native "that is not a connection"    interp "7 is not a connection"
//!     net/close_listener  native "that is not an open socket"  interp "7 is not an open socket"
//!     net/close_conn      native "that is not an open socket"  interp "7 is not an open socket"
//!     os/kill             native "that is not a running process"  interp "999 is not a running process"
//!
//! `net/accept` was the bad one. Native's arm never looked at the handle:
//! "nothing connected" is true of a listener nobody has dialled, and it was
//! said for a value that is not a listener at all.
//!
//! WHY THIS IS A RUST TEST AND NOT A CORPUS FIXTURE. The runtime corpus is
//! walked by a third engine — the in-process interpreter in tests/oracle.rs —
//! whose executor has no sockets and refuses with "this engine has no
//! sockets". That refusal is correct under the law, and it is a different
//! sentence, so a shared .stderr golden cannot hold both. The corpus asserts
//! one text for every engine that walks it; this pair of engines is asserted
//! here instead. tests/a_file_that_is_not_text.rs is here for the same reason.
//!
//! WHAT IS PINNED. That native and the interpreter say the same thing, and
//! what that thing is. NOT that the third engine must join them: whether the
//! in-process interpreter should grow sockets is a separate question, and the
//! refusal it gives today is a sanctioned answer rather than a gap.
use std::process::Command;

/// A record shaped like a listener or a connection, holding an int where the
/// handle goes. `opaque` is what gets it past the type checker: the checker
/// refuses the literal, and the runtime check is the backstop for a value
/// whose type it cannot narrow.
fn program(call: &str, import: &str) -> String {
    format!(
        "import \"std/{import}\"\n\n\
         pub type stand_in\n  handle\n\n\
         pub fn opaque x\n  x\n\n\
         pub play = {call} . (r -> print \"{{r}}\")\n"
    )
}

fn both_engines(call: &str, import: &str) -> (String, String) {
    let dir = std::env::temp_dir()
        .join(format!("kanso-sockets-{}", call.replace(['/', ' ', '(', ')', '"'], "_")));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    let name = "probe";
    std::fs::write(dir.join(format!("{name}.kso")), program(call, import))
        .expect("the program writes");
    std::fs::write(dir.join("run.kso"), format!("import \"./{name}\"\n\n{name}/play\n"))
        .expect("the entry writes");

    let run = |interp: bool| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kanso"));
        cmd.arg("run").arg("run.kso").current_dir(&dir);
        if interp {
            cmd.arg("--interp");
        }
        let done = cmd.output().expect("kanso runs");
        String::from_utf8_lossy(&done.stderr).into_owned()
    };
    let said = (run(false), run(true));
    let _ = std::fs::remove_dir_all(&dir);
    said
}

fn agreed(call: &str, import: &str, message: &str) {
    let want = format!("error[endpoint]: unhandled err reached the executor: {message:?}\n");
    let (native, interp) = both_engines(call, import);
    assert_eq!(native, want, "native said something else for `{call}`");
    assert_eq!(interp, want, "the interpreter said something else for `{call}`");
}

#[test]
fn a_port_read_from_a_socket_that_is_not_open() {
    // `socket` rather than `listener`, which the interpreter used to say. Both
    // words are accurate — a listening socket is a socket — and `socket` is
    // already what close answers for either kind. Native keeping its word is
    // also what keeps the two arms sharing one string: clang folds their
    // returns into one block, and a distinct wording costs 128 bytes of .text
    // in every compiled binary. Measured; see bench/text_golden.txt.
    agreed("net/port (opaque (stand_in 7))", "net", "that is not an open socket");
}

#[test]
fn an_accept_on_something_that_is_not_a_listener() {
    agreed("net/accept (opaque (stand_in 7))", "net", "that is not a listener");
}

#[test]
fn a_read_from_something_that_is_not_a_connection() {
    agreed("net/read (opaque (stand_in 7))", "net", "that is not a connection");
}

#[test]
fn a_write_to_something_that_is_not_a_connection() {
    agreed("net/write (opaque (stand_in 7)) \"hi\"", "net", "that is not a connection");
}

#[test]
fn a_close_of_a_socket_that_is_not_open() {
    agreed("net/close_conn (opaque (stand_in 7))", "net", "that is not an open socket");
}

#[test]
fn a_kill_of_a_process_that_is_not_running() {
    // No stand_in here: `os/kill` takes the handle itself, and its wrapper
    // reaches the builtin without reading a field first.
    agreed("os/kill (opaque 999)", "os", "that is not a running process");
}
